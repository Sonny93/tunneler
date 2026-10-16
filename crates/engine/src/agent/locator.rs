use std::path::{Path, PathBuf};

use ssh_config::AgentSocket;

const AUTH_SOCKET_VARIABLE: &str = "SSH_AUTH_SOCK";
const WINDOWS_PIPE_PREFIX: &str = r"\\.\pipe\";
const OPENSSH_WINDOWS_PIPE: &str = r"\\.\pipe\openssh-ssh-agent";

/// Where an SSH agent can be reached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentEndpoint {
    /// A Unix domain socket (Unix only).
    UnixSocket(PathBuf),
    /// A Windows named pipe, such as the OpenSSH agent's (Windows only).
    NamedPipe(PathBuf),
    /// The `PuTTY` agent (Windows only).
    Pageant,
}

/// Where the engine learns about the agent of the machine.
///
/// The engine never reads the process environment itself, so tests provide
/// their own locator.
pub trait AgentLocator: Send + Sync {
    /// The endpoints to try, in order. `socket` is `IdentityAgent` when it is
    /// set; `None` means the default location of the platform.
    fn locate(&self, socket: Option<&AgentSocket>) -> Vec<AgentEndpoint>;
}

/// The agent of the current user, found through the process environment
/// (`SSH_AUTH_SOCK` on Unix, the OpenSSH pipe then Pageant on Windows).
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemAgentLocator;

impl AgentLocator for SystemAgentLocator {
    fn locate(&self, socket: Option<&AgentSocket>) -> Vec<AgentEndpoint> {
        locate_endpoints(socket, &|name| std::env::var(name).ok())
    }
}

fn locate_endpoints(
    socket: Option<&AgentSocket>,
    variable: &dyn Fn(&str) -> Option<String>,
) -> Vec<AgentEndpoint> {
    match socket {
        Some(AgentSocket::Path(path)) => vec![endpoint_for_path(path)],
        Some(AgentSocket::Environment(name)) => variable(name)
            .map(|value| endpoint_for_path(Path::new(&value)))
            .into_iter()
            .collect(),
        None => default_endpoints(variable),
    }
}

fn default_endpoints(variable: &dyn Fn(&str) -> Option<String>) -> Vec<AgentEndpoint> {
    if cfg!(windows) {
        return vec![
            AgentEndpoint::NamedPipe(PathBuf::from(OPENSSH_WINDOWS_PIPE)),
            AgentEndpoint::Pageant,
        ];
    }
    variable(AUTH_SOCKET_VARIABLE)
        .map(|value| AgentEndpoint::UnixSocket(PathBuf::from(value)))
        .into_iter()
        .collect()
}

fn endpoint_for_path(path: &Path) -> AgentEndpoint {
    if path.to_string_lossy().starts_with(WINDOWS_PIPE_PREFIX) {
        return AgentEndpoint::NamedPipe(path.to_owned());
    }
    AgentEndpoint::UnixSocket(path.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_variables(_name: &str) -> Option<String> {
        None
    }

    fn auth_socket_only(name: &str) -> Option<String> {
        (name == AUTH_SOCKET_VARIABLE).then(|| "/run/agent.sock".to_owned())
    }

    #[test]
    fn should_use_the_configured_path_when_identity_agent_is_a_path() {
        let socket = AgentSocket::Path(PathBuf::from("/tmp/agent"));
        assert_eq!(
            locate_endpoints(Some(&socket), &no_variables),
            vec![AgentEndpoint::UnixSocket(PathBuf::from("/tmp/agent"))]
        );
    }

    #[test]
    fn should_read_the_variable_when_identity_agent_names_one() {
        let socket = AgentSocket::Environment("SSH_AUTH_SOCK".to_owned());
        assert_eq!(
            locate_endpoints(Some(&socket), &auth_socket_only),
            vec![AgentEndpoint::UnixSocket(PathBuf::from("/run/agent.sock"))]
        );
    }

    #[test]
    fn should_find_nothing_when_the_named_variable_is_unset() {
        let socket = AgentSocket::Environment("SSH_AUTH_SOCK".to_owned());
        assert_eq!(locate_endpoints(Some(&socket), &no_variables), Vec::new());
    }

    #[test]
    fn should_treat_a_pipe_path_as_a_named_pipe_when_identity_agent_is_a_pipe() {
        let socket = AgentSocket::Path(PathBuf::from(OPENSSH_WINDOWS_PIPE));
        assert_eq!(
            locate_endpoints(Some(&socket), &no_variables),
            vec![AgentEndpoint::NamedPipe(PathBuf::from(
                OPENSSH_WINDOWS_PIPE
            ))]
        );
    }

    #[cfg(unix)]
    #[test]
    fn should_use_the_auth_socket_variable_when_identity_agent_is_unset() {
        assert_eq!(
            locate_endpoints(None, &auth_socket_only),
            vec![AgentEndpoint::UnixSocket(PathBuf::from("/run/agent.sock"))]
        );
    }

    #[cfg(unix)]
    #[test]
    fn should_find_nothing_when_identity_agent_is_unset_and_the_variable_too() {
        assert_eq!(locate_endpoints(None, &no_variables), Vec::new());
    }
}
