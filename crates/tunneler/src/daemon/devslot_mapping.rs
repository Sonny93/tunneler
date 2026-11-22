//! Conversions between the devslot types of the engine and the wire types of
//! the IPC protocol.

use engine::{
    DevslotDiscovery, DevslotError, DevslotFailedPort, DevslotInstance, DevslotPort,
    DevslotPortStatus, DevslotRemovedRoute, DevslotResync, DevslotResyncTunnel, DevslotSkipReason,
    DevslotSkippedPort, DevslotStaleAction, DevslotStaleReason, DevslotStaleTunnel, DevslotUp,
    DevslotUpPort, DevslotWarning,
};
use ipc::protocol::{
    DevslotDiscoveryDto, DevslotFailedPortDto, DevslotInstanceDto, DevslotPortDto,
    DevslotPortStatusDto, DevslotRemovedRouteDto, DevslotResyncResultDto, DevslotResyncTunnelDto,
    DevslotSkipReasonDto, DevslotSkippedPortDto, DevslotStaleActionDto, DevslotStaleDto,
    DevslotStaleReasonDto, DevslotUpPortDto, DevslotUpResultDto, DevslotWarningDto, IpcError,
    MAX_LINE_BYTES, ResponsePayload, encode_response,
};

use super::mapping::{definition_from_dto, definition_to_dto};

const TOO_LARGE_REASON: &str = "the listing is too large to send to the window";
const UP_TOO_LARGE_REASON: &str = "the result is too large to send";

/// What happened when the daemon was asked to open the application of an
/// instance in the browser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OpenOutcome {
    /// Opening was not asked, or there was no application tunnel to open.
    NotAttempted,
    /// The browser was launched on this address.
    Opened(String),
    /// The browser could not be launched; this says why.
    Failed(String),
}

/// Maps a discovery to its DTO, refusing one whose response line the client
/// would reject for its size.
pub(crate) fn discovery_to_payload(
    discovery: &DevslotDiscovery,
) -> Result<DevslotDiscoveryDto, IpcError> {
    let dto = discovery_to_dto(discovery);
    let payload = ResponsePayload::DevslotDiscovery(dto);
    let line = encode_response(Some(u64::MAX), &Ok(payload.clone()))
        .map_err(|error| IpcError::internal(error.to_string()))?;
    if line.len() >= MAX_LINE_BYTES {
        return Err(IpcError::devslot_invalid_output(
            &discovery.host,
            TOO_LARGE_REASON,
        ));
    }
    let ResponsePayload::DevslotDiscovery(dto) = payload else {
        return Err(IpcError::internal("the discovery payload changed shape"));
    };
    Ok(dto)
}

fn discovery_to_dto(discovery: &DevslotDiscovery) -> DevslotDiscoveryDto {
    DevslotDiscoveryDto {
        host: discovery.host.clone(),
        persist_default: discovery.persist_default,
        instances: discovery.instances.iter().map(instance_to_dto).collect(),
        stale: discovery.stale.iter().map(stale_to_dto).collect(),
        warnings: discovery.warnings.iter().map(warning_to_dto).collect(),
    }
}

fn instance_to_dto(instance: &DevslotInstance) -> DevslotInstanceDto {
    DevslotInstanceDto {
        name: instance.name.clone(),
        project: instance.project.clone(),
        project_tag: instance.project_tag.clone(),
        path: instance.path.clone(),
        worktree: instance.is_worktree,
        slot: instance.slot,
        app_url: instance.app_url.clone(),
        services_running: instance.services_running,
        ports: instance.ports.iter().map(port_to_dto).collect(),
    }
}

fn port_to_dto(port: &DevslotPort) -> DevslotPortDto {
    let (status, tunnel_id) = match port.status {
        DevslotPortStatus::New => (DevslotPortStatusDto::New, None),
        DevslotPortStatus::Ambiguous => (DevslotPortStatusDto::Ambiguous, None),
        DevslotPortStatus::AlreadyTunneled { tunnel_id } => (
            DevslotPortStatusDto::AlreadyTunneled,
            Some(tunnel_id.to_string()),
        ),
    };
    DevslotPortDto {
        var: port.var.clone(),
        port: port.port,
        kind: port.kind.clone(),
        service: port.service.clone(),
        target: port.target,
        url: port.url.clone(),
        host_name: port.host_name.clone(),
        forced: port.is_forced,
        local_port_busy: port.is_local_port_busy,
        suggested_local_port: port.suggested_local_port,
        status,
        tunnel_id,
    }
}

