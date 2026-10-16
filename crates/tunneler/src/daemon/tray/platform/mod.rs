//! The tray thread of each platform. Each one exposes `spawn`, which starts
//! the thread that owns the icon, and guarantees that when the thread reports
//! `Ok` through `built`, the [`TrayStatus`](super::TrayStatus) holds the
//! initial status of the icon.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(not(any(target_os = "linux", windows)))]
mod unsupported;
#[cfg(windows)]
mod windows;

#[cfg(target_os = "linux")]
pub(super) use linux::spawn;
#[cfg(not(any(target_os = "linux", windows)))]
pub(super) use unsupported::spawn;
#[cfg(windows)]
pub(super) use windows::spawn;

/// The name of the thread that owns the tray icon.
#[cfg(any(target_os = "linux", windows))]
const TRAY_THREAD_NAME: &str = "tray";
