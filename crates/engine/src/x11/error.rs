use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use thiserror::Error;

use crate::error::ConnectError;

/// Why X11 forwarding could not be set up or served.
#[derive(Debug, Error)]
pub enum X11Error {
    /// No setting, no `$DISPLAY` and no platform default names a local display.
    #[error("no local X display: set DISPLAY or the x11_display setting")]
    NoLocalDisplay,
    /// The text is not a valid X display name.
    #[error("invalid X display \"{text}\": {reason}")]
    InvalidDisplay {
        /// The text that was parsed.
        text: String,
        /// What is wrong with it.
        reason: &'static str,
    },
    /// The local X server does not accept connections.
    #[error("cannot connect to the local X display {display}: {source}")]
    ConnectDisplay {
        /// The display that was tried.
        display: String,
        /// The socket failure.
        source: io::Error,
    },
    /// The local X server did not answer in time.
    #[error("the local X display {display} did not answer within {timeout:?}")]
    DisplayTimeout {
        /// The display that was tried.
        display: String,
        /// The bound that elapsed.
        timeout: Duration,
    },
    /// The display is a Unix socket and the platform has none.
    #[error("the local X display {display} is a Unix socket, which this platform lacks")]
    UnixSocketUnsupported {
        /// The display that was tried.
        display: String,
    },
    /// The `Xauthority` file exists but cannot be read.
    #[error("cannot read the Xauthority file {}: {source}", path.display())]
    ReadXauthority {
        /// The file.
        path: PathBuf,
        /// The file system failure.
        source: io::Error,
    },
    /// The system has no entropy for the session cookie.
    #[error("cannot generate the X11 cookie: {0}")]
    Entropy(getrandom::Error),
    /// The SSH session could not be opened.
    #[error("{0}")]
    Connect(Arc<ConnectError>),
    /// The session manager shut down.
    #[error("the engine is shutting down")]
    ShuttingDown,
    /// The server refused the X11 forwarding request.
    #[error("the server refused X11 forwarding")]
    Refused,
    /// The server refused to run the command.
    #[error("the server refused to run the command")]
    CommandRefused,
    /// The command printed an empty `DISPLAY`.
    #[error("the server did not set DISPLAY: is X11Forwarding enabled on the server?")]
    EmptyRemoteDisplay,
    /// The server printed no `DISPLAY` line in time.
    #[error("the server did not report DISPLAY within {0:?}")]
    RemoteDisplayTimeout(Duration),
    /// The server did not answer a request in time.
    #[error("the server did not answer within {0:?}")]
    RequestTimeout(Duration),
    /// The session channel could not be driven.
    #[error("the session channel failed: {0}")]
    Channel(Box<russh::Error>),
    /// The session channel closed before the expected answer.
    #[error("the session channel closed")]
    ChannelClosed,
}

impl X11Error {
    /// Whether the failure comes from losing the session rather than from
    /// the server or the configuration, so that trying again can help.
    pub(crate) fn is_session_loss(&self) -> bool {
        matches!(self, Self::Channel(_) | Self::ChannelClosed)
    }
}