fn stale_to_dto(stale: &DevslotStaleTunnel) -> DevslotStaleDto {
    DevslotStaleDto {
        tunnel: definition_to_dto(&stale.definition),
        is_temporary: stale.is_temporary,
        reason: match stale.reason {
            DevslotStaleReason::PortGone => DevslotStaleReasonDto::PortGone,
            DevslotStaleReason::Reassigned => DevslotStaleReasonDto::Reassigned,
        },
        action: match stale.action {
            DevslotStaleAction::Remove => DevslotStaleActionDto::Remove,
            DevslotStaleAction::Ask => DevslotStaleActionDto::Ask,
        },
    }
}

fn warning_to_dto(warning: &DevslotWarning) -> DevslotWarningDto {
    match warning {
        DevslotWarning::InvalidPort { instance, var } => DevslotWarningDto::InvalidPort {
            instance: instance.clone(),
            var: var.clone(),
        },
        DevslotWarning::ProjectWithoutTag { instance } => DevslotWarningDto::ProjectWithoutTag {
            instance: instance.clone(),
        },
    }
}

/// Reads the tunnels of a resync request; a tunnel whose `id` differs from the
/// id of its definition is refused.
pub(crate) fn resync_tunnels_from_dto(
    tunnels: Vec<DevslotResyncTunnelDto>,
) -> Result<Vec<DevslotResyncTunnel>, IpcError> {
    tunnels.into_iter().map(resync_tunnel_from_dto).collect()
}

fn resync_tunnel_from_dto(dto: DevslotResyncTunnelDto) -> Result<DevslotResyncTunnel, IpcError> {
    if dto.id != dto.definition.id {
        return Err(IpcError::invalid_request(format!(
            "the tunnel {} carries the definition of another tunnel",
            dto.id
        )));
    }
    let definition = definition_from_dto(dto.definition)
        .map_err(|error| IpcError::invalid_request(error.to_string()))?;
    Ok(DevslotResyncTunnel {
        definition,
        is_temporary: dto.is_temporary,
    })
}

pub(crate) fn resync_to_dto(resync: &DevslotResync) -> DevslotResyncResultDto {
    DevslotResyncResultDto {
        removed: resync.removed.iter().map(ToString::to_string).collect(),
        skipped: resync.skipped.iter().map(ToString::to_string).collect(),
    }
}

/// Maps what opening an instance did to its DTO, refusing one whose response
/// line the client would reject for its size.
pub(crate) fn up_to_payload(
    up: &DevslotUp,
    opening: OpenOutcome,
) -> Result<DevslotUpResultDto, IpcError> {
    let dto = up_to_dto(up, opening);
    let payload = ResponsePayload::DevslotUp(dto);
    let line = encode_response(Some(u64::MAX), &Ok(payload.clone()))
        .map_err(|error| IpcError::internal(error.to_string()))?;
    if line.len() >= MAX_LINE_BYTES {
        return Err(IpcError::devslot_invalid_output(
            &up.host,
            UP_TOO_LARGE_REASON,
        ));
    }
    let ResponsePayload::DevslotUp(dto) = payload else {
        return Err(IpcError::internal("the result payload changed shape"));
    };
    Ok(dto)
}

