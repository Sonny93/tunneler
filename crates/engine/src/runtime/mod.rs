//! The tunnel runtime: local, remote and dynamic forwards driven by one task
//! per tunnel, sharing one SSH session per host, reconnecting after losses,
//! reporting state, health, metrics and logs through the engine event bus.
//! It also runs the X11 anchor of each host and its X11 launchers.

mod backoff;
mod catalog;
mod counting_stream;
mod definition_diff;
mod dynamic_forward;
mod engine;
mod events;
mod failure;
mod forward_run;
mod local_forward;
mod log_buffer;
mod metrics;
mod options;
mod port_watch;
mod reload;
mod remote_forward;
mod retry;
mod session_manager;
mod shared;
mod state;
mod tunnel_actor;
mod x11_anchor;
mod x11_anchors;
mod x11_launcher;
mod x11_state;

pub use catalog::TunnelSnapshot;
pub use engine::{Engine, EngineDependencies, EngineError, EngineSnapshot, NewTunnel};
pub use events::EngineEvent;
pub use log_buffer::{LogLevel, TunnelLogEntry};
pub use metrics::TunnelMetrics;
pub use options::EngineOptions;
pub use state::{FailureReason, TargetHealth, TunnelState, TunnelStatus};
pub use x11_state::{X11AnchorSnapshot, X11AnchorState};
