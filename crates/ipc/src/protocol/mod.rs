//! The wire protocol: JSON lines, one object per `\n`.
//!
//! Requests carry an `id` the response echoes; events carry none. See
//! [`Request`] for the methods and [`EventMessage`] for the events.

mod devslot;
mod devslot_names;
mod devslot_up;
mod dto;
mod error;
mod event;
mod frame;
mod hello;
mod line;
mod request;
mod response;
mod secret;

pub use devslot::{
    DevslotDiscoveryDto, DevslotInstanceDto, DevslotOriginDto, DevslotPortDto,
    DevslotPortStatusDto, DevslotResyncParams, DevslotResyncResultDto, DevslotResyncTunnelDto,
    DevslotStaleActionDto, DevslotStaleDto, DevslotStaleReasonDto, DevslotWarningDto,
};
pub use devslot_names::{
    DevslotHostName, DevslotInstanceName, DevslotNameError, DevslotNameProblem,
    MAX_DEVSLOT_HOST_LENGTH, MAX_DEVSLOT_INSTANCE_LENGTH, find_devslot_host_name_problem,
    find_devslot_instance_name_problem,
};
pub use devslot_up::{
    DevslotFailedPortDto, DevslotLinkDto, DevslotRemovedRouteDto, DevslotSkipReasonDto,
    DevslotSkippedPortDto, DevslotUpParams, DevslotUpPortDto, DevslotUpResultDto,
};
pub use dto::DevslotStalePolicyDto;
pub use dto::{
    AvailableOpenerDto, CredentialRequestDto, DefaultOpenerDto, EpochMillis, ExternalForwardDto,
    ExternalProcessDto, FailureReasonDto, HostKeyAlertReasonDto, HostKeyDecisionDto, HostOptionDto,
    ImportCandidateDto, ImportPreviewDto, ImportWarningDto, LanguageDto, LogEntryDto, LogLevelDto,
    MetricsDto, OpenDefaultDto, OpenHostDto, OpenOptionsDto, OpenSchemeDto, OpenerSourceDto,
    OpenerSpecDto, PortOwnerDto, PromptDto, RememberDto, ResolvedHostDto, SettingsDto, SnapshotDto,
    SoundEventDto, StandbyNotifyDto, StandbySoundDto, TargetHealthDto, TemporaryTunnelSpecDto,
    TrayStatusDto, TunnelDefinitionDto, TunnelDto, TunnelKindDto, TunnelSpecDto, TunnelStateDto,
    X11AnchorDto, X11LauncherDto, X11ProbeDto, X11ServerStartDto, X11StateDto,
};
pub use error::{ErrorCode, IpcError};
pub use event::{
    ConfigInvalidEvent, CredentialRequestEvent, EventMessage, ExternalsChangedEvent,
    HostKeyRequestEvent, PromptResolvedEvent, ShowRequestedEvent, TrayChangedEvent,
    TunnelChangedEvent, TunnelLogEvent, TunnelMetricsEvent, X11ChangedEvent,
};
pub use frame::{MalformedFrame, Outcome, ServerFrame, encode_event, encode_response};
pub use hello::{ClientKind, Hello, HelloResult, PROTOCOL_VERSION};
pub use line::{LineRead, MAX_LINE_BYTES, encode_line, read_line};
pub use request::{
    CredentialAnswer, CredentialResponseParams, HostKeyResponseParams, HostParams,
    ImportScriptParams, MAX_IMPORT_SCRIPT_BYTES, METHOD_NAMES, OpenTunnelParams,
    PreviewSoundParams, ProcessParams, QuitParams, Request, RequestEnvelope, RequestRejection,
    SettingsUpdateParams, TagParams, TagRenameParams, TunnelIdParams, UiClaimParams, X11HostParams,
    X11LaunchParams,
};
pub use response::{
    AdoptResult, CheckTargetResult, DaemonPathsResult, DeleteTagResult, EmptyResult,
    ForgetSecretsResult, HostsResult, LogsResult, OpenTunnelResult, OpenersResult, RenameTagResult,
    ResponsePayload, UiClaimResult,
};
pub use secret::Secret;
