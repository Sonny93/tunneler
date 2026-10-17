use crate::error::{ConnectError, SessionError};

/// Whether trying to connect again can succeed.
///
/// Network trouble is retried. Everything that needs the user or a change of
/// configuration is fatal, so that a wrong password or an unknown host key
/// never turns into a storm of prompts.
pub(crate) fn is_retryable(error: &ConnectError) -> bool {
    match error {
        ConnectError::Transport(_)
        | ConnectError::HandshakeTimeout { .. }
        | ConnectError::Tunnel(_) => true,
        ConnectError::Handshake(source) | ConnectError::Protocol(source) => {
            is_connection_loss(source)
        }
        ConnectError::Jump { source, .. } => is_retryable(source),
        ConnectError::Configuration(_)
        | ConnectError::KnownHosts(_)
        | ConnectError::HostKey(_)
        | ConnectError::Authentication { .. }
        | ConnectError::JumpChain(_)
        | ConnectError::Prompt(_)
        | ConnectError::Agent(_) => false,
    }
}

fn is_connection_loss(error: &russh::Error) -> bool {
    matches!(
        error,
        russh::Error::IO(_)
            | russh::Error::Disconnect
            | russh::Error::HUP
            | russh::Error::ConnectionTimeout
            | russh::Error::KeepaliveTimeout
            | russh::Error::InactivityTimeout
            | russh::Error::SendError
            | russh::Error::Elapsed(_)
    )
}

