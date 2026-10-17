use serde::{Deserialize, Serialize};

use crate::protocol::dto::{
    ImportPreviewDto, LogEntryDto, ResolvedHostDto, SnapshotDto, TunnelDto,
};
use crate::protocol::hello::HelloResult;

/// The result of a method that has nothing to return: `{}`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmptyResult {}

/// The result of `tunnel.logs`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogsResult {
    /// The recent entries, oldest first.
    pub entries: Vec<LogEntryDto>,
}

/// The result of `hosts.list`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostsResult {
    /// The concrete `Host` aliases, in the order the `ssh_config` declares them.
    pub hosts: Vec<String>,
}

/// The result of `external.adopt`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoptResult {
    /// The identifiers of the tunnels that were started.
    pub ids: Vec<String>,
}

/// The result of `secrets.forget`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForgetSecretsResult {
    /// How many identity files had a remembered passphrase.
    pub forgotten_count: u64,
}

/// What a handler answers on success; serialized as the bare `result` value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum ResponsePayload {
    /// Nothing to report.
    Empty(EmptyResult),
    /// The answer to `hello`.
    Hello(HelloResult),
    /// The answer to `state.snapshot`.
    Snapshot(SnapshotDto),
    /// The answer to `tunnel.create` and `tunnel.update`.
    Tunnel(TunnelDto),
    /// The answer to `tunnel.logs`.
    Logs(LogsResult),
    /// The answer to `hosts.list`.
    Hosts(HostsResult),
    /// The answer to `hosts.resolve`.
    ResolvedHost(ResolvedHostDto),
    /// The answer to `import.ssh_config` and `import.script`.
    ImportPreview(ImportPreviewDto),
    /// The answer to `external.adopt`.
    Adopt(AdoptResult),
    /// The answer to `secrets.forget`.
    ForgetSecrets(ForgetSecretsResult),
}

impl ResponsePayload {
    /// The payload of methods that have nothing to return.
    pub const fn empty() -> Self {
        Self::Empty(EmptyResult {})
    }
}
