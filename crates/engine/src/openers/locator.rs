use std::env;
use std::path::Path;

/// Finds programs on this machine.
pub trait ProgramLocator: Send + Sync {
    /// Whether `program` can be run: an absolute path that exists, or a name
    /// found in the `PATH`.
    fn is_available(&self, program: &str) -> bool;
}

/// Looks in the real file system and the `PATH`.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemProgramLocator;

impl ProgramLocator for SystemProgramLocator {
    fn is_available(&self, program: &str) -> bool {
        let path = Path::new(program);
        if path.is_absolute() {
            return is_runnable(path);
        }
        let Some(search_path) = env::var_os("PATH") else {
            return false;
        };
        env::split_paths(&search_path).any(|directory| is_runnable(&directory.join(program)))
    }
}

#[cfg(unix)]
fn is_runnable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    const EXECUTE_BITS: u32 = 0o111;
    path.metadata().is_ok_and(|metadata| {
        metadata.is_file() && metadata.permissions().mode() & EXECUTE_BITS != 0
    })
}

#[cfg(not(unix))]
fn is_runnable(path: &Path) -> bool {
    if path.is_file() {
        return true;
    }
    let mut with_extension = std::path::PathBuf::from(path);
    with_extension.set_extension("exe");
    with_extension.is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_not_find_a_program_that_does_not_exist() {
        assert!(!SystemProgramLocator.is_available("tunneler-no-such-program"));
    }

    #[cfg(unix)]
    #[test]
    fn should_find_a_program_on_the_path() {
        assert!(SystemProgramLocator.is_available("sh"));
    }

    #[cfg(unix)]
    #[test]
    fn should_find_an_absolute_path_that_exists() {
        assert!(SystemProgramLocator.is_available("/bin/sh"));
    }

    #[cfg(unix)]
    #[test]
    fn should_not_find_an_absolute_path_that_is_not_executable() {
        assert!(!SystemProgramLocator.is_available("/etc/passwd"));
    }
}
