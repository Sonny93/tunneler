use std::path::Path;
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio::time::timeout;

use super::display::{DisplayEndpoint, LocalDisplay};
use super::error::X11Error;

const LOCAL_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// A connection to the local X server.
pub(crate) trait DisplayStream: AsyncRead + AsyncWrite + Unpin + Send {}

impl<Stream> DisplayStream for Stream where Stream: AsyncRead + AsyncWrite + Unpin + Send {}

pub(crate) type BoxedDisplayStream = Box<dyn DisplayStream>;

/// Connects to the local X server.
pub(crate) async fn connect_display(
    display: &LocalDisplay,
) -> Result<BoxedDisplayStream, X11Error> {
    match timeout(LOCAL_CONNECT_TIMEOUT, connect_endpoint(display)).await {
        Ok(connected) => connected,
        Err(_) => Err(X11Error::DisplayTimeout {
            display: display.to_string(),
            timeout: LOCAL_CONNECT_TIMEOUT,
        }),
    }
}

async fn connect_endpoint(display: &LocalDisplay) -> Result<BoxedDisplayStream, X11Error> {
    match &display.endpoint {
        DisplayEndpoint::Tcp { host, port } => {
            let stream = TcpStream::connect((host.as_str(), *port))
                .await
                .map_err(|source| X11Error::ConnectDisplay {
                    display: display.to_string(),
                    source,
                })?;
            Ok(Box::new(stream))
        }
        DisplayEndpoint::UnixSocket(path) => connect_unix_socket(display, path).await,
    }
}

#[cfg(unix)]
async fn connect_unix_socket(
    display: &LocalDisplay,
    path: &Path,
) -> Result<BoxedDisplayStream, X11Error> {
    let stream = tokio::net::UnixStream::connect(path)
        .await
        .map_err(|source| X11Error::ConnectDisplay {
            display: display.to_string(),
            source,
        })?;
    Ok(Box::new(stream))
}

#[cfg(not(unix))]
#[allow(
    clippy::unused_async,
    reason = "matches the signature of the Unix implementation"
)]
async fn connect_unix_socket(
    display: &LocalDisplay,
    _path: &Path,
) -> Result<BoxedDisplayStream, X11Error> {
    Err(X11Error::UnixSocketUnsupported {
        display: display.to_string(),
    })
}

#[cfg(all(test, unix))]
mod tests {
    use std::path::PathBuf;

    use tokio::net::{TcpListener, UnixListener};

    use super::*;

    fn tcp_display(port: u16) -> LocalDisplay {
        LocalDisplay {
            number: 0,
            endpoint: DisplayEndpoint::Tcp {
                host: "127.0.0.1".to_owned(),
                port,
            },
        }
    }

    #[tokio::test]
    async fn should_connect_when_the_tcp_display_listens() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        assert!(connect_display(&tcp_display(port)).await.is_ok());
    }

    #[tokio::test]
    async fn should_fail_when_nothing_listens_on_the_tcp_display() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        assert!(matches!(
            connect_display(&tcp_display(port)).await,
            Err(X11Error::ConnectDisplay { .. })
        ));
    }

    #[tokio::test]
    async fn should_connect_when_the_unix_socket_listens() {
        let directory = tempfile::tempdir().unwrap();
        let path: PathBuf = directory.path().join("X0");
        let _listener = UnixListener::bind(&path).unwrap();
        let display = LocalDisplay {
            number: 0,
            endpoint: DisplayEndpoint::UnixSocket(path),
        };

        assert!(connect_display(&display).await.is_ok());
    }
}
