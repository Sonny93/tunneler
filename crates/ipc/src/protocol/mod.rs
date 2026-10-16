//! The wire protocol: JSON lines, one object per `\n`.
//!
//! Requests carry an `id` the response echoes; events carry none. See
//! [`Request`] for the methods and [`EventMessage`] for the events.

mod dto;
mod error;
mod event;
mod frame;
mod hello;
mod line;
mod request;
mod response;
mod secret;

pub use dto::{
    CredentialRequestDto, EpochMillis, ExternalForwardDto, ExternalProcessDto, FailureReasonDto,
    HostKeyAlertReasonDto, HostKeyDecisionDto, LogEntryDto, LogLevelDto, MetricsDto, PortOwnerDto,
    PromptDto, SettingsDto, SnapshotDto, TargetHealthDto, TrayStatusDto, TunnelDefinitionDto,
    TunnelDto, TunnelKindDto, TunnelSpecDto, TunnelStateDto,
};
pub use error::{ErrorCode, IpcError};
pub use event::{
    ConfigInvalidEvent, CredentialRequestEvent, EventMessage, ExternalsChangedEvent,
    HostKeyRequestEvent, PromptResolvedEvent, TrayChangedEvent, TunnelChangedEvent, TunnelLogEvent,
    TunnelMetricsEvent,
};
pub use frame::{MalformedFrame, Outcome, ServerFrame, encode_event, encode_response};
pub use hello::{ClientKind, Hello, HelloResult, PROTOCOL_VERSION};
pub use line::{LineRead, MAX_LINE_BYTES, encode_line, read_line};
pub use request::{
    CredentialAnswer, CredentialResponseParams, HostKeyResponseParams, METHOD_NAMES, ProcessParams,
    QuitParams, Request, RequestEnvelope, RequestRejection, TagParams, TunnelIdParams,
};
pub use response::{AdoptResult, EmptyResult, HostsResult, LogsResult, ResponsePayload};
pub use secret::Secret;
