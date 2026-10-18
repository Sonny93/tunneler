//! Releases the console window of a daemon that nobody started from a terminal.
//!
//! `tunneler.exe` is a console-subsystem program because it is also the CLI.
//! Started from Explorer or by the login entry, Windows gives it a console
//! window of its own that would stay open for the daemon's whole life. When no
//! other process shares that console, the daemon detaches from it.

/// Whether the console belongs to this process alone.
///
/// A count of zero means the process has no console, so there is nothing to
/// release.
fn should_release_console(attached_process_count: u32) -> bool {
    attached_process_count == 1
}

/// Closes the console window when this process is the only one attached to it.
///
/// Does nothing outside Windows, and on Windows when the daemon was started
/// from a terminal, which keeps its console. Call it before the logging is set
/// up: once the console is gone, standard error is no longer a terminal.
pub(crate) fn release_console_if_owned() {
    if should_release_console(platform::attached_process_count()) {
        platform::free_console();
    }
}

#[cfg(windows)]
#[allow(unsafe_code)]
mod platform {
    use windows_sys::Win32::System::Console::{FreeConsole, GetConsoleProcessList};

    pub(super) fn attached_process_count() -> u32 {
        let mut process_ids = [0_u32; 1];
        // SAFETY: the pointer refers to a live buffer of exactly the length
        // passed, and the call only writes within that length.
        unsafe { GetConsoleProcessList(process_ids.as_mut_ptr(), 1) }
    }

    pub(super) fn free_console() {
        // SAFETY: the call takes no arguments and only detaches this process
        // from its console; a failure leaves the console attached.
        unsafe { FreeConsole() };
    }
}

#[cfg(not(windows))]
mod platform {
    pub(super) fn attached_process_count() -> u32 {
        0
    }

    pub(super) fn free_console() {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_release_the_console_when_this_process_is_its_only_user() {
        assert!(should_release_console(1));
    }

    #[test]
    fn should_keep_the_console_when_a_shell_shares_it() {
        assert!(!should_release_console(2));
    }

    #[test]
    fn should_keep_nothing_to_release_when_the_process_has_no_console() {
        assert!(!should_release_console(0));
    }
}
