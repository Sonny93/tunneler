//! Lets the window of another process come to the front.
//!
//! Windows only lets the process that has the focus, or one it allowed, take
//! the foreground. When a link starts a second UI that hands the link over to
//! the open one, the second UI is the one the browser just gave the focus to:
//! it must allow the first one before it asks for the link to be shown.

/// Allows any process to take the foreground window. Does nothing outside
/// Windows; a failure only means the window may stay behind, so it is ignored.
pub fn allow_any_process_to_take_the_foreground() {
    platform::allow_any_process_to_take_the_foreground();
}

#[cfg(windows)]
#[allow(unsafe_code)]
mod platform {
    use windows_sys::Win32::UI::WindowsAndMessaging::{ASFW_ANY, AllowSetForegroundWindow};

    pub(super) fn allow_any_process_to_take_the_foreground() {
        // SAFETY: the call takes the `ASFW_ANY` constant and only changes which
        // processes may take the foreground; a failure leaves the permission
        // as it was.
        unsafe { AllowSetForegroundWindow(ASFW_ANY) };
    }
}

#[cfg(not(windows))]
mod platform {
    pub(super) fn allow_any_process_to_take_the_foreground() {}
}
