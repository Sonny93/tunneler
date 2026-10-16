#![allow(dead_code, unused_imports, clippy::unwrap_used, clippy::expect_used)]

//! Fixtures for the integration tests. The in-process russh server, keys,
//! listeners, agent and client-side files come from the `test-support` crate
//! and are re-exported here; this module adds the fakes of engine traits: a
//! hermetic local environment, scripted prompter/verifier, a manual clock and
//! the engine harness.

#[cfg(unix)]
mod agent;
mod clock;
mod engine_harness;
mod environment;
mod scripted;
mod workspace;

#[cfg(unix)]
pub use agent::FixedAgentLocator;
pub use clock::ManualClock;
pub use engine_harness::{wait_for_event, wait_for_state, wait_until};
pub use environment::FakeEnvironment;
pub use scripted::{ScriptedPrompter, ScriptedVerifier};
#[cfg(unix)]
pub use test_support::TestAgent;
pub use test_support::{
    ALIAS, EchoServer, HostSpec, REMOTE_USER, SilentListener, TestServer, TestServerBuilder,
    TestUser, fingerprint_of, fixture_key, free_port, generate_key, write_private_key,
};
pub use workspace::{NoAgentLocator, Workspace, quiet_prompter, rejecting_verifier};
