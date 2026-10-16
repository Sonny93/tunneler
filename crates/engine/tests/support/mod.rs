#![allow(dead_code, unused_imports, clippy::unwrap_used, clippy::expect_used)]

//! Shared fixtures for the integration tests: an in-process russh server,
//! throwaway keys, `ssh_config` and `known_hosts` files, a hermetic local
//! environment and scripted prompter/verifier fakes.

mod environment;
mod keys;
mod listeners;
mod scripted;
mod server;
mod workspace;

pub use environment::FakeEnvironment;
pub use keys::{fingerprint_of, generate_key, write_private_key};
pub use listeners::{EchoServer, SilentListener};
pub use scripted::{ScriptedPrompter, ScriptedVerifier};
pub use server::{TestServer, TestServerBuilder, TestUser};
pub use workspace::{ALIAS, REMOTE_USER, Workspace, quiet_prompter, rejecting_verifier};
