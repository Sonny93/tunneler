use std::sync::Arc;

use russh::Channel;
use russh::client::Msg;
use tracing::{info, warn};

use crate::runtime::session_manager::{AcquireError, SessionLease, SessionManager};
use crate::store::X11Launcher;
use crate::x11::{ChannelEvent, LocalX11Resolver, X11Error, next_channel_event, start_command};

/// Runs the saved commands of the hosts in sessions with X11 forwarding.
pub(crate) struct X11Launches {
    sessions: SessionManager,
    resolver: Arc<LocalX11Resolver>,
}

impl X11Launches {
    pub(crate) fn new(sessions: SessionManager, resolver: Arc<LocalX11Resolver>) -> Self {
        Self { sessions, resolver }
    }

    /// Runs the command of `launcher` and returns once the server accepted
    /// it. The session stays open until the command ends; its output goes to
    /// the log.
    pub(crate) async fn launch(&self, launcher: X11Launcher) -> Result<(), X11Error> {
        let lease = self
            .sessions
            .acquire(&launcher.host)
            .await
            .map_err(acquisition_failure)?;
        let session = lease.session();
        let local = self.resolver.resolve(&launcher.host).await?;
        let forwarding = session.enable_x11(local)?;
        let mut channel = session.open_x11_session(&forwarding).await?;
        if let Err(error) = start_command(&mut channel, &launcher.command).await {
            let _ = channel.close().await;
            return Err(error);
        }
        tokio::spawn(follow_command(lease, channel, launcher));
        Ok(())
    }
}

fn acquisition_failure(error: AcquireError) -> X11Error {
    match error {
        AcquireError::Connect(error) => X11Error::Connect(error),
        AcquireError::Aborted => X11Error::ShuttingDown,
    }
}

#[derive(Clone, Copy)]
enum OutputStream {
    Standard,
    Error,
}

async fn follow_command(lease: SessionLease, mut channel: Channel<Msg>, launcher: X11Launcher) {
    let mut exit_status = None;
    loop {
        let event = tokio::select! {
            () = lease.lost() => break,
            event = next_channel_event(&mut channel) => event,
        };
        match event {
            ChannelEvent::Stdout(chunk) => log_output(&launcher, &chunk, OutputStream::Standard),
            ChannelEvent::Stderr(chunk) => log_output(&launcher, &chunk, OutputStream::Error),
            ChannelEvent::ExitStatus(status) => exit_status = Some(status),
            ChannelEvent::Closed => break,
            ChannelEvent::Accepted | ChannelEvent::Refused => {}
        }
    }
    log_end(&launcher, exit_status);
}

fn log_output(launcher: &X11Launcher, chunk: &[u8], stream: OutputStream) {
    let host = &launcher.host;
    let name = &launcher.name;
    for line in String::from_utf8_lossy(chunk)
        .lines()
        .filter(|line| !line.is_empty())
    {
        match stream {
            OutputStream::Standard => info!(%host, launcher = %name, "{line}"),
            OutputStream::Error => warn!(%host, launcher = %name, "{line}"),
        }
    }
}

fn log_end(launcher: &X11Launcher, exit_status: Option<u32>) {
    let host = &launcher.host;
    let name = &launcher.name;
    if let Some(status) = exit_status {
        info!(%host, launcher = %name, "exited with status {status}");
    } else {
        info!(%host, launcher = %name, "ended without an exit status");
    }
}
