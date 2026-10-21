//! Fixtures shared by the integration tests of several crates: an in-process
//! russh server, throwaway keys, TCP listeners, an SSH agent and the client side
//! files (`ssh_config`, `known_hosts`).
//!
//! The crate never depends on the engine: a dev-dependency cycle would give the
//! tests two copies of the engine types. Fakes of engine traits live next to the
//! engine tests instead.
//!
//! It is test code only, so helpers panic on failure rather than return errors.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers fail by panicking"
)]
#![warn(missing_docs)]

#[cfg(unix)]
mod agent;
mod keys;
mod listeners;
mod server;
mod workspace;

#[cfg(unix)]
pub use agent::TestAgent;
pub use keys::{fingerprint_of, fixture_key, generate_key, write_private_key};
pub use listeners::{EchoServer, SilentListener, free_port, free_ports};
pub use server::{TestServer, TestServerBuilder, TestUser, X11Request};
pub use workspace::{ALIAS, HostSpec, REMOTE_USER, Workspace};
