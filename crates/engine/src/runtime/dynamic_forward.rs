use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use fast_socks5::server::{Socks5ServerProtocol, SocksServerError, states};
use fast_socks5::util::target_addr::TargetAddr;
use fast_socks5::{ReplyError, Socks5Command};
use thiserror::Error;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tracing::debug;

use crate::error::SessionError;
use crate::runtime::counting_stream::CountingStream;
use crate::runtime::local_forward::relay;
use crate::runtime::log_buffer::LogLevel;
use crate::runtime::shared::TunnelShared;
use crate::session::SshSession;

const SOCKS_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
const UNSPECIFIED_BOUND_ADDRESS: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);

/// Everything a task serving one SOCKS5 client needs.
pub(crate) struct SocksJob {
    pub(crate) client: TcpStream,
    pub(crate) peer: SocketAddr,
    pub(crate) session: Arc<SshSession>,
    pub(crate) shared: Arc<TunnelShared>,
}

#[derive(Debug, Error)]
enum RequestFailure {
    #[error("{0}")]
    Invalid(SocksServerError),
    #[error("no SOCKS request within {0:?}")]
    TimedOut(Duration),
}

struct SocksRequest {
    protocol: Socks5ServerProtocol<TcpStream, states::CommandRead>,
    command: Socks5Command,
    target: TargetAddr,
}

/// Serves one SOCKS5 client: reads its request within a bound, opens a
/// `direct-tcpip` channel to the target it names and relays.
pub(crate) async fn socks_connection(job: SocksJob) {
    let SocksJob {
        client,
        peer,
        session,
        shared,
    } = job;
    let _active = shared.counters().begin_connection();
    let request = match read_request_within(client, SOCKS_HANDSHAKE_TIMEOUT).await {
        Ok(request) => request,
        Err(failure) => {
            return shared.log(
                LogLevel::Warn,
                format!("connection from {peer} closed: {failure}"),
            );
        }
    };
    serve_request(request, peer, &session, &shared).await;
}

async fn read_request_within(
    client: TcpStream,
    limit: Duration,
) -> Result<SocksRequest, RequestFailure> {
    timeout(limit, read_request(client))
        .await
        .map_err(|_| RequestFailure::TimedOut(limit))?
        .map_err(RequestFailure::Invalid)
}

async fn read_request(client: TcpStream) -> Result<SocksRequest, SocksServerError> {
    let (protocol, command, target) = Socks5ServerProtocol::accept_no_auth(client)
        .await?
        .read_command()
        .await?;
    Ok(SocksRequest {
        protocol,
        command,
        target,
    })
}

async fn serve_request(
    request: SocksRequest,
    peer: SocketAddr,
    session: &SshSession,
    shared: &TunnelShared,
) {
    let SocksRequest {
        protocol,
        command,
        target,
    } = request;
    if let Some(rejection) = rejection_for(&command) {
        shared.log(
            LogLevel::Warn,
            format!("connection from {peer} closed: SOCKS command {command:?} is not supported"),
        );
        return reply_error(protocol, rejection).await;
    }
    let (host, port) = destination_of(&target);
    match session.open_direct_tcpip(&host, port, Some(peer)).await {
        Ok(channel) => connect(protocol, channel, shared).await,
        Err(error) => {
            shared.log(
                LogLevel::Warn,
                format!("connection from {peer} closed: cannot reach {host}:{port}: {error}"),
            );
            reply_error(protocol, reply_error_for(&error)).await;
        }
    }
}

async fn connect<Channel>(
    protocol: Socks5ServerProtocol<TcpStream, states::CommandRead>,
    channel: Channel,
    shared: &TunnelShared,
) where
    Channel: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    match protocol.reply_success(UNSPECIFIED_BOUND_ADDRESS).await {
        Ok(client) => {
            let counted_client =
                CountingStream::on_client_side(client, Arc::clone(shared.counters()));
            relay(counted_client, channel).await;
        }
        Err(error) => debug!(%error, "cannot tell the SOCKS client that the connection is open"),
    }
}

async fn reply_error(
    protocol: Socks5ServerProtocol<TcpStream, states::CommandRead>,
    error: ReplyError,
) {
    if let Err(failure) = protocol.reply_error(&error).await {
        debug!(%failure, "cannot send the SOCKS error to the client");
    }
}

/// The reply that refuses a SOCKS command; `None` for the only command
/// served, `CONNECT`.
fn rejection_for(command: &Socks5Command) -> Option<ReplyError> {
    match command {
        Socks5Command::TCPConnect => None,
        Socks5Command::TCPBind | Socks5Command::UDPAssociate => {
            Some(ReplyError::CommandNotSupported)
        }
    }
}

/// The host and port to open the `direct-tcpip` channel to. A domain name
/// stays a name: the server resolves it.
fn destination_of(target: &TargetAddr) -> (String, u16) {
    match target {
        TargetAddr::Ip(address) => (address.ip().to_string(), address.port()),
        TargetAddr::Domain(name, port) => (name.clone(), *port),
    }
}

