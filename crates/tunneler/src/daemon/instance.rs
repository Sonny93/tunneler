use ipc::{BindError, Endpoint, IpcServer};

use super::error::DaemonError;

/// Binds the endpoint, which is what makes this process the only daemon.
///
/// When the endpoint is taken, a live daemon is reported as
/// [`DaemonError::AlreadyRunning`]; a socket left by a crashed daemon is
/// removed and the bind is attempted once more.
pub(crate) async fn bind_single_instance(endpoint: &Endpoint) -> Result<IpcServer, DaemonError> {
    match IpcServer::bind(endpoint) {
        Ok(server) => Ok(server),
        Err(BindError::AddressInUse) => reclaim_and_bind(endpoint).await,
        Err(error) => Err(error.into()),
    }
}

async fn reclaim_and_bind(endpoint: &Endpoint) -> Result<IpcServer, DaemonError> {
    if endpoint.is_daemon_alive().await {
        return Err(DaemonError::AlreadyRunning);
    }
    endpoint
        .reclaim_stale()
        .map_err(DaemonError::ReclaimStale)?;
    IpcServer::bind(endpoint).map_err(|error| match error {
        BindError::AddressInUse => DaemonError::EndpointInUse,
        other => other.into(),
    })
}

#[cfg(test)]
#[cfg(unix)]
mod tests {
    use std::os::unix::net::UnixListener;

    use super::*;
    use crate::daemon::testing::{TestServer, private_tempdir};

    #[tokio::test]
    async fn should_bind_when_the_endpoint_is_free() {
        let directory = private_tempdir();
        let endpoint = Endpoint::from_path(directory.path().join("tunneler.sock"));

        assert!(bind_single_instance(&endpoint).await.is_ok());
    }

    #[tokio::test]
    async fn should_report_already_running_when_a_daemon_answers() {
        let running = TestServer::start();

        let outcome = bind_single_instance(&running.endpoint).await;

        assert!(matches!(outcome, Err(DaemonError::AlreadyRunning)));
    }

    #[tokio::test]
    async fn should_reclaim_the_socket_when_a_crashed_daemon_left_it_behind() {
        let directory = private_tempdir();
        let path = directory.path().join("tunneler.sock");
        drop(UnixListener::bind(&path).unwrap());
        let endpoint = Endpoint::from_path(&path);

        assert!(bind_single_instance(&endpoint).await.is_ok());
    }

    #[tokio::test]
    async fn should_report_the_endpoint_in_use_when_a_regular_file_holds_the_path() {
        let directory = private_tempdir();
        let path = directory.path().join("tunneler.sock");
        std::fs::write(&path, b"not a socket").unwrap();
        let endpoint = Endpoint::from_path(&path);

        let outcome = bind_single_instance(&endpoint).await;

        assert!(matches!(outcome, Err(DaemonError::EndpointInUse)));
    }
}
