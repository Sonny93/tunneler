use std::path::PathBuf;

use tracing::warn;

const DISPLAY_VARIABLE: &str = "DISPLAY";
const XAUTHORITY_VARIABLE: &str = "XAUTHORITY";

/// The family of operating systems whose X conventions differ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayPlatform {
    /// Unix sockets exist; there is no default display.
    Unix,
    /// No Unix sockets for X; the default display is `127.0.0.1:0.0`.
    Windows,
}

impl DisplayPlatform {
    /// The platform this program was built for.
    pub const fn current() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else {
            Self::Unix
        }
    }
}

/// What the engine reads from the machine to find the local X server and
/// its credentials; tests provide a fake.
pub trait X11Environment: Send + Sync {
    /// The value of `$DISPLAY`, when set and not empty.
    fn display(&self) -> Option<String>;

    /// The value of `$XAUTHORITY`, when set and not empty.
    fn xauthority_path(&self) -> Option<PathBuf>;

    /// The home directory of the user, where `.Xauthority` lives by default.
    fn home_directory(&self) -> Option<PathBuf>;

    /// The host name, which `Xauthority` entries of the local family carry.
    fn host_name(&self) -> String;

    /// The platform, which decides the default display and the meaning of an
    /// empty host.
    fn platform(&self) -> DisplayPlatform;
}

/// The real machine.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemX11Environment;

impl X11Environment for SystemX11Environment {
    fn display(&self) -> Option<String> {
        non_empty_variable(DISPLAY_VARIABLE)
    }

    fn xauthority_path(&self) -> Option<PathBuf> {
        non_empty_variable(XAUTHORITY_VARIABLE).map(PathBuf::from)
    }

    fn home_directory(&self) -> Option<PathBuf> {
        std::env::home_dir()
    }

    fn host_name(&self) -> String {
        whoami::hostname().unwrap_or_else(|error| {
            warn!(%error, "cannot read the host name; Xauthority entries of the local family are ignored");
            String::new()
        })
    }

    fn platform(&self) -> DisplayPlatform {
        DisplayPlatform::current()
    }
}

fn non_empty_variable(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}