/// Whether the server reported that it could not reach the target, as
/// opposed to a problem with the session itself.
pub(crate) fn is_target_unreachable(error: &SessionError) -> bool {
    match error {
        SessionError::OpenChannel { source, .. } => matches!(
            **source,
            russh::Error::ChannelOpenFailure(russh::ChannelOpenFailure::ConnectFailed)
        ),
        SessionError::ForwardRefused { .. }
        | SessionError::RequestForward { .. }
        | SessionError::CancelForward { .. }
        | SessionError::Disconnect(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::net::SocketAddr;
    use std::path::PathBuf;
    use std::time::Duration;

    use crate::error::{HostKeyFailure, JumpChainError, TransportError};
    use crate::host_config::HostConfigError;
    use crate::prompter::PromptError;

    use super::*;

    fn io_failure() -> io::Error {
        io::Error::from(io::ErrorKind::ConnectionRefused)
    }

    fn address() -> SocketAddr {
        SocketAddr::from(([127, 0, 0, 1], 22))
    }

    fn jump(source: ConnectError) -> ConnectError {
        ConnectError::Jump {
            hop: "bastion".to_owned(),
            source: Box::new(source),
        }
    }

    fn rejected_by_user() -> ConnectError {
        ConnectError::HostKey(HostKeyFailure::RejectedByUser {
            presented_fingerprint: "SHA256:abc".to_owned(),
        })
    }

    fn open_channel_failure(source: russh::Error) -> SessionError {
        SessionError::OpenChannel {
            host: "db".to_owned(),
            port: 5432,
            source: Box::new(source),
        }
    }

    #[test]
    fn should_retry_when_the_tcp_connection_is_refused() {
        let error = ConnectError::Transport(TransportError::Connect {
            address: address(),
            source: io_failure(),
        });

        assert!(is_retryable(&error));
    }

    #[test]
    fn should_retry_when_the_host_name_does_not_resolve() {
        let error = ConnectError::Transport(TransportError::Resolve {
            host: "nowhere".to_owned(),
            port: 22,
            source: io_failure(),
        });

        assert!(is_retryable(&error));
    }

    #[test]
    fn should_retry_when_the_transport_times_out() {
        let error = ConnectError::Transport(TransportError::Timeout {
            address: address(),
            timeout: Duration::from_secs(1),
        });

        assert!(is_retryable(&error));
    }

    #[test]
    fn should_retry_when_the_handshake_times_out() {
        let error = ConnectError::HandshakeTimeout {
            timeout: Duration::from_secs(1),
        };

        assert!(is_retryable(&error));
    }

    #[test]
    fn should_retry_when_the_handshake_fails_on_an_io_error() {
        assert!(is_retryable(&ConnectError::Handshake(russh::Error::IO(
            io_failure()
        ))));
    }

    #[test]
    fn should_retry_when_the_server_hangs_up_during_the_handshake() {
        assert!(is_retryable(&ConnectError::Handshake(russh::Error::HUP)));
    }

    #[test]
    fn should_not_retry_when_the_handshake_finds_no_common_algorithm() {
        let error = ConnectError::Handshake(russh::Error::UnknownAlgo);

        assert!(!is_retryable(&error));
    }

    #[test]
    fn should_retry_when_the_connection_breaks_during_authentication() {
        assert!(is_retryable(&ConnectError::Protocol(
            russh::Error::Disconnect
        )));
    }

    #[test]
    fn should_not_retry_when_the_protocol_is_violated_during_authentication() {
        assert!(!is_retryable(&ConnectError::Protocol(
            russh::Error::Inconsistent
        )));
    }

    #[test]
    fn should_retry_when_the_channel_to_the_next_host_cannot_be_opened() {
        let error = ConnectError::Tunnel(open_channel_failure(russh::Error::Disconnect));

        assert!(is_retryable(&error));
    }

    #[test]
    fn should_retry_when_a_jump_host_is_unreachable() {
        let error = jump(ConnectError::Transport(TransportError::Connect {
            address: address(),
            source: io_failure(),
        }));

        assert!(is_retryable(&error));
    }

    #[test]
    fn should_not_retry_when_a_jump_host_refuses_the_credentials() {
        let error = jump(ConnectError::Authentication { tried: Vec::new() });

        assert!(!is_retryable(&error));
    }

    #[test]
    fn should_not_retry_when_a_jump_host_key_is_refused() {
        assert!(!is_retryable(&jump(rejected_by_user())));
    }

    #[test]
    fn should_not_retry_when_the_host_key_is_refused() {
        assert!(!is_retryable(&rejected_by_user()));
    }

    #[test]
    fn should_not_retry_when_the_host_key_changed() {
        let error = ConnectError::HostKey(HostKeyFailure::Changed {
            presented_fingerprint: "SHA256:new".to_owned(),
            known_fingerprint: "SHA256:old".to_owned(),
            path: PathBuf::from("known_hosts"),
            line: 1,
        });

        assert!(!is_retryable(&error));
    }

    #[test]
    fn should_not_retry_when_authentication_fails() {
        assert!(!is_retryable(&ConnectError::Authentication {
            tried: Vec::new()
        }));
    }

    #[test]
    fn should_not_retry_when_the_alias_cannot_be_resolved() {
        let error = ConnectError::Configuration(HostConfigError::Resolution {
            alias: "web".to_owned(),
            source: Box::new(ssh_config::Error::Io {
                path: PathBuf::from("config"),
                source: io_failure(),
            }),
        });

        assert!(!is_retryable(&error));
    }

    #[test]
    fn should_not_retry_when_the_known_hosts_cannot_be_read() {
        let error = ConnectError::KnownHosts(known_hosts::Error::Io {
            path: PathBuf::from("known_hosts"),
            source: io_failure(),
        });

        assert!(!is_retryable(&error));
    }

    #[test]
    fn should_not_retry_when_the_jump_chain_is_a_cycle() {
        let error = ConnectError::JumpChain(JumpChainError::Cycle { chain: Vec::new() });

        assert!(!is_retryable(&error));
    }

    #[test]
    fn should_not_retry_when_a_prompt_cannot_be_asked() {
        let error = ConnectError::Prompt(PromptError::Unavailable {
            reason: "no UI".to_owned(),
        });

        assert!(!is_retryable(&error));
    }

    #[test]
    fn should_not_retry_when_the_agent_fails() {
        let error = ConnectError::Agent(Box::new(russh::keys::Error::KeyIsEncrypted));

        assert!(!is_retryable(&error));
    }

    #[test]
    fn should_report_the_target_unreachable_when_the_server_says_connect_failed() {
        let error = open_channel_failure(russh::Error::ChannelOpenFailure(
            russh::ChannelOpenFailure::ConnectFailed,
        ));

        assert!(is_target_unreachable(&error));
    }

    #[test]
    fn should_not_report_the_target_unreachable_when_the_server_prohibits_the_forward() {
        let error = open_channel_failure(russh::Error::ChannelOpenFailure(
            russh::ChannelOpenFailure::AdministrativelyProhibited,
        ));

        assert!(!is_target_unreachable(&error));
    }

    #[test]
    fn should_not_report_the_target_unreachable_when_the_session_is_gone() {
        assert!(!is_target_unreachable(&open_channel_failure(
            russh::Error::SendError
        )));
    }

    #[test]
    fn should_not_report_the_target_unreachable_when_the_disconnect_fails() {
        assert!(!is_target_unreachable(&SessionError::Disconnect(
            russh::Error::SendError
        )));
    }
}