fn up_to_dto(up: &DevslotUp, opening: OpenOutcome) -> DevslotUpResultDto {
    let (opened, open_error) = match opening {
        OpenOutcome::NotAttempted => (None, None),
        OpenOutcome::Opened(address) => (Some(address), None),
        OpenOutcome::Failed(message) => (None, Some(message)),
    };
    DevslotUpResultDto {
        host: up.host.clone(),
        instance: up.instance.clone(),
        started: up.started.iter().map(up_port_to_dto).collect(),
        existing: up.existing.iter().map(up_port_to_dto).collect(),
        skipped: up
            .skipped
            .iter()
            .copied()
            .map(skipped_port_to_dto)
            .collect(),
        failed: up.failed.iter().map(failed_port_to_dto).collect(),
        removed: up
            .removed
            .iter()
            .copied()
            .map(removed_route_to_dto)
            .collect(),
        needs_review: u32::try_from(up.needs_review).unwrap_or(u32::MAX),
        app_url: up.app_url.clone(),
        app_port_shared: up.is_app_port_shared,
        opened,
        open_error,
    }
}

fn up_port_to_dto(port: &DevslotUpPort) -> DevslotUpPortDto {
    DevslotUpPortDto {
        remote_port: port.remote_port,
        listen_port: port.listen_port,
        tunnel_id: port.tunnel_id.to_string(),
    }
}

fn skipped_port_to_dto(port: DevslotSkippedPort) -> DevslotSkippedPortDto {
    DevslotSkippedPortDto {
        remote_port: port.remote_port,
        reason: match port.reason {
            DevslotSkipReason::LocalPortBusy => DevslotSkipReasonDto::LocalPortBusy,
            DevslotSkipReason::NeedsReview => DevslotSkipReasonDto::NeedsReview,
            DevslotSkipReason::PrivilegedPort => DevslotSkipReasonDto::PrivilegedPort,
            DevslotSkipReason::ExistingTunnel => DevslotSkipReasonDto::ExistingTunnel,
        },
    }
}

fn failed_port_to_dto(port: &DevslotFailedPort) -> DevslotFailedPortDto {
    DevslotFailedPortDto {
        remote_port: port.remote_port,
        message: port.message.clone(),
    }
}

fn removed_route_to_dto(route: DevslotRemovedRoute) -> DevslotRemovedRouteDto {
    DevslotRemovedRouteDto {
        listen_port: route.listen_port,
        remote_port: route.remote_port,
        is_temporary: route.is_temporary,
    }
}

