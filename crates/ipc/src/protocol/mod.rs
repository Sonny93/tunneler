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
    CredentialRequestDto, EpochMillis, FailureReasonDto, HostKeyAlertReasonDto, HostKeyDecisionDto,
    LogEntryDto, LogLevelDto, MetricsDto, PromptDto, SettingsDto, SnapshotDto, TargetHealthDto,
    TunnelDefinitionDto, TunnelDto, TunnelKindDto, TunnelSpecDto, TunnelStateDto,
};
pub use error::{ErrorCode, IpcError};
pub use event::{
    ConfigInvalidEvent, CredentialRequestEvent, EventMessage, HostKeyRequestEvent,
    PromptResolvedEvent, TunnelChangedEvent, TunnelLogEvent, TunnelMetricsEvent,
};
pub use frame::{MalformedFrame, Outcome, ServerFrame, encode_event, encode_response};
pub use hello::{ClientKind, Hello, HelloResult, PROTOCOL_VERSION};
pub use line::{LineRead, MAX_LINE_BYTES, encode_line, read_line};
pub use request::{
    CredentialAnswer, CredentialResponseParams, HostKeyResponseParams, METHOD_NAMES, QuitParams,
    Request, RequestEnvelope, RequestRejection, TagParams, TunnelIdParams,
};
pub use response::{EmptyResult, LogsResult, ResponsePayload};
pub use secret::Secret;
