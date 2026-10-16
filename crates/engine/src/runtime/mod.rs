//! The tunnel runtime: local forwards driven by one task per tunnel, sharing
//! one SSH session per host, reconnecting after losses, reporting state,
//! health, metrics and logs through the engine event bus.

mod backoff;
mod catalog;
mod counting_stream;
mod definition_diff;
mod engine;
mod events;
mod failure;
mod forward_run;
mod local_forward;
mod log_buffer;
mod metrics;
mod options;
mod reload;
mod retry;
mod session_manager;
mod shared;
mod state;
mod tunnel_actor;

pub use catalog::TunnelSnapshot;
pub use engine::{Engine, EngineDependencies, EngineError, EngineSnapshot, NewTunnel};
pub use events::EngineEvent;
pub use log_buffer::{LogLevel, TunnelLogEntry};
pub use metrics::TunnelMetrics;
pub use options::EngineOptions;
pub use state::{FailureReason, TargetHealth, TunnelState, TunnelStatus};