pub(crate) fn devslot_error_to_ipc(error: &DevslotError) -> IpcError {
    match error {
        DevslotError::InstanceNotFound { host, instance } => {
            IpcError::devslot_instance_not_found(host, instance)
        }
        DevslotError::NotFound { host, path } => IpcError::devslot_not_found(host, path),
        DevslotError::Failed {
            host,
            status,
            stderr,
        } => IpcError::devslot_failed(host, *status, stderr),
        DevslotError::InvalidOutput { host, reason } => {
            IpcError::devslot_invalid_output(host, reason)
        }
        DevslotError::Timeout { host, seconds } => IpcError::remote_command_timeout(host, *seconds),
        DevslotError::Connect { host, source } => {
            IpcError::connection_failed(host, source.to_string())
        }
        DevslotError::ShuttingDown => IpcError::internal(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use engine::{OpenOptions, TunnelDefinition, TunnelId, TunnelKind};
    use ipc::protocol::ErrorCode;
    use pretty_assertions::assert_eq;

    use super::*;

    fn definition() -> TunnelDefinition {
        TunnelDefinition {
            id: TunnelId::generate(),
            host: "vega".to_owned(),
            kind: TunnelKind::Local,
            listen_port: 20050,
            listen_address: Some("127.0.0.1".to_owned()),
            target_host: None,
            target_port: None,
            tags: vec!["devslot".to_owned()],
            open: OpenOptions::default(),
            standby: None,
        }
    }

    fn port(status: DevslotPortStatus) -> DevslotPort {
        DevslotPort {
            var: "PORT".to_owned(),
            port: 20050,
            kind: "web".to_owned(),
            url: Some("http://a.localhost:20050".to_owned()),
            host_name: Some("a.localhost".to_owned()),
            service: None,
            target: None,
            is_forced: true,
            status,
            is_local_port_busy: true,
            suggested_local_port: Some(30050),
        }
    }

    #[test]
    fn should_give_the_tunnel_id_when_the_port_is_already_tunneled() {
        let tunnel_id = TunnelId::generate();

        let dto = port_to_dto(&port(DevslotPortStatus::AlreadyTunneled { tunnel_id }));

        assert_eq!(
            (dto.status, dto.tunnel_id, dto.forced, dto.host_name),
            (
                DevslotPortStatusDto::AlreadyTunneled,
                Some(tunnel_id.to_string()),
                true,
                Some("a.localhost".to_owned())
            )
        );
    }

    #[test]
    fn should_carry_the_busy_flag_and_the_suggestion_when_the_local_port_is_busy() {
        let dto = port_to_dto(&port(DevslotPortStatus::New));

        assert_eq!(
            (dto.local_port_busy, dto.suggested_local_port),
            (true, Some(30050))
        );
    }

    #[test]
    fn should_give_no_tunnel_id_when_the_port_is_new_or_ambiguous() {
        assert_eq!(
            (
                port_to_dto(&port(DevslotPortStatus::New)),
                port_to_dto(&port(DevslotPortStatus::Ambiguous))
            )
                .0
                .tunnel_id,
            None
        );
    }

    #[test]
    fn should_map_a_stale_tunnel_with_its_reason_and_action() {
        let stale = DevslotStaleTunnel {
            definition: definition(),
            is_temporary: false,
            reason: DevslotStaleReason::Reassigned,
            action: DevslotStaleAction::Ask,
        };

        let dto = stale_to_dto(&stale);

        assert_eq!(
            (dto.reason, dto.action, dto.is_temporary, dto.tunnel.id),
            (
                DevslotStaleReasonDto::Reassigned,
                DevslotStaleActionDto::Ask,
                false,
                stale.definition.id.to_string()
            )
        );
    }

    #[test]
    fn should_read_a_resync_tunnel_when_its_id_matches_its_definition() {
        let definition = definition();
        let dto = DevslotResyncTunnelDto {
            id: definition.id.to_string(),
            definition: definition_to_dto(&definition),
            is_temporary: true,
        };

        let tunnel = resync_tunnel_from_dto(dto).unwrap();

        assert_eq!((tunnel.definition, tunnel.is_temporary), (definition, true));
    }

    #[test]
    fn should_refuse_a_resync_tunnel_when_its_id_differs_from_its_definition() {
        let definition = definition();
        let dto = DevslotResyncTunnelDto {
            id: TunnelId::generate().to_string(),
            definition: definition_to_dto(&definition),
            is_temporary: true,
        };

        assert_eq!(
            resync_tunnel_from_dto(dto).unwrap_err().code,
            ErrorCode::InvalidRequest
        );
    }

    #[test]
    fn should_refuse_a_resync_tunnel_when_its_id_is_not_a_ulid() {
        let mut dto = DevslotResyncTunnelDto {
            id: "nope".to_owned(),
            definition: definition_to_dto(&definition()),
            is_temporary: false,
        };
        dto.definition.id = "nope".to_owned();

        assert_eq!(
            resync_tunnel_from_dto(dto).unwrap_err().code,
            ErrorCode::InvalidRequest
        );
    }

    #[test]
    fn should_map_each_devslot_error_to_its_code() {
        let errors = [
            DevslotError::NotFound {
                host: "vega".to_owned(),
                path: "ds".to_owned(),
            },
            DevslotError::Failed {
                host: "vega".to_owned(),
                status: Some(2),
                stderr: "boom".to_owned(),
            },
            DevslotError::InvalidOutput {
                host: "vega".to_owned(),
                reason: "bad".to_owned(),
            },
            DevslotError::Timeout {
                host: "vega".to_owned(),
                seconds: 20,
            },
            DevslotError::ShuttingDown,
            DevslotError::Connect {
                host: "vega".to_owned(),
                source: std::sync::Arc::new(engine::ConnectError::Authentication {
                    tried: Vec::new(),
                }),
            },
        ];

        assert_eq!(
            errors
                .each_ref()
                .map(|error| devslot_error_to_ipc(error).code),
            [
                ErrorCode::DevslotNotFound,
                ErrorCode::DevslotFailed,
                ErrorCode::DevslotInvalidOutput,
                ErrorCode::RemoteCommandTimeout,
                ErrorCode::Internal,
                ErrorCode::ConnectionFailed
            ]
        );
    }

    fn up_with(removed: Vec<DevslotRemovedRoute>) -> DevslotUp {
        DevslotUp {
            host: "vega".to_owned(),
            instance: "web-shop-098a".to_owned(),
            started: vec![DevslotUpPort {
                remote_port: 20050,
                listen_port: 30050,
                tunnel_id: TunnelId::generate(),
            }],
            existing: Vec::new(),
            skipped: vec![
                DevslotSkippedPort {
                    remote_port: 20051,
                    reason: DevslotSkipReason::LocalPortBusy,
                },
                DevslotSkippedPort {
                    remote_port: 20052,
                    reason: DevslotSkipReason::NeedsReview,
                },
                DevslotSkippedPort {
                    remote_port: 80,
                    reason: DevslotSkipReason::PrivilegedPort,
                },
            ],
            failed: vec![DevslotFailedPort {
                remote_port: 20053,
                message: "boom".to_owned(),
            }],
            removed,
            needs_review: 2,
            app_tunnel: None,
            app_url: Some("http://web-shop-098a.localhost:20050".to_owned()),
            is_app_port_shared: true,
        }
    }

    #[test]
    fn should_map_every_part_of_the_result_when_opening_an_instance() {
        let up = up_with(vec![DevslotRemovedRoute {
            listen_port: 20999,
            remote_port: 20998,
            is_temporary: true,
        }]);

        let dto = up_to_payload(&up, OpenOutcome::NotAttempted).unwrap();

        assert_eq!(
            (
                dto.started[0].tunnel_id.clone(),
                dto.skipped
                    .iter()
                    .map(|skipped| skipped.reason)
                    .collect::<Vec<_>>(),
                dto.failed[0].message.clone(),
                dto.removed,
                dto.needs_review,
                dto.app_port_shared
            ),
            (
                up.started[0].tunnel_id.to_string(),
                vec![
                    DevslotSkipReasonDto::LocalPortBusy,
                    DevslotSkipReasonDto::NeedsReview,
                    DevslotSkipReasonDto::PrivilegedPort
                ],
                "boom".to_owned(),
                vec![DevslotRemovedRouteDto {
                    listen_port: 20999,
                    remote_port: 20998,
                    is_temporary: true
                }],
                2,
                true
            )
        );
    }

    #[test]
    fn should_carry_the_opened_address_when_the_browser_was_launched() {
        let dto = up_to_payload(
            &up_with(Vec::new()),
            OpenOutcome::Opened("http://a.localhost:30050/".to_owned()),
        )
        .unwrap();

        assert_eq!(
            (dto.opened, dto.open_error),
            (Some("http://a.localhost:30050/".to_owned()), None)
        );
    }

    #[test]
    fn should_carry_the_open_error_when_the_browser_could_not_be_launched() {
        let dto = up_to_payload(
            &up_with(Vec::new()),
            OpenOutcome::Failed("no browser".to_owned()),
        )
        .unwrap();

        assert_eq!(
            (dto.opened, dto.open_error),
            (None, Some("no browser".to_owned()))
        );
    }

    #[test]
    fn should_refuse_the_result_when_its_encoded_line_would_exceed_the_ipc_limit() {
        let routes = vec![
            DevslotRemovedRoute {
                listen_port: 20999,
                remote_port: 20999,
                is_temporary: true,
            };
            30_000
        ];

        let error = up_to_payload(&up_with(routes), OpenOutcome::NotAttempted).unwrap_err();

        assert_eq!(error.code, ErrorCode::DevslotInvalidOutput);
    }

    #[test]
    fn should_map_an_instance_that_is_not_listed_to_its_own_code_with_both_names() {
        let error = DevslotError::InstanceNotFound {
            host: "vega".to_owned(),
            instance: "web-shop-098a".to_owned(),
        };

        let ipc_error = devslot_error_to_ipc(&error);

        assert_eq!(
            (ipc_error.code, ipc_error.data),
            (
                ErrorCode::DevslotInstanceNotFound,
                Some(serde_json::json!({"host": "vega", "instance": "web-shop-098a"}))
            )
        );
    }

    #[test]
    fn should_carry_the_status_and_the_standard_error_when_the_command_failed() {
        let error = DevslotError::Failed {
            host: "vega".to_owned(),
            status: Some(2),
            stderr: "boom".to_owned(),
        };

        assert_eq!(
            devslot_error_to_ipc(&error).data,
            Some(serde_json::json!({"host": "vega", "status": 2, "stderr": "boom"}))
        );
    }

    const MAX_TEXT: usize = 256;
    const MAX_PATH: usize = 1024;
    const REALISTIC_PORTS_PER_INSTANCE: usize = 4;

    fn quotes(length: usize) -> String {
        "\"".repeat(length)
    }

    fn adversarial_port(index: usize) -> DevslotPort {
        DevslotPort {
            var: quotes(MAX_TEXT),
            port: 20000 + u16::try_from(index).unwrap(),
            kind: quotes(MAX_TEXT),
            url: Some(quotes(MAX_TEXT)),
            host_name: Some(format!("{}.localhost", "a".repeat(MAX_TEXT - 10))),
            service: Some(quotes(MAX_TEXT)),
            target: Some(65535),
            is_forced: true,
            status: DevslotPortStatus::AlreadyTunneled {
                tunnel_id: TunnelId::generate(),
            },
            is_local_port_busy: false,
            suggested_local_port: None,
        }
    }

    fn realistic_port(index: usize) -> DevslotPort {
        let url = format!("http://web-shop-feat-auth-3f2a.localhost:{}", 20050 + index);
        DevslotPort {
            var: "VITE_HMR_PORT".to_owned(),
            port: 20050 + u16::try_from(index).unwrap(),
            kind: "web".to_owned(),
            url: Some(url),
            host_name: Some("web-shop-feat-auth-3f2a.localhost".to_owned()),
            service: None,
            target: None,
            is_forced: false,
            status: DevslotPortStatus::New,
            is_local_port_busy: false,
            suggested_local_port: None,
        }
    }

    fn instance_of(
        text: impl Fn() -> String,
        path: impl Fn() -> String,
        port: fn(usize) -> DevslotPort,
        port_count: usize,
    ) -> DevslotInstance {
        DevslotInstance {
            name: text(),
            project: Some(text()),
            project_tag: Some(format!("ds-{}", "a".repeat(61))),
            path: path(),
            is_worktree: Some(true),
            slot: u32::MAX,
            app_url: Some(text()),
            services_running: Some(true),
            ports: (0..port_count).map(port).collect(),
        }
    }

    fn discovery_of(instance: &DevslotInstance) -> DevslotDiscovery {
        DevslotDiscovery {
            host: "vega".to_owned(),
            persist_default: true,
            instances: vec![instance.clone(); engine::MAX_DEVSLOT_INSTANCES],
            stale: Vec::new(),
            warnings: Vec::new(),
        }
    }

    #[test]
    fn should_refuse_the_discovery_when_its_encoded_line_would_exceed_the_ipc_limit() {
        let instance = instance_of(
            || quotes(MAX_TEXT),
            || quotes(MAX_PATH),
            adversarial_port,
            engine::MAX_DEVSLOT_PORTS / engine::MAX_DEVSLOT_INSTANCES,
        );

        let error = discovery_to_payload(&discovery_of(&instance)).unwrap_err();

        assert_eq!(error.code, ErrorCode::DevslotInvalidOutput);
    }

    #[test]
    fn should_accept_a_realistic_large_discovery_when_it_fits_the_ipc_limit() {
        let instance = instance_of(
            || "web-shop-feat-auth-3f2a".to_owned(),
            || "/home/me/dev/web-shop.feat-auth".to_owned(),
            realistic_port,
            REALISTIC_PORTS_PER_INSTANCE,
        );

        let dto = discovery_to_payload(&discovery_of(&instance)).unwrap();

        assert_eq!(dto.instances.len(), engine::MAX_DEVSLOT_INSTANCES);
    }
}