/// The SOCKS error that tells the client why the channel did not open.
fn reply_error_for(error: &SessionError) -> ReplyError {
    let SessionError::OpenChannel { source, .. } = error else {
        return ReplyError::GeneralFailure;
    };
    match **source {
        russh::Error::ChannelOpenFailure(russh::ChannelOpenFailure::ConnectFailed) => {
            ReplyError::ConnectionRefused
        }
        russh::Error::ChannelOpenFailure(russh::ChannelOpenFailure::AdministrativelyProhibited) => {
            ReplyError::ConnectionNotAllowed
        }
        _ => ReplyError::GeneralFailure,
    }
}

#[cfg(test)]
mod tests {
    use tokio::io::AsyncWriteExt;
    use tokio::net::TcpListener;

    use super::*;

    const SHORT_LIMIT: Duration = Duration::from_millis(50);

    async fn connected_pair() -> (TcpStream, TcpStream) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let client = TcpStream::connect(listener.local_addr().unwrap())
            .await
            .unwrap();
        let (server, _) = listener.accept().await.unwrap();
        (client, server)
    }

    fn open_failure(source: russh::Error) -> SessionError {
        SessionError::OpenChannel {
            host: "db".to_owned(),
            port: 5432,
            source: Box::new(source),
        }
    }

    #[tokio::test]
    async fn should_time_out_when_the_client_sends_nothing() {
        let (_client, server) = connected_pair().await;

        let failure = read_request_within(server, SHORT_LIMIT).await.err();

        assert!(matches!(failure, Some(RequestFailure::TimedOut(_))));
    }

    #[tokio::test]
    async fn should_reject_the_request_when_the_client_does_not_speak_socks5() {
        let (mut client, server) = connected_pair().await;
        client.write_all(b"GET / HTTP/1.1\r\n\r\n").await.unwrap();

        let failure = read_request_within(server, SHORT_LIMIT).await.err();

        assert!(matches!(failure, Some(RequestFailure::Invalid(_))));
    }

    #[tokio::test]
    async fn should_read_the_target_when_the_client_sends_a_connect_request() {
        let (mut client, server) = connected_pair().await;
        client
            .write_all(&[5, 1, 0, 5, 1, 0, 1, 198, 51, 100, 70, 0x15, 0x38])
            .await
            .unwrap();

        let request = read_request_within(server, SHORT_LIMIT).await.unwrap();

        assert_eq!(
            destination_of(&request.target),
            ("198.51.100.70".to_owned(), 5432)
        );
    }

    #[test]
    fn should_serve_the_command_when_it_is_connect() {
        assert!(rejection_for(&Socks5Command::TCPConnect).is_none());
    }

    #[test]
    fn should_refuse_the_command_when_it_is_bind() {
        let rejection = rejection_for(&Socks5Command::TCPBind).unwrap();

        assert_eq!(rejection.as_u8(), ReplyError::CommandNotSupported.as_u8());
    }

    #[test]
    fn should_refuse_the_command_when_it_is_udp_associate() {
        let rejection = rejection_for(&Socks5Command::UDPAssociate).unwrap();

        assert_eq!(rejection.as_u8(), ReplyError::CommandNotSupported.as_u8());
    }

    #[test]
    fn should_use_the_textual_address_when_the_target_is_an_ip() {
        let target = TargetAddr::Ip(SocketAddr::from(([198, 51, 100, 70], 5432)));

        assert_eq!(destination_of(&target), ("198.51.100.70".to_owned(), 5432));
    }

    #[test]
    fn should_keep_the_name_unresolved_when_the_target_is_a_domain() {
        let target = TargetAddr::Domain("db.internal".to_owned(), 5432);

        assert_eq!(destination_of(&target), ("db.internal".to_owned(), 5432));
    }

    #[test]
    fn should_reply_connection_refused_when_the_server_cannot_connect() {
        let error = open_failure(russh::Error::ChannelOpenFailure(
            russh::ChannelOpenFailure::ConnectFailed,
        ));

        assert_eq!(
            reply_error_for(&error).as_u8(),
            ReplyError::ConnectionRefused.as_u8()
        );
    }

    #[test]
    fn should_reply_not_allowed_when_the_server_prohibits_the_channel() {
        let error = open_failure(russh::Error::ChannelOpenFailure(
            russh::ChannelOpenFailure::AdministrativelyProhibited,
        ));

        assert_eq!(
            reply_error_for(&error).as_u8(),
            ReplyError::ConnectionNotAllowed.as_u8()
        );
    }

    #[test]
    fn should_reply_general_failure_when_the_session_broke() {
        let error = open_failure(russh::Error::Disconnect);

        assert_eq!(
            reply_error_for(&error).as_u8(),
            ReplyError::GeneralFailure.as_u8()
        );
    }
}
