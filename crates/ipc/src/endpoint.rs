use std::io;
use std::path::{Path, PathBuf};

use interprocess::local_socket::tokio::Stream;
use interprocess::local_socket::traits::tokio::Stream as _;
use interprocess::local_socket::{GenericFilePath, GenericNamespaced, Name, ToFsName, ToNsName};

use crate::error::EndpointError;

#[cfg(unix)]
const SOCKET_FILE_NAME: &str = "tunneler.sock";
#[cfg(unix)]
const FALLBACK_DIRECTORY_PREFIX: &str = "tunneler-";
#[cfg(windows)]
const PIPE_NAME_PREFIX: &str = "tunneler-";

#[derive(Debug, Clone, PartialEq, Eq)]
enum Location {
    Path(PathBuf),
    PipeName(String),
}

/// Where the daemon listens: a socket file on Unix, a named pipe on Windows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint {
    location: Location,
    requires_private_directory: bool,
}

impl Endpoint {
    /// An endpoint at an explicit location, for `--socket`, `TUNNELER_SOCKET`
    /// and tests. On Windows the path must be a `\\.\pipe\` path.
    pub fn from_path(path: impl Into<PathBuf>) -> Self {
        Self {
            location: Location::Path(path.into()),
            requires_private_directory: false,
        }
    }

    /// An endpoint named in the platform's namespace: a named pipe on Windows,
    /// an abstract socket on Linux, a file in the temporary directory elsewhere.
    pub fn from_name(name: impl Into<String>) -> Self {
        Self {
            location: Location::PipeName(name.into()),
            requires_private_directory: false,
        }
    }

    /// The socket file, when the endpoint is one.
    pub fn socket_path(&self) -> Option<&Path> {
        match &self.location {
            Location::Path(path) => Some(path),
            Location::PipeName(_) => None,
        }
    }

    #[cfg(unix)]
    pub(crate) fn requires_private_directory(&self) -> bool {
        self.requires_private_directory
    }

    #[cfg(all(test, unix))]
    pub(crate) fn from_path_in_private_directory(path: impl Into<PathBuf>) -> Self {
        Self {
            location: Location::Path(path.into()),
            requires_private_directory: true,
        }
    }

    pub(crate) fn local_socket_name(&self) -> io::Result<Name<'_>> {
        match &self.location {
            Location::Path(path) => path.as_os_str().to_fs_name::<GenericFilePath>(),
            Location::PipeName(name) => name.as_str().to_ns_name::<GenericNamespaced>(),
        }
    }

    /// Whether something accepts connections on the endpoint.
    pub async fn is_daemon_alive(&self) -> bool {
        let Ok(name) = self.local_socket_name() else {
            return false;
        };
        Stream::connect(name).await.is_ok()
    }

    /// Removes the socket file an earlier daemon left behind after crashing.
    ///
    /// The file is removed only when it is a socket and nothing accepts
    /// connections on it. Returns whether it was removed. A named pipe vanishes
    /// with its owner, so on Windows this does nothing and returns `false`.
    ///
    /// # Errors
    ///
    /// Fails when the socket cannot be inspected or removed.
    pub fn reclaim_stale(&self) -> io::Result<bool> {
        let Location::Path(path) = &self.location else {
            return Ok(false);
        };
        reclaim_stale_socket(path)
    }
}

