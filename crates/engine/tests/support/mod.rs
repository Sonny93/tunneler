#![allow(dead_code, unused_imports, clippy::unwrap_used, clippy::expect_used)]

//! Shared fixtures for the integration tests: an in-process russh server,
//! throwaway keys, `ssh_config` and `known_hosts` files, a hermetic local
//! environment and scripted prompter/verifier fakes.

#[cfg(unix)]
mod agent;
mod clock;
mod engine_harness;
mod environment;
mod keys;
mod listeners;
mod scripted;
mod server;
mod workspace;

#[cfg(unix)]
pub use agent::{FixedAgentLocator, TestAgent};
pub use clock::ManualClock;
pub use engine_harness::{wait_for_event, wait_for_state, wait_until};
pub use environment::FakeEnvironment;
pub use keys::{fingerprint_of, fixture_key, generate_key, write_private_key};
pub use listeners::{EchoServer, SilentListener, free_port};
pub use scripted::{ScriptedPrompter, ScriptedVerifier};
pub use server::{TestServer, TestServerBuilder, TestUser};
pub use workspace::{
    ALIAS, HostSpec, NoAgentLocator, REMOTE_USER, Workspace, quiet_prompter, rejecting_verifier,
};
