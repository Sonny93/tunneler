use interprocess::local_socket::ListenerOptions;
use interprocess::local_socket::tokio::Listener;

use crate::endpoint::Endpoint;
use crate::error::BindError;

pub(crate) fn create_listener(endpoint: &Endpoint) -> Result<Listener, BindError> {
    create_listener_with_restrictions(endpoint).map_err(|error| match error {
        BindError::Io(error) if error.kind() == std::io::ErrorKind::AddrInUse => {
            BindError::AddressInUse
        }
        other => other,
    })
}

#[cfg(unix)]
mod restrictions {
    use std::io;
    use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
    use std::path::Path;

    use interprocess::local_socket::tokio::{Listener, Stream};
    use interprocess::local_socket::traits::StreamCommon as _;
    use interprocess::os::unix::local_socket::ListenerOptionsExt;

    use super::create_options;
    use crate::endpoint::Endpoint;
    use crate::error::BindError;
    use crate::server::handler::PeerInfo;

    const OWNER_ONLY_FILE_MODE: u32 = 0o600;
    /// The same mode as [`OWNER_ONLY_FILE_MODE`], typed as the platform's `mode_t`.
    const OWNER_ONLY_SOCKET_MODE: rustix::fs::RawMode = 0o600;
    const OWNER_ONLY_DIRECTORY_MODE: u32 = 0o700;
    const GROUP_AND_OTHER_BITS: u32 = 0o077;

    pub(super) fn create_listener_with_restrictions(
        endpoint: &Endpoint,
    ) -> Result<Listener, BindError> {
        if endpoint.requires_private_directory() {
            ensure_private_directory(parent_of(endpoint)?)?;
        }
        let options = create_options(endpoint)?;
        match options.mode(OWNER_ONLY_SOCKET_MODE).create_tokio() {
            Err(error) if error.kind() == io::ErrorKind::Unsupported => {
                bind_then_restrict(endpoint)
            }
            other => other.map_err(BindError::from),
        }
    }

    fn bind_then_restrict(endpoint: &Endpoint) -> Result<Listener, BindError> {
        let directory = parent_of(endpoint)?;
        verify_private_directory(directory)?;
        let listener = create_options(endpoint)?.create_tokio()?;
        if let Some(path) = endpoint.socket_path() {
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(OWNER_ONLY_FILE_MODE))?;
        }
        Ok(listener)
    }

    fn parent_of(endpoint: &Endpoint) -> Result<&Path, BindError> {
        endpoint
            .socket_path()
            .and_then(Path::parent)
            .ok_or_else(|| {
                BindError::Io(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "the socket path has no parent directory",
                ))
            })
    }

    fn ensure_private_directory(directory: &Path) -> Result<(), BindError> {
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(OWNER_ONLY_DIRECTORY_MODE)
            .create(directory)?;
        verify_private_directory(directory)
    }

    fn verify_private_directory(directory: &Path) -> Result<(), BindError> {
        let metadata = std::fs::symlink_metadata(directory)?;
        let is_private = metadata.is_dir()
            && metadata.uid() == rustix::process::geteuid().as_raw()
            && metadata.mode() & GROUP_AND_OTHER_BITS == 0;
        if is_private {
            return Ok(());
        }
        Err(BindError::InsecureDirectory {
            path: directory.to_path_buf(),
        })
    }

    pub(crate) fn inspect_peer(stream: &Stream) -> Option<PeerInfo> {
        let credentials = match stream.peer_creds() {
            Ok(credentials) => credentials,
            Err(error) => {
                tracing::warn!(%error, "cannot read the credentials of a peer");
                return None;
            }
        };
        let user_id = credentials.euid()?;
        if user_id != rustix::process::geteuid().as_raw() {
            tracing::warn!(user_id, "rejecting a connection from another user");
            return None;
        }
        Some(PeerInfo {
            process_id: credentials.pid().and_then(|pid| u32::try_from(pid).ok()),
            user_id: Some(user_id),
        })
    }
}

#[cfg(windows)]
mod restrictions {
    use interprocess::local_socket::tokio::{Listener, Stream};
    use interprocess::local_socket::traits::StreamCommon as _;
    use interprocess::os::windows::local_socket::ListenerOptionsExt;
    use interprocess::os::windows::security_descriptor::SecurityDescriptor;
    use widestring::u16cstr;

    use super::create_options;
    use crate::endpoint::Endpoint;
    use crate::error::BindError;
    use crate::server::handler::PeerInfo;

    const OWNER_AND_SYSTEM_ONLY: &widestring::U16CStr = u16cstr!("D:P(A;;GA;;;OW)(A;;GA;;;SY)");

    pub(super) fn create_listener_with_restrictions(
        endpoint: &Endpoint,
    ) -> Result<Listener, BindError> {
        let descriptor = SecurityDescriptor::deserialize(OWNER_AND_SYSTEM_ONLY)?;
        let options = create_options(endpoint)?.security_descriptor(descriptor);
        Ok(options.create_tokio()?)
    }

    #[allow(clippy::unnecessary_wraps)]
    pub(crate) fn inspect_peer(stream: &Stream) -> Option<PeerInfo> {
        let process_id = stream
            .peer_creds()
            .ok()
            .and_then(|credentials| credentials.pid());
        Some(PeerInfo {
            process_id,
            user_id: None,
        })
    }
}

use restrictions::create_listener_with_restrictions;
pub(crate) use restrictions::inspect_peer;

fn create_options(endpoint: &Endpoint) -> Result<ListenerOptions<'_>, BindError> {
    Ok(ListenerOptions::new().name(endpoint.local_socket_name()?))
}

#[cfg(all(test, unix))]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use pretty_assertions::assert_eq;

    use super::*;

    const GROUP_AND_OTHER_BITS: u32 = 0o077;

    #[tokio::test]
    async fn should_create_the_directory_closed_to_others_when_it_is_required_private() {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("tunneler-1000");
        let endpoint = Endpoint::from_path_in_private_directory(directory.join("tunneler.sock"));

        let _listener = create_listener(&endpoint).unwrap();

        let mode = std::fs::metadata(&directory).unwrap().permissions().mode();
        assert_eq!(mode & GROUP_AND_OTHER_BITS, 0);
    }

    #[tokio::test]
    async fn should_refuse_an_existing_directory_open_to_others_when_it_is_required_private() {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("tunneler-1000");
        std::fs::create_dir(&directory).unwrap();
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o755)).unwrap();
        let endpoint = Endpoint::from_path_in_private_directory(directory.join("tunneler.sock"));

        let error = create_listener(&endpoint).err().unwrap();

        assert!(matches!(error, BindError::InsecureDirectory { .. }));
    }

    #[tokio::test]
    async fn should_fail_with_io_when_the_directory_of_a_plain_endpoint_is_missing() {
        let root = tempfile::tempdir().unwrap();
        let endpoint = Endpoint::from_path(root.path().join("missing").join("tunneler.sock"));

        let error = create_listener(&endpoint).err().unwrap();

        assert!(matches!(error, BindError::Io(_)));
    }
}