#[cfg(unix)]
fn reclaim_stale_socket(path: &Path) -> io::Result<bool> {
    use std::os::unix::fs::FileTypeExt;
    use std::os::unix::net::UnixStream;

    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_socket() => {}
        Ok(_) => return Ok(false),
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    }
    match UnixStream::connect(path) {
        Ok(_) => Ok(false),
        Err(error) if error.kind() == io::ErrorKind::ConnectionRefused => {
            std::fs::remove_file(path)?;
            Ok(true)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

#[cfg(not(unix))]
#[allow(clippy::unnecessary_wraps)]
fn reclaim_stale_socket(_path: &Path) -> io::Result<bool> {
    Ok(false)
}

/// The endpoint the daemon and its clients use unless told otherwise.
///
/// - Linux and macOS: `$XDG_RUNTIME_DIR/tunneler.sock`. Without a usable
///   `$XDG_RUNTIME_DIR`, `tunneler-<uid>/tunneler.sock` under the temporary
///   directory (`$TMPDIR`, else `/tmp`), a directory the server creates with mode
///   `0700` and refuses to use unless you own it and others cannot enter it.
/// - Windows: the named pipe `tunneler-<user name>`.
///
/// # Errors
///
/// On Windows, fails when the user name cannot be determined.
#[cfg(unix)]
#[allow(clippy::unnecessary_wraps)]
pub fn default_endpoint() -> Result<Endpoint, EndpointError> {
    let runtime_directory = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from);
    let user_id = rustix::process::geteuid().as_raw();
    Ok(unix_endpoint(
        runtime_directory,
        &std::env::temp_dir(),
        user_id,
    ))
}

#[cfg(unix)]
fn unix_endpoint(
    runtime_directory: Option<PathBuf>,
    temporary_directory: &Path,
    user_id: u32,
) -> Endpoint {
    if let Some(directory) = runtime_directory.filter(|directory| directory.is_absolute()) {
        return Endpoint::from_path(directory.join(SOCKET_FILE_NAME));
    }
    let directory = temporary_directory.join(format!("{FALLBACK_DIRECTORY_PREFIX}{user_id}"));
    Endpoint {
        location: Location::Path(directory.join(SOCKET_FILE_NAME)),
        requires_private_directory: true,
    }
}

/// The endpoint the daemon and its clients use unless told otherwise.
///
/// On Windows, the named pipe `tunneler-<user name>`.
///
/// # Errors
///
/// Fails when the user name cannot be determined.
#[cfg(windows)]
pub fn default_endpoint() -> Result<Endpoint, EndpointError> {
    let user_name =
        whoami::username().map_err(|error| EndpointError::UnknownUser(error.to_string()))?;
    Ok(Endpoint::from_name(pipe_name_for(&user_name)))
}

#[cfg(windows)]
fn pipe_name_for(user_name: &str) -> String {
    let sanitized: String = user_name
        .chars()
        .map(|character| {
            if character.is_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect();
    format!("{PIPE_NAME_PREFIX}{sanitized}")
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    const USER_ID: u32 = 1000;

    #[test]
    fn should_use_the_runtime_directory_without_requiring_a_private_one_when_it_is_set() {
        let endpoint = unix_endpoint(
            Some(PathBuf::from("/run/user/1000")),
            Path::new("/tmp"),
            USER_ID,
        );

        assert_eq!(
            endpoint.socket_path(),
            Some(Path::new("/run/user/1000/tunneler.sock"))
        );
        assert!(!endpoint.requires_private_directory());
    }

    #[test]
    fn should_fall_back_to_a_private_per_user_directory_when_the_runtime_directory_is_unset() {
        let endpoint = unix_endpoint(None, Path::new("/tmp"), USER_ID);

        assert_eq!(
            endpoint.socket_path(),
            Some(Path::new("/tmp/tunneler-1000/tunneler.sock"))
        );
        assert!(endpoint.requires_private_directory());
    }

    #[test]
    fn should_fall_back_when_the_runtime_directory_is_relative() {
        let endpoint = unix_endpoint(
            Some(PathBuf::from("relative")),
            Path::new("/var/tmp"),
            USER_ID,
        );

        assert_eq!(
            endpoint.socket_path(),
            Some(Path::new("/var/tmp/tunneler-1000/tunneler.sock"))
        );
    }

    #[test]
    fn should_not_reclaim_anything_when_the_path_does_not_exist() {
        let endpoint = Endpoint::from_path("/nonexistent-directory/tunneler.sock");

        assert!(!endpoint.reclaim_stale().unwrap());
    }

    #[test]
    fn should_not_reclaim_a_regular_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("tunneler.sock");
        std::fs::write(&path, b"precious").unwrap();

        let removed = Endpoint::from_path(&path).reclaim_stale().unwrap();

        assert!(!removed && path.exists());
    }
}
