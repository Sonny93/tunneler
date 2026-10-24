//! The X servers installed on this machine, and how to run them.

use std::io;
use std::path::{Path, PathBuf};

use engine::{LocalX11Servers, X11Server};

use super::launch::spawn_detached;

const PROGRAM_FILES_VARIABLE: &str = "ProgramFiles";
const PROGRAM_FILES_X86_VARIABLE: &str = "ProgramFiles(x86)";

/// The X servers the daemon knows how to run, in the order it prefers them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ServerKind {
    VcXsrv,
    Xming,
}

impl ServerKind {
    fn name(self) -> &'static str {
        match self {
            Self::VcXsrv => "VcXsrv",
            Self::Xming => "Xming",
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        [Self::VcXsrv, Self::Xming]
            .into_iter()
            .find(|kind| kind.name() == name)
    }

    /// Never `-ac`: local connections are allowed by the default host list of
    /// the server, and `-ac` would open the display to the network.
    fn arguments(self, display_number: u16) -> Vec<String> {
        let display = format!(":{display_number}");
        let extra_arguments: &[&str] = match self {
            Self::VcXsrv => &["-multiwindow", "-clipboard", "-wgl"],
            Self::Xming => &["-multiwindow", "-clipboard"],
        };
        std::iter::once(display)
            .chain(
                extra_arguments
                    .iter()
                    .map(|argument| (*argument).to_owned()),
            )
            .collect()
    }
}

/// Finds the first installed X server: `VcXsrv`, then Xming in `ProgramFiles(x86)`,
/// then Xming in `ProgramFiles`.
fn locate_server(
    program_files: Option<&Path>,
    program_files_x86: Option<&Path>,
    is_file: impl Fn(&Path) -> bool,
) -> Option<X11Server> {
    let candidates = [
        (ServerKind::VcXsrv, program_files, "VcXsrv/vcxsrv.exe"),
        (ServerKind::Xming, program_files_x86, "Xming/Xming.exe"),
        (ServerKind::Xming, program_files, "Xming/Xming.exe"),
    ];
    candidates
        .into_iter()
        .find_map(|(kind, directory, relative_path)| {
            let executable = directory?.join(relative_path);
            is_file(&executable).then(|| X11Server {
                name: kind.name().to_owned(),
                executable,
            })
        })
}

/// The real machine. Only Windows has servers to find; elsewhere the user
/// runs their own.
pub(crate) struct SystemX11Servers;

impl LocalX11Servers for SystemX11Servers {
    fn find(&self) -> Option<X11Server> {
        if !cfg!(windows) {
            return None;
        }
        let program_files = std::env::var_os(PROGRAM_FILES_VARIABLE).map(PathBuf::from);
        let program_files_x86 = std::env::var_os(PROGRAM_FILES_X86_VARIABLE).map(PathBuf::from);
        locate_server(
            program_files.as_deref(),
            program_files_x86.as_deref(),
            Path::is_file,
        )
    }

    fn start(&self, server: &X11Server, display_number: u16) -> io::Result<()> {
        let kind = ServerKind::from_name(&server.name).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("unknown X server {}", server.name),
            )
        })?;
        let executable = server.executable.to_str().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "the X server path is not valid Unicode",
            )
        })?;
        spawn_detached(executable, &kind.arguments(display_number))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn locate(installed: &[&str]) -> Option<X11Server> {
        let installed: Vec<PathBuf> = installed.iter().map(PathBuf::from).collect();
        locate_server(Some(Path::new("/pf")), Some(Path::new("/pf86")), |path| {
            installed.iter().any(|candidate| candidate == path)
        })
    }

    #[test]
    fn should_run_vcxsrv_with_a_multiwindow_clipboard_and_wgl_display() {
        assert_eq!(
            ServerKind::VcXsrv.arguments(3),
            [":3", "-multiwindow", "-clipboard", "-wgl"]
        );
    }

    #[test]
    fn should_run_xming_with_a_multiwindow_clipboard_display() {
        assert_eq!(
            ServerKind::Xming.arguments(0),
            [":0", "-multiwindow", "-clipboard"]
        );
    }

    #[test]
    fn should_never_disable_access_control_when_running_a_server() {
        for kind in [ServerKind::VcXsrv, ServerKind::Xming] {
            assert!(!kind.arguments(0).contains(&"-ac".to_owned()));
        }
    }

    #[test]
    fn should_prefer_vcxsrv_when_both_servers_are_installed() {
        let server = locate(&["/pf/Xming/Xming.exe", "/pf/VcXsrv/vcxsrv.exe"]).unwrap();

        assert_eq!(server.name, "VcXsrv");
        assert_eq!(server.executable, PathBuf::from("/pf/VcXsrv/vcxsrv.exe"));
    }

    #[test]
    fn should_prefer_xming_of_the_x86_directory_when_it_is_installed_in_both() {
        let server = locate(&["/pf/Xming/Xming.exe", "/pf86/Xming/Xming.exe"]).unwrap();

        assert_eq!(server.executable, PathBuf::from("/pf86/Xming/Xming.exe"));
    }

    #[test]
    fn should_find_xming_when_it_is_only_in_program_files() {
        let server = locate(&["/pf/Xming/Xming.exe"]).unwrap();

        assert_eq!(server.name, "Xming");
    }

    #[test]
    fn should_find_nothing_when_no_server_is_installed() {
        assert_eq!(locate(&[]), None);
    }

    #[test]
    fn should_find_nothing_when_the_program_files_variables_are_unset() {
        assert_eq!(locate_server(None, None, |_| true), None);
    }

    #[test]
    fn should_refuse_to_start_a_server_it_does_not_know() {
        let unknown = X11Server {
            name: "Mystery".to_owned(),
            executable: PathBuf::from("/opt/mystery"),
        };

        let outcome = SystemX11Servers.start(&unknown, 0);

        assert_eq!(outcome.unwrap_err().kind(), io::ErrorKind::InvalidInput);
    }
}
