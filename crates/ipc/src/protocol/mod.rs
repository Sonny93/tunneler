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
    HostKeyAlertReasonDto, HostKeyDecisionDto, HostOptionDto, ImportCandidateDto, ImportPreviewDto,
    ImportWarningDto, LogEntryDto, LogLevelDto, MetricsDto, PortOwnerDto, PromptDto, RememberDto,
    ResolvedHostDto, SettingsDto, SnapshotDto, TargetHealthDto, TemporaryTunnelSpecDto,
    TrayStatusDto, TunnelDefinitionDto, TunnelDto, TunnelKindDto, TunnelSpecDto, TunnelStateDto,
    X11AnchorDto, X11LauncherDto, X11StateDto,
};
pub use error::{ErrorCode, IpcError};
pub use event::{
    ConfigInvalidEvent, CredentialRequestEvent, EventMessage, ExternalsChangedEvent,
    HostKeyRequestEvent, PromptResolvedEvent, TrayChangedEvent, TunnelChangedEvent, TunnelLogEvent,
    TunnelMetricsEvent, X11ChangedEvent,
};
pub use frame::{MalformedFrame, Outcome, ServerFrame, encode_event, encode_response};
pub use hello::{ClientKind, Hello, HelloResult, PROTOCOL_VERSION};
pub use line::{LineRead, MAX_LINE_BYTES, encode_line, read_line};
pub use request::{
    CredentialAnswer, CredentialResponseParams, HostKeyResponseParams, HostParams,
    ImportScriptParams, MAX_IMPORT_SCRIPT_BYTES, METHOD_NAMES, ProcessParams, QuitParams, Request,
    RequestEnvelope, RequestRejection, TagParams, TagRenameParams, TunnelIdParams, X11HostParams,
    X11LaunchParams,
};
pub use response::{
    AdoptResult, CheckTargetResult, DaemonPathsResult, DeleteTagResult, EmptyResult,
    ForgetSecretsResult, HostsResult, LogsResult, RenameTagResult, ResponsePayload, UiClaimResult,
};
pub use secret::Secret;
