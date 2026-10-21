#![allow(clippy::unwrap_used, clippy::expect_used)]

use ipc::protocol::{
    AdoptResult, AvailableOpenerDto, CheckTargetResult, ClientKind, ConfigInvalidEvent,
    CredentialAnswer, CredentialRequestDto, CredentialRequestEvent, CredentialResponseParams,
    DaemonPathsResult, DefaultOpenerDto, DeleteTagResult, ErrorCode, EventMessage,
    ExternalForwardDto, ExternalProcessDto, ExternalsChangedEvent, FailureReasonDto,
    ForgetSecretsResult, Hello, HostKeyAlertReasonDto, HostKeyDecisionDto, HostKeyRequestEvent,
    HostKeyResponseParams, HostOptionDto, HostParams, HostsResult, ImportCandidateDto,
    ImportPreviewDto, ImportScriptParams, ImportWarningDto, IpcError, LogEntryDto, LogLevelDto,
    MAX_IMPORT_SCRIPT_BYTES, MAX_LINE_BYTES, METHOD_NAMES, MetricsDto, OpenHostDto, OpenOptionsDto,
    OpenSchemeDto, OpenTunnelParams, OpenTunnelResult, OpenerSourceDto, OpenerSpecDto,
    OpenersResult, OpenersUpdateParams, PROTOCOL_VERSION, PortOwnerDto, ProcessParams, PromptDto,
    PromptResolvedEvent, QuitParams, RememberDto, RenameTagResult, Request, RequestEnvelope,
    ResolvedHostDto, ResponsePayload, Secret, ServerFrame, SettingsDto, SnapshotDto, TagParams,
    TagRenameParams, TargetHealthDto, TemporaryTunnelSpecDto, TrayChangedEvent, TrayStatusDto,
    TunnelChangedEvent, TunnelDefinitionDto, TunnelDto, TunnelIdParams, TunnelKindDto,
    TunnelLogEvent, TunnelMetricsEvent, TunnelSpecDto, TunnelStateDto, UiClaimResult, X11AnchorDto,
    X11ChangedEvent, X11HostParams, X11LaunchParams, X11LauncherDto, X11StateDto, encode_event,
    encode_response,
};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};

const TUNNEL_ID: &str = "01J9ZK3Q8M5N7R2T4V6X8Y0ABC";

fn spec() -> TunnelSpecDto {
    TunnelSpecDto {
        host: "nova".to_owned(),
        kind: TunnelKindDto::Local,
        listen_port: 3333,
        listen_address: None,
        target_host: Some("db.internal".to_owned()),
        target_port: Some(5432),
        tags: vec!["dev".to_owned()],
        open: OpenOptionsDto::default(),
        standby: None,
    }
}

fn definition() -> TunnelDefinitionDto {
    TunnelDefinitionDto {
        id: TUNNEL_ID.to_owned(),
        spec: spec(),
    }
}

fn settings() -> SettingsDto {
    SettingsDto {
        default_bind_address: "127.0.0.1".to_owned(),
        default_remote_bind_address: "127.0.0.1".to_owned(),
        keepalive_interval_ms: 30_000,
        keepalive_max: 3,
        session_idle_grace_ms: 60_000,
        reconnect_initial_delay_ms: 1_000,
        reconnect_max_delay_ms: 60_000,
        restore_on_launch: true,
        start_on_launch_tags: vec!["dev".to_owned()],
        launch_at_login: false,
        use_os_keyring: false,
        x11_display: None,
        open_host: OpenHostDto::Localhost,
        standby: false,
        standby_idle_s: 300,
        standby_notify: ipc::protocol::StandbyNotifyDto::Visual,
    }
}

fn opener_spec() -> OpenerSpecDto {
    OpenerSpecDto {
        id: "dbeaver".to_owned(),
        name: "DBeaver".to_owned(),
        command: "dbeaver".to_owned(),
        args: vec!["-con".to_owned(), "host={host}".to_owned()],
        ports: vec![5432],
    }
}

fn tunnel() -> TunnelDto {
    TunnelDto {
        definition: definition(),
        state: TunnelStateDto::Reconnecting {
            attempt: 2,
            next_retry_at: 1_700_000_005_000,
        },
        health: TargetHealthDto::Unreachable {
            since: 1_700_000_000_000,
            error: "connection refused".to_owned(),
        },
        metrics: MetricsDto {
            active_connections: 1,
            total_connections: 4,
            bytes_in: 10,
            bytes_out: 20,
            last_activity: Some(1_700_000_001_000),
        },
        is_temporary: false,
        default_opener: DefaultOpenerDto {
            id: "browser".to_owned(),
            name: "Browser".to_owned(),
        },
    }
}

fn temporary_spec() -> TemporaryTunnelSpecDto {
    TemporaryTunnelSpecDto {
        host: "nova".to_owned(),
        kind: TunnelKindDto::Local,
        listen_port: 5000,
        listen_address: None,
        target_host: None,
        target_port: None,
    }
}

fn port_owner() -> PortOwnerDto {
    PortOwnerDto {
        pid: 4242,
        process: "ssh".to_owned(),
    }
}

fn external_process() -> ExternalProcessDto {
    ExternalProcessDto {
        pid: 4242,
        process: "ssh".to_owned(),
        destination: Some("nova".to_owned()),
        forwards: vec![
            ExternalForwardDto {
                kind: TunnelKindDto::Local,
                listen_address: Some("127.0.0.1".to_owned()),
                listen_port: 3333,
                target_host: Some("db.internal".to_owned()),
                target_port: Some(5432),
            },
            ExternalForwardDto {
                kind: TunnelKindDto::Dynamic,
                listen_address: None,
                listen_port: 1080,
                target_host: None,
                target_port: None,
            },
        ],
    }
}

fn x11_host() -> X11HostParams {
    X11HostParams {
        host: "nova".to_owned(),
    }
}

fn x11_launch() -> X11LaunchParams {
    X11LaunchParams {
        host: "nova".to_owned(),
        name: "xterm".to_owned(),
    }
}

fn x11_anchor() -> X11AnchorDto {
    X11AnchorDto {
        host: "nova".to_owned(),
        state: X11StateDto::Active {
            display: "localhost:10.0".to_owned(),
        },
    }
}

fn x11_launcher() -> X11LauncherDto {
    X11LauncherDto {
        host: "nova".to_owned(),
        name: "xterm".to_owned(),
        command: "xterm -ls".to_owned(),
    }
}

fn every_request() -> Vec<Request> {
    let tunnel_id = TunnelIdParams {
        id: TUNNEL_ID.to_owned(),
    };
    let tag = TagParams {
        tag: "dev".to_owned(),
    };
    vec![
        Request::Hello(Hello::new(ClientKind::Ui, "0.1.0", true, true)),
        Request::StateSnapshot {},
        Request::TunnelCreate(spec()),
        Request::TunnelUpdate(definition()),
        Request::TunnelDelete(tunnel_id.clone()),
        Request::TunnelStart(tunnel_id.clone()),
        Request::TunnelStop(tunnel_id.clone()),
        Request::TunnelRestart(tunnel_id.clone()),
        Request::TunnelLogs(tunnel_id.clone()),
        Request::TunnelStartTemporary(temporary_spec()),
        Request::TunnelPersist(tunnel_id.clone()),
        Request::TunnelCheckTarget(tunnel_id.clone()),
        Request::TunnelOpen(OpenTunnelParams {
            id: TUNNEL_ID.to_owned(),
            with: Some("remmina-vnc".to_owned()),
        }),
        Request::OpenersUpdate(OpenersUpdateParams {
            openers: vec![opener_spec()],
        }),
        Request::HostsList {},
        Request::HostsResolve(HostParams {
            host: "nova".to_owned(),
        }),
        Request::ImportSshConfig {},
        Request::ImportScript(ImportScriptParams {
            text: "ssh -L 3333:localhost:3333 nova".to_owned(),
        }),
        Request::TagStart(tag.clone()),
        Request::TagStop(tag),
        Request::SettingsUpdate(settings()),
        Request::CredentialRespond(CredentialResponseParams {
            prompt_id: "p1".to_owned(),
            answer: Some(CredentialAnswer::Single(Secret::new("hunter2".to_owned()))),
            remember: None,
        }),
        Request::HostKeyRespond(HostKeyResponseParams {
            prompt_id: "p2".to_owned(),
            decision: HostKeyDecisionDto::AcceptAndSave,
        }),
        Request::DaemonQuit(QuitParams { force: false }),
        Request::UiShow {},
        Request::ExternalAdopt(TunnelIdParams {
            id: TUNNEL_ID.to_owned(),
        }),
        Request::ExternalKill(ProcessParams { pid: 4242 }),
        Request::ExternalIgnore(ProcessParams { pid: 4242 }),
        Request::X11Start(x11_host()),
        Request::X11Stop(x11_host()),
        Request::X11Launch(x11_launch()),
        Request::SecretsForget {},
        Request::UiClaim {},
        Request::TagsRename(TagRenameParams {
            from: "dev".to_owned(),
            to: "staging".to_owned(),
        }),
        Request::TagsDelete(TagParams {
            tag: "dev".to_owned(),
        }),
        Request::DaemonPaths {},
    ]
}

fn every_event() -> Vec<EventMessage> {
    vec![
        EventMessage::TunnelChanged(TunnelChangedEvent {
            id: TUNNEL_ID.to_owned(),
            state: TunnelStateDto::Failed {
                reason: FailureReasonDto::PortInUse {
                    address: "127.0.0.1:3333".to_owned(),
                },
            },
            health: TargetHealthDto::Reachable,
        }),
        EventMessage::TunnelMetrics(TunnelMetricsEvent {
            id: TUNNEL_ID.to_owned(),
            metrics: MetricsDto::default(),
        }),
        EventMessage::TunnelLog(TunnelLogEvent {
            id: TUNNEL_ID.to_owned(),
            entry: LogEntryDto {
                at: 1_700_000_000_000,
                level: LogLevelDto::Warn,
                message: "session lost".to_owned(),
            },
        }),
        EventMessage::ConfigReloaded {},
        EventMessage::ConfigInvalid(ConfigInvalidEvent {
            messages: vec!["line 3: bad port".to_owned()],
        }),
        EventMessage::CredentialRequest(CredentialRequestEvent {
            prompt_id: "p1".to_owned(),
            request: CredentialRequestDto::Passphrase {
                alias: "nova".to_owned(),
                key_path: "/home/me/.ssh/id_ed25519".to_owned(),
            },
        }),
        EventMessage::CredentialRequest(CredentialRequestEvent {
            prompt_id: "p3".to_owned(),
            request: CredentialRequestDto::KeyboardInteractive {
                alias: "nova".to_owned(),
                name: String::new(),
                instructions: String::new(),
                prompts: vec![PromptDto {
                    text: "Code: ".to_owned(),
                    echo: true,
                }],
            },
        }),
        EventMessage::HostKeyRequest(HostKeyRequestEvent::Prompt {
            prompt_id: "p2".to_owned(),
            alias: "nova".to_owned(),
            lookup_name: "[nova]:2222".to_owned(),
            algorithm: "ssh-ed25519".to_owned(),
            fingerprint_sha256: "SHA256:abc".to_owned(),
        }),
        EventMessage::HostKeyRequest(HostKeyRequestEvent::Alert {
            alias: "nova".to_owned(),
            reason: HostKeyAlertReasonDto::Changed {
                known_fingerprint: "SHA256:old".to_owned(),
                path: "/home/me/.ssh/known_hosts".to_owned(),
                line: 12,
            },
            presented_fingerprint: "SHA256:new".to_owned(),
        }),
        EventMessage::QuitRequested {},
        EventMessage::TrayChanged(TrayChangedEvent {
            status: TrayStatusDto::MissingHost,
        }),
        EventMessage::ShowRequested {},
        EventMessage::PromptResolved(PromptResolvedEvent {
            prompt_id: "p1".to_owned(),
        }),
        EventMessage::ExternalsChanged(ExternalsChangedEvent {
            processes: vec![external_process()],
        }),
        EventMessage::X11Changed(X11ChangedEvent {
            host: "nova".to_owned(),
            state: X11StateDto::Starting,
        }),
    ]
}

fn to_json<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).unwrap()
}

#[test]
fn should_name_every_method_in_the_method_list() {
    let mut serialized: Vec<String> = every_request()
        .iter()
        .map(|request| to_json(request)["method"].as_str().unwrap().to_owned())
        .collect();
    let mut declared: Vec<String> = METHOD_NAMES.iter().map(ToString::to_string).collect();
    serialized.sort();
    declared.sort();

    assert_eq!(serialized, declared);
}

#[test]
fn should_round_trip_every_request() {
    for (index, request) in every_request().into_iter().enumerate() {
        let envelope = RequestEnvelope {
            id: u64::try_from(index).unwrap(),
            request,
        };
        let line = serde_json::to_vec(&envelope).unwrap();

        assert_eq!(RequestEnvelope::parse(&line).unwrap(), envelope);
    }
}

#[test]
fn should_round_trip_every_event() {
    for event in every_event() {
        let line = encode_event(&event).unwrap();

        assert_eq!(
            ServerFrame::parse(line.as_bytes()).unwrap(),
            ServerFrame::Event(Some(event))
        );
    }
}

#[test]
fn should_serialize_hello_as_documented() {
    let envelope = RequestEnvelope {
        id: 1,
        request: Request::Hello(Hello::new(ClientKind::Cli, "0.1.0", false, false)),
    };

    assert_eq!(
        serde_json::to_string(&envelope).unwrap(),
        r#"{"id":1,"method":"hello","params":{"protocolVersion":3,"clientKind":"cli","clientVersion":"0.1.0","subscribeEvents":false,"canPrompt":false}}"#
    );
}

#[test]
fn should_serialize_tunnel_create_as_documented() {
    let envelope = RequestEnvelope {
        id: 2,
        request: Request::TunnelCreate(spec()),
    };

    assert_eq!(
        serde_json::to_string(&envelope).unwrap(),
        r#"{"id":2,"method":"tunnel.create","params":{"host":"nova","kind":"local","listenPort":3333,"targetHost":"db.internal","targetPort":5432,"tags":["dev"]}}"#
    );
}

#[test]
fn should_serialize_the_open_options_of_a_tunnel_when_they_are_set() {
    let spec = TunnelSpecDto {
        open: OpenOptionsDto {
            scheme: Some(OpenSchemeDto::Https),
            path: Some("/admin".to_owned()),
            host: Some(OpenHostDto::Ip),
            with: Some("remmina-vnc".to_owned()),
        },
        ..spec()
    };

    assert_eq!(
        to_json(&spec)["open"],
        json!({"scheme":"https","path":"/admin","host":"ip","with":"remmina-vnc"})
    );
}

#[test]
fn should_omit_the_open_options_of_a_tunnel_when_none_is_set() {
    assert_eq!(to_json(&spec()).get("open"), None);
}

#[test]
fn should_deserialize_a_tunnel_without_open_options_as_having_none() {
    let value = to_json(&spec());

    let parsed: TunnelSpecDto = serde_json::from_value(value).unwrap();

    assert!(parsed.open.is_empty());
}

#[test]
fn should_deserialize_settings_without_open_host_as_localhost() {
    let mut value = to_json(&settings());
    value.as_object_mut().unwrap().remove("openHost");

    let parsed: SettingsDto = serde_json::from_value(value).unwrap();

    assert_eq!(parsed.open_host, OpenHostDto::Localhost);
}

#[test]
fn should_serialize_tunnel_open_as_documented() {
    let envelope = RequestEnvelope {
        id: 5,
        request: Request::TunnelOpen(OpenTunnelParams {
            id: TUNNEL_ID.to_owned(),
            with: None,
        }),
    };

    assert_eq!(
        serde_json::to_string(&envelope).unwrap(),
        r#"{"id":5,"method":"tunnel.open","params":{"id":"01J9ZK3Q8M5N7R2T4V6X8Y0ABC"}}"#
    );
}

#[test]
fn should_serialize_an_open_tunnel_response_as_documented() {
    let result = OpenTunnelResult {
        target: "http://localhost:3333/".to_owned(),
    };

    let line = encode_response(Some(6), &Ok(ResponsePayload::OpenTunnel(result))).unwrap();

    assert_eq!(
        line,
        "{\"id\":6,\"result\":{\"target\":\"http://localhost:3333/\"}}\n"
    );
}

#[test]
fn should_serialize_tunnel_open_with_an_opener_as_documented() {
    let envelope = RequestEnvelope {
        id: 5,
        request: Request::TunnelOpen(OpenTunnelParams {
            id: TUNNEL_ID.to_owned(),
            with: Some("remmina-vnc".to_owned()),
        }),
    };

    assert_eq!(
        serde_json::to_string(&envelope).unwrap(),
        r#"{"id":5,"method":"tunnel.open","params":{"id":"01J9ZK3Q8M5N7R2T4V6X8Y0ABC","with":"remmina-vnc"}}"#
    );
}

#[test]
fn should_serialize_openers_update_as_documented() {
    let envelope = RequestEnvelope {
        id: 8,
        request: Request::OpenersUpdate(OpenersUpdateParams {
            openers: vec![opener_spec()],
        }),
    };

    assert_eq!(
        serde_json::to_string(&envelope).unwrap(),
        r#"{"id":8,"method":"openers.update","params":{"openers":[{"id":"dbeaver","name":"DBeaver","command":"dbeaver","args":["-con","host={host}"],"ports":[5432]}]}}"#
    );
}

#[test]
fn should_serialize_an_openers_response_with_the_source_of_each_opener() {
    let result = OpenersResult {
        openers: vec![AvailableOpenerDto {
            spec: opener_spec(),
            source: OpenerSourceDto::Detected,
        }],
    };

    let line = encode_response(Some(9), &Ok(ResponsePayload::Openers(result))).unwrap();

    assert_eq!(
        line,
        "{\"id\":9,\"result\":{\"openers\":[{\"id\":\"dbeaver\",\"name\":\"DBeaver\",\"command\":\"dbeaver\",\"args\":[\"-con\",\"host={host}\"],\"ports\":[5432],\"source\":\"detected\"}]}}\n"
    );
}

#[test]
fn should_deserialize_an_opener_without_arguments_or_ports() {
    let parsed: OpenerSpecDto =
        serde_json::from_value(json!({"id":"a","name":"A","command":"a"})).unwrap();

    assert_eq!((parsed.args, parsed.ports), (Vec::new(), Vec::new()));
}

#[test]
fn should_serialize_the_unknown_opener_error_with_the_available_ids() {
    let error = IpcError::unknown_opener("no opener", &["browser".to_owned(), "a".to_owned()]);

    let line = encode_response(Some(7), &Err(error)).unwrap();

    assert_eq!(
        line,
        "{\"id\":7,\"error\":{\"code\":\"unknown_opener\",\"message\":\"no opener\",\"data\":[\"browser\",\"a\"]}}\n"
    );
}

#[test]
fn should_serialize_the_open_failed_error_code_as_documented() {
    let line = encode_response(Some(7), &Err(IpcError::open_failed("no browser"))).unwrap();

    assert_eq!(
        line,
        "{\"id\":7,\"error\":{\"code\":\"open_failed\",\"message\":\"no browser\"}}\n"
    );
}

#[test]
fn should_serialize_tunnel_start_temporary_as_documented() {
    let envelope = RequestEnvelope {
        id: 3,
        request: Request::TunnelStartTemporary(temporary_spec()),
    };

    assert_eq!(
        serde_json::to_string(&envelope).unwrap(),
        r#"{"id":3,"method":"tunnel.start_temporary","params":{"host":"nova","kind":"local","listenPort":5000}}"#
    );
}

#[test]
fn should_serialize_tunnel_persist_as_documented() {
    let envelope = RequestEnvelope {
        id: 5,
        request: Request::TunnelPersist(TunnelIdParams {
            id: TUNNEL_ID.to_owned(),
        }),
    };

    assert_eq!(
        serde_json::to_string(&envelope).unwrap(),
        r#"{"id":5,"method":"tunnel.persist","params":{"id":"01J9ZK3Q8M5N7R2T4V6X8Y0ABC"}}"#
    );
}

#[test]
fn should_serialize_tunnel_check_target_as_documented() {
    let envelope = RequestEnvelope {
        id: 5,
        request: Request::TunnelCheckTarget(TunnelIdParams {
            id: TUNNEL_ID.to_owned(),
        }),
    };

    assert_eq!(
        serde_json::to_string(&envelope).unwrap(),
        r#"{"id":5,"method":"tunnel.check_target","params":{"id":"01J9ZK3Q8M5N7R2T4V6X8Y0ABC"}}"#
    );
}

#[test]
fn should_serialize_a_check_target_response_as_documented() {
    let result = CheckTargetResult {
        health: TargetHealthDto::Unreachable {
            since: 1_700_000_000_000,
            error: "connection refused".to_owned(),
        },
    };

    let line = encode_response(Some(6), &Ok(ResponsePayload::CheckTarget(result))).unwrap();

    assert_eq!(
        line,
        "{\"id\":6,\"result\":{\"health\":{\"kind\":\"unreachable\",\"since\":1700000000000,\"error\":\"connection refused\"}}}\n"
    );
}

#[test]
fn should_serialize_a_temporary_tunnel_with_its_flag() {
    let temporary = TunnelDto {
        is_temporary: true,
        ..tunnel()
    };

    assert_eq!(to_json(&temporary)["isTemporary"], json!(true));
}

#[test]
fn should_deserialize_a_tunnel_without_the_temporary_flag_as_saved() {
    let mut value = to_json(&tunnel());
    value.as_object_mut().unwrap().remove("isTemporary");

    let parsed: TunnelDto = serde_json::from_value(value).unwrap();

    assert!(!parsed.is_temporary);
}

#[test]
fn should_serialize_hosts_list_as_documented() {
    let envelope = RequestEnvelope {
        id: 6,
        request: Request::HostsList {},
    };

    assert_eq!(
        serde_json::to_string(&envelope).unwrap(),
        r#"{"id":6,"method":"hosts.list","params":{}}"#
    );
}

#[test]
fn should_deserialize_hosts_list_when_params_are_omitted() {
    let line = br#"{"id":6,"method":"hosts.list"}"#;

    assert_eq!(
        RequestEnvelope::parse(line).unwrap().request,
        Request::HostsList {}
    );
}

#[test]
fn should_serialize_a_hosts_response_as_documented() {
    let hosts = HostsResult {
        hosts: vec!["nova".to_owned(), "db".to_owned()],
    };

    let line = encode_response(Some(7), &Ok(ResponsePayload::Hosts(hosts))).unwrap();

    assert_eq!(
        line,
        "{\"id\":7,\"result\":{\"hosts\":[\"nova\",\"db\"]}}\n"
    );
}

fn import_preview() -> ImportPreviewDto {
    ImportPreviewDto {
        candidates: vec![
            ImportCandidateDto {
                host: Some("nova".to_owned()),
                kind: TunnelKindDto::Local,
                listen_port: 3333,
                listen_address: Some("127.0.0.1".to_owned()),
                target_host: Some("127.0.0.1".to_owned()),
                target_port: Some(3333),
                comment: Some("adonis".to_owned()),
                line: Some(4),
            },
            ImportCandidateDto {
                host: None,
                kind: TunnelKindDto::Dynamic,
                listen_port: 1080,
                listen_address: None,
                target_host: None,
                target_port: None,
                comment: None,
                line: None,
            },
        ],
        warnings: vec![
            ImportWarningDto::MalformedForward {
                line: 7,
                text: "nonsense".to_owned(),
            },
            ImportWarningDto::UnsupportedForward {
                line: None,
                host: Some("sock".to_owned()),
                text: "8080 /run/app.sock".to_owned(),
            },
            ImportWarningDto::UnsupportedForward {
                line: Some(2),
                host: None,
                text: "9000".to_owned(),
            },
            ImportWarningDto::UnknownDestination {
                destination: "jules@198.51.100.157".to_owned(),
            },
            ImportWarningDto::HostResolutionFailed {
                host: "broken".to_owned(),
                message: "bad port".to_owned(),
            },
        ],
    }
}

#[test]
fn should_serialize_hosts_resolve_as_documented() {
    let envelope = RequestEnvelope {
        id: 8,
        request: Request::HostsResolve(HostParams {
            host: "nova".to_owned(),
        }),
    };

    assert_eq!(
        serde_json::to_string(&envelope).unwrap(),
        r#"{"id":8,"method":"hosts.resolve","params":{"host":"nova"}}"#
    );
}

#[test]
fn should_serialize_a_resolved_host_response_as_documented() {
    let resolved = ResolvedHostDto {
        host: "nova".to_owned(),
        options: vec![HostOptionDto {
            key: "hostname".to_owned(),
            value: "198.51.100.157".to_owned(),
        }],
        warnings: vec!["unsupported criterion".to_owned()],
    };

    let line = encode_response(Some(8), &Ok(ResponsePayload::ResolvedHost(resolved))).unwrap();

    assert_eq!(
        line,
        "{\"id\":8,\"result\":{\"host\":\"nova\",\"options\":[{\"key\":\"hostname\",\"value\":\"198.51.100.157\"}],\"warnings\":[\"unsupported criterion\"]}}\n"
    );
}

#[test]
fn should_serialize_import_ssh_config_as_documented() {
    let envelope = RequestEnvelope {
        id: 9,
        request: Request::ImportSshConfig {},
    };

    assert_eq!(
        serde_json::to_string(&envelope).unwrap(),
        r#"{"id":9,"method":"import.ssh_config","params":{}}"#
    );
}

#[test]
fn should_serialize_import_script_as_documented() {
    let envelope = RequestEnvelope {
        id: 10,
        request: Request::ImportScript(ImportScriptParams {
            text: "3333 5432".to_owned(),
        }),
    };

    assert_eq!(
        serde_json::to_string(&envelope).unwrap(),
        r#"{"id":10,"method":"import.script","params":{"text":"3333 5432"}}"#
    );
}

#[test]
fn should_reject_import_script_when_text_exceeds_the_limit() {
    let envelope = RequestEnvelope {
        id: 11,
        request: Request::ImportScript(ImportScriptParams {
            text: "x".repeat(MAX_IMPORT_SCRIPT_BYTES + 1),
        }),
    };
    let line = serde_json::to_vec(&envelope).unwrap();

    let rejection = RequestEnvelope::parse(&line).unwrap_err();

    assert_eq!(
        (rejection.id, rejection.error.code),
        (Some(11), ErrorCode::InvalidRequest)
    );
}

#[test]
fn should_serialize_an_import_preview_response_as_documented() {
    let line = encode_response(
        Some(10),
        &Ok(ResponsePayload::ImportPreview(import_preview())),
    )
    .unwrap();

    let expected = json!({
        "id": 10,
        "result": {
            "candidates": [
                {
                    "host": "nova",
                    "kind": "local",
                    "listenPort": 3333,
                    "listenAddress": "127.0.0.1",
                    "targetHost": "127.0.0.1",
                    "targetPort": 3333,
                    "comment": "adonis",
                    "line": 4
                },
                { "kind": "dynamic", "listenPort": 1080 }
            ],
            "warnings": [
                { "kind": "malformedForward", "line": 7, "text": "nonsense" },
                { "kind": "unsupportedForward", "host": "sock", "text": "8080 /run/app.sock" },
                { "kind": "unsupportedForward", "line": 2, "text": "9000" },
                { "kind": "unknownDestination", "destination": "jules@198.51.100.157" },
                { "kind": "hostResolutionFailed", "host": "broken", "message": "bad port" }
            ]
        }
    });
    assert_eq!(serde_json::from_str::<Value>(&line).unwrap(), expected);
}

#[test]
fn should_round_trip_an_import_preview_through_json() {
    let preview = import_preview();

    let text = serde_json::to_string(&preview).unwrap();

    assert_eq!(
        serde_json::from_str::<ImportPreviewDto>(&text).unwrap(),
        preview
    );
}

#[test]
fn should_serialize_a_credential_answer_as_a_plain_string() {
    let envelope = RequestEnvelope {
        id: 3,
        request: Request::CredentialRespond(CredentialResponseParams {
            prompt_id: "p1".to_owned(),
            answer: Some(CredentialAnswer::Single(Secret::new("hunter2".to_owned()))),
            remember: None,
        }),
    };

    assert_eq!(
        serde_json::to_string(&envelope).unwrap(),
        r#"{"id":3,"method":"credential.respond","params":{"promptId":"p1","answer":"hunter2"}}"#
    );
}

fn credential_response_line(remember_member: &str) -> String {
    format!(
        r#"{{"id":3,"method":"credential.respond","params":{{"promptId":"p1","answer":"hunter2"{remember_member}}}}}"#
    )
}

fn remembered_as(remember_member: &str) -> Option<RememberDto> {
    let line = credential_response_line(remember_member);
    let Request::CredentialRespond(params) =
        RequestEnvelope::parse(line.as_bytes()).unwrap().request
    else {
        panic!("not a credential response");
    };
    params.remember
}

#[test]
fn should_serialize_the_remember_choice_when_a_passphrase_is_kept() {
    let envelope = RequestEnvelope {
        id: 3,
        request: Request::CredentialRespond(CredentialResponseParams {
            prompt_id: "p1".to_owned(),
            answer: Some(CredentialAnswer::Single(Secret::new("hunter2".to_owned()))),
            remember: Some(RememberDto::Session),
        }),
    };

    assert_eq!(
        serde_json::to_string(&envelope).unwrap(),
        credential_response_line(r#","remember":"session""#)
    );
}

#[test]
fn should_deserialize_the_session_remember_choice() {
    assert_eq!(
        remembered_as(r#","remember":"session""#),
        Some(RememberDto::Session)
    );
}

#[test]
fn should_deserialize_the_keyring_remember_choice() {
    assert_eq!(
        remembered_as(r#","remember":"keyring""#),
        Some(RememberDto::Keyring)
    );
}

#[test]
fn should_not_remember_when_the_remember_choice_is_absent() {
    assert_eq!(remembered_as(""), None);
}

#[test]
fn should_not_remember_when_the_remember_choice_is_null() {
    assert_eq!(remembered_as(r#","remember":null"#), None);
}

#[test]
fn should_reject_an_unknown_remember_choice() {
    let line = credential_response_line(r#","remember":"forever""#);

    let rejection = RequestEnvelope::parse(line.as_bytes()).unwrap_err();

    assert_eq!(rejection.error.code, ErrorCode::InvalidRequest);
}

#[test]
fn should_serialize_the_secrets_forget_request_as_documented() {
    assert_eq!(
        to_json(&Request::SecretsForget {}),
        json!({"method":"secrets.forget","params":{}})
    );
}

#[test]
fn should_accept_secrets_forget_without_params() {
    let envelope = RequestEnvelope::parse(br#"{"id":5,"method":"secrets.forget"}"#).unwrap();

    assert_eq!(envelope.request, Request::SecretsForget {});
}

#[test]
fn should_serialize_a_forget_secrets_response_as_documented() {
    let result = ForgetSecretsResult { forgotten_count: 2 };

    let line = encode_response(Some(6), &Ok(ResponsePayload::ForgetSecrets(result))).unwrap();

    assert_eq!(line, "{\"id\":6,\"result\":{\"forgottenCount\":2}}\n");
}

#[test]
fn should_serialize_the_ui_claim_request_as_documented() {
    assert_eq!(
        to_json(&Request::UiClaim {}),
        json!({"method":"ui.claim","params":{}})
    );
}

#[test]
fn should_accept_ui_claim_without_params() {
    let envelope = RequestEnvelope::parse(br#"{"id":5,"method":"ui.claim"}"#).unwrap();

    assert_eq!(envelope.request, Request::UiClaim {});
}

#[test]
fn should_serialize_a_ui_claim_response_as_documented() {
    let result = UiClaimResult { claimed: false };

    let line = encode_response(Some(6), &Ok(ResponsePayload::UiClaim(result))).unwrap();

    assert_eq!(line, "{\"id\":6,\"result\":{\"claimed\":false}}\n");
}

#[test]
fn should_serialize_the_tags_rename_request_as_documented() {
    let request = Request::TagsRename(TagRenameParams {
        from: "dev".to_owned(),
        to: "staging".to_owned(),
    });

    assert_eq!(
        to_json(&request),
        json!({"method":"tags.rename","params":{"from":"dev","to":"staging"}})
    );
}

#[test]
fn should_reject_a_tags_rename_request_without_a_new_name() {
    let line = br#"{"id":5,"method":"tags.rename","params":{"from":"dev"}}"#;

    let rejection = RequestEnvelope::parse(line).unwrap_err();

    assert_eq!(rejection.error.code, ErrorCode::InvalidRequest);
}

#[test]
fn should_serialize_a_tags_rename_response_as_documented() {
    let result = RenameTagResult { renamed_count: 2 };

    let line = encode_response(Some(6), &Ok(ResponsePayload::RenameTag(result))).unwrap();

    assert_eq!(line, "{\"id\":6,\"result\":{\"renamedCount\":2}}\n");
}

#[test]
fn should_serialize_the_tags_delete_request_as_documented() {
    let request = Request::TagsDelete(TagParams {
        tag: "dev".to_owned(),
    });

    assert_eq!(
        to_json(&request),
        json!({"method":"tags.delete","params":{"tag":"dev"}})
    );
}

#[test]
fn should_serialize_a_tags_delete_response_as_documented() {
    let result = DeleteTagResult { removed_count: 3 };

    let line = encode_response(Some(7), &Ok(ResponsePayload::DeleteTag(result))).unwrap();

    assert_eq!(line, "{\"id\":7,\"result\":{\"removedCount\":3}}\n");
}

#[test]
fn should_serialize_the_daemon_paths_request_as_documented() {
    assert_eq!(
        to_json(&Request::DaemonPaths {}),
        json!({"method":"daemon.paths","params":{}})
    );
}

#[test]
fn should_accept_daemon_paths_without_params() {
    let envelope = RequestEnvelope::parse(br#"{"id":5,"method":"daemon.paths"}"#).unwrap();

    assert_eq!(envelope.request, Request::DaemonPaths {});
}

#[test]
fn should_serialize_a_daemon_paths_response_as_documented() {
    let result = DaemonPathsResult {
        config_file: "/home/me/.config/tunneler/tunnels.toml".to_owned(),
    };

    let line = encode_response(Some(6), &Ok(ResponsePayload::DaemonPaths(result))).unwrap();

    assert_eq!(
        line,
        "{\"id\":6,\"result\":{\"configFile\":\"/home/me/.config/tunneler/tunnels.toml\"}}\n"
    );
}

#[test]
fn should_deserialize_a_cancelled_credential_answer_as_none() {
    let line =
        br#"{"id":3,"method":"credential.respond","params":{"promptId":"p1","answer":null}}"#;

    assert_eq!(
        RequestEnvelope::parse(line).unwrap().request,
        Request::CredentialRespond(CredentialResponseParams {
            prompt_id: "p1".to_owned(),
            answer: None,
            remember: None,
        })
    );
}

#[test]
fn should_deserialize_keyboard_interactive_answers_as_a_list() {
    let line =
        br#"{"id":3,"method":"credential.respond","params":{"promptId":"p1","answer":["a","b"]}}"#;

    let Request::CredentialRespond(params) = RequestEnvelope::parse(line).unwrap().request else {
        panic!("not a credential response");
    };

    assert_eq!(
        params.answer,
        Some(CredentialAnswer::Multiple(vec![
            Secret::new("a".to_owned()),
            Secret::new("b".to_owned())
        ]))
    );
}

#[test]
fn should_not_leak_a_secret_when_a_request_is_formatted_with_debug() {
    let request = Request::CredentialRespond(CredentialResponseParams {
        prompt_id: "p1".to_owned(),
        answer: Some(CredentialAnswer::Single(Secret::new("hunter2".to_owned()))),
        remember: None,
    });

    assert!(!format!("{request:?}").contains("hunter2"));
}

#[test]
fn should_serialize_a_snapshot_response_as_documented() {
    let snapshot = SnapshotDto {
        tunnels: vec![tunnel()],
        settings: settings(),
        tray: TrayStatusDto::Active,
        externals: Vec::new(),
        x11_anchors: Vec::new(),
        x11_launchers: Vec::new(),
        openers: Vec::new(),
    };

    let line = encode_response(Some(4), &Ok(ResponsePayload::Snapshot(snapshot))).unwrap();

    assert_eq!(
        line,
        concat!(
            r#"{"id":4,"result":{"tunnels":[{"id":"01J9ZK3Q8M5N7R2T4V6X8Y0ABC","host":"nova","kind":"local","listenPort":3333,"targetHost":"db.internal","targetPort":5432,"tags":["dev"],"#,
            r#""state":{"kind":"reconnecting","attempt":2,"nextRetryAt":1700000005000},"#,
            r#""health":{"kind":"unreachable","since":1700000000000,"error":"connection refused"},"#,
            r#""metrics":{"activeConnections":1,"totalConnections":4,"bytesIn":10,"bytesOut":20,"lastActivity":1700000001000},"isTemporary":false,"defaultOpener":{"id":"browser","name":"Browser"}}],"#,
            r#""settings":{"defaultBindAddress":"127.0.0.1","defaultRemoteBindAddress":"127.0.0.1","keepaliveIntervalMs":30000,"keepaliveMax":3,"sessionIdleGraceMs":60000,"reconnectInitialDelayMs":1000,"reconnectMaxDelayMs":60000,"restoreOnLaunch":true,"startOnLaunchTags":["dev"],"launchAtLogin":false,"useOsKeyring":false,"openHost":"localhost","standby":false,"standbyIdleS":300,"standbyNotify":"visual"},"tray":{"kind":"active"},"externals":[],"x11Anchors":[],"x11Launchers":[],"openers":[]}}"#,
            "\n"
        )
    );
}

#[test]
fn should_serialize_every_tray_status_with_a_kind_tag() {
    assert_eq!(
        [
            to_json(&TrayStatusDto::Active),
            to_json(&TrayStatusDto::MissingHost),
            to_json(&TrayStatusDto::Failed {
                message: "no icon".to_owned()
            }),
            to_json(&TrayStatusDto::Disabled),
        ],
        [
            json!({"kind":"active"}),
            json!({"kind":"missingHost"}),
            json!({"kind":"failed","message":"no icon"}),
            json!({"kind":"disabled"}),
        ]
    );
}

#[test]
fn should_deserialize_a_snapshot_without_tray_as_disabled() {
    let mut snapshot = to_json(&SnapshotDto {
        tunnels: Vec::new(),
        settings: settings(),
        tray: TrayStatusDto::Active,
        externals: Vec::new(),
        x11_anchors: Vec::new(),
        x11_launchers: Vec::new(),
        openers: Vec::new(),
    });
    snapshot.as_object_mut().unwrap().remove("tray");

    let parsed: SnapshotDto = serde_json::from_value(snapshot).unwrap();

    assert_eq!(parsed.tray, TrayStatusDto::Disabled);
}

#[test]
fn should_serialize_a_tray_changed_event_as_documented() {
    let event = EventMessage::TrayChanged(TrayChangedEvent {
        status: TrayStatusDto::MissingHost,
    });

    assert_eq!(
        encode_event(&event).unwrap(),
        concat!(
            r#"{"event":"tray.changed","data":{"status":{"kind":"missingHost"}}}"#,
            "\n"
        )
    );
}

#[test]
fn should_serialize_a_show_requested_event_with_an_empty_data_object() {
    assert_eq!(
        encode_event(&EventMessage::ShowRequested {}).unwrap(),
        "{\"event\":\"show.requested\",\"data\":{}}\n"
    );
}

#[test]
fn should_serialize_an_error_response_with_its_data() {
    let line = encode_response(Some(5), &Err(IpcError::tunnels_running(2))).unwrap();

    assert_eq!(
        line,
        "{\"id\":5,\"error\":{\"code\":\"tunnels_running\",\"message\":\"2 tunnels are still running\",\"data\":2}}\n"
    );
}

#[test]
fn should_serialize_a_protocol_mismatch_with_both_versions() {
    let line = encode_response(
        Some(1),
        &Err(IpcError::protocol_mismatch(PROTOCOL_VERSION, 9)),
    )
    .unwrap();

    assert_eq!(
        line,
        concat!(
            r#"{"id":1,"error":{"code":"protocol_mismatch","message":"the daemon speaks protocol 3, the client protocol 9","data":{"client":9,"daemon":3}}}"#,
            "\n"
        )
    );
}

#[test]
fn should_serialize_tunnel_changed_as_documented() {
    let event = EventMessage::TunnelChanged(TunnelChangedEvent {
        id: TUNNEL_ID.to_owned(),
        state: TunnelStateDto::Reconnecting {
            attempt: 2,
            next_retry_at: 1_700_000_005_000,
        },
        health: TargetHealthDto::Unknown,
    });

    assert_eq!(
        encode_event(&event).unwrap(),
        concat!(
            r#"{"event":"tunnel.changed","data":{"id":"01J9ZK3Q8M5N7R2T4V6X8Y0ABC","state":{"kind":"reconnecting","attempt":2,"nextRetryAt":1700000005000},"health":{"kind":"unknown"}}}"#,
            "\n"
        )
    );
}

#[test]
fn should_serialize_a_credential_request_as_one_flat_object() {
    let event = EventMessage::CredentialRequest(CredentialRequestEvent {
        prompt_id: "p1".to_owned(),
        request: CredentialRequestDto::Password {
            alias: "nova".to_owned(),
            user: "me".to_owned(),
            host: "nova.example.org".to_owned(),
        },
    });

    assert_eq!(
        encode_event(&event).unwrap(),
        concat!(
            r#"{"event":"credential.request","data":{"promptId":"p1","kind":"password","alias":"nova","user":"me","host":"nova.example.org"}}"#,
            "\n"
        )
    );
}

#[test]
fn should_serialize_a_host_key_alert_without_a_prompt_id() {
    let event = EventMessage::HostKeyRequest(HostKeyRequestEvent::Alert {
        alias: "nova".to_owned(),
        reason: HostKeyAlertReasonDto::Revoked {
            path: "/k".to_owned(),
            line: 3,
        },
        presented_fingerprint: "SHA256:new".to_owned(),
    });

    assert_eq!(
        encode_event(&event).unwrap(),
        concat!(
            r#"{"event":"hostkey.request","data":{"kind":"alert","alias":"nova","reason":{"kind":"revoked","path":"/k","line":3},"presentedFingerprint":"SHA256:new"}}"#,
            "\n"
        )
    );
}

#[test]
fn should_serialize_an_event_without_fields_with_an_empty_data_object() {
    assert_eq!(
        encode_event(&EventMessage::QuitRequested {}).unwrap(),
        "{\"event\":\"quit.requested\",\"data\":{}}\n"
    );
}

#[test]
fn should_serialize_a_failure_reason_with_camel_case_fields() {
    let state = TunnelStateDto::Failed {
        reason: FailureReasonDto::RemoteForwardRefused {
            address: "localhost:8080".to_owned(),
        },
    };

    assert_eq!(
        to_json(&state),
        json!({"kind":"failed","reason":{"kind":"remoteForwardRefused","address":"localhost:8080"}})
    );
}

#[test]
fn should_serialize_an_unresponsive_agent_failure_with_its_timeout_in_milliseconds() {
    let state = TunnelStateDto::Failed {
        reason: FailureReasonDto::AgentUnresponsive { timeout_ms: 30_000 },
    };

    assert_eq!(
        to_json(&state),
        json!({"kind":"failed","reason":{"kind":"agentUnresponsive","timeoutMs":30000}})
    );
}

#[test]
fn should_serialize_a_standby_state_without_a_failure_as_a_bare_kind() {
    let state = TunnelStateDto::Standby { last_failure: None };

    assert_eq!(to_json(&state), json!({"kind":"standby"}));
}

#[test]
fn should_serialize_a_standby_state_with_its_last_failure() {
    let state = TunnelStateDto::Standby {
        last_failure: Some(FailureReasonDto::Connect {
            message: "authentication failed".to_owned(),
        }),
    };

    assert_eq!(
        to_json(&state),
        json!({"kind":"standby","lastFailure":{"kind":"connect","message":"authentication failed"}})
    );
}

#[test]
fn should_deserialize_settings_without_standby_as_off_with_five_minutes() {
    let mut value = to_json(&settings());
    let fields = value.as_object_mut().unwrap();
    fields.remove("standby");
    fields.remove("standbyIdleS");

    let parsed: SettingsDto = serde_json::from_value(value).unwrap();

    assert_eq!((parsed.standby, parsed.standby_idle_s), (false, 300));
}

#[test]
fn should_deserialize_settings_without_standby_notify_as_visual() {
    let mut value = to_json(&settings());
    value.as_object_mut().unwrap().remove("standbyNotify");

    let parsed: SettingsDto = serde_json::from_value(value).unwrap();

    assert_eq!(
        parsed.standby_notify,
        ipc::protocol::StandbyNotifyDto::Visual
    );
}

#[test]
fn should_serialize_each_standby_notify_mode_in_camel_case() {
    let modes = [
        ipc::protocol::StandbyNotifyDto::None,
        ipc::protocol::StandbyNotifyDto::Visual,
        ipc::protocol::StandbyNotifyDto::Sound,
        ipc::protocol::StandbyNotifyDto::Both,
    ];

    assert_eq!(
        modes.map(|mode| to_json(&mode)),
        [
            json!("none"),
            json!("visual"),
            json!("sound"),
            json!("both")
        ]
    );
}

#[test]
fn should_serialize_the_standby_override_of_a_tunnel_only_when_it_is_set() {
    let spec = TunnelSpecDto {
        standby: Some(true),
        ..spec()
    };

    assert_eq!(to_json(&spec)["standby"], json!(true));
}

#[test]
fn should_deserialize_a_tunnel_spec_without_standby_as_inheriting() {
    let value = to_json(&spec());

    let parsed: TunnelSpecDto = serde_json::from_value(value).unwrap();

    assert_eq!(parsed.standby, None);
}

#[test]
fn should_limit_lines_to_one_mebibyte() {
    assert_eq!(MAX_LINE_BYTES, 1_048_576);
}

#[test]
fn should_serialize_the_external_state_with_its_owner() {
    assert_eq!(
        to_json(&TunnelStateDto::External {
            owner: port_owner()
        }),
        json!({"kind":"external","owner":{"pid":4242,"process":"ssh"}})
    );
}

#[test]
fn should_serialize_a_conflict_with_its_owner() {
    assert_eq!(
        to_json(&TunnelStateDto::Conflict {
            owner: Some(port_owner())
        }),
        json!({"kind":"conflict","owner":{"pid":4242,"process":"ssh"}})
    );
}

#[test]
fn should_serialize_a_conflict_without_an_owner_when_it_is_unknown() {
    assert_eq!(
        to_json(&TunnelStateDto::Conflict { owner: None }),
        json!({"kind":"conflict"})
    );
}

#[test]
fn should_deserialize_a_conflict_without_an_owner_as_none() {
    let state: TunnelStateDto = serde_json::from_value(json!({"kind":"conflict"})).unwrap();

    assert_eq!(state, TunnelStateDto::Conflict { owner: None });
}

#[test]
fn should_serialize_an_external_process_without_its_absent_options() {
    let process = ExternalProcessDto {
        pid: 7,
        process: "ssh".to_owned(),
        destination: None,
        forwards: vec![ExternalForwardDto {
            kind: TunnelKindDto::Dynamic,
            listen_address: None,
            listen_port: 1080,
            target_host: None,
            target_port: None,
        }],
    };

    assert_eq!(
        to_json(&process),
        json!({"pid":7,"process":"ssh","forwards":[{"kind":"dynamic","listenPort":1080}]})
    );
}

#[test]
fn should_serialize_a_snapshot_with_its_externals() {
    let snapshot = SnapshotDto {
        tunnels: Vec::new(),
        settings: settings(),
        tray: TrayStatusDto::Active,
        externals: vec![external_process()],
        x11_anchors: Vec::new(),
        x11_launchers: Vec::new(),
        openers: Vec::new(),
    };

    assert_eq!(
        to_json(&snapshot)["externals"],
        json!([{
            "pid":4242,
            "process":"ssh",
            "destination":"nova",
            "forwards":[
                {"kind":"local","listenAddress":"127.0.0.1","listenPort":3333,"targetHost":"db.internal","targetPort":5432},
                {"kind":"dynamic","listenPort":1080}
            ]
        }])
    );
}

#[test]
fn should_deserialize_a_snapshot_without_externals_as_empty() {
    let mut snapshot = to_json(&SnapshotDto {
        tunnels: Vec::new(),
        settings: settings(),
        tray: TrayStatusDto::Active,
        externals: vec![external_process()],
        x11_anchors: Vec::new(),
        x11_launchers: Vec::new(),
        openers: Vec::new(),
    });
    snapshot.as_object_mut().unwrap().remove("externals");

    let parsed: SnapshotDto = serde_json::from_value(snapshot).unwrap();

    assert_eq!(parsed.externals, Vec::new());
}

#[test]
fn should_serialize_an_externals_changed_event_as_documented() {
    let event = EventMessage::ExternalsChanged(ExternalsChangedEvent {
        processes: vec![ExternalProcessDto {
            pid: 4242,
            process: "ssh".to_owned(),
            destination: Some("nova".to_owned()),
            forwards: vec![ExternalForwardDto {
                kind: TunnelKindDto::Local,
                listen_address: None,
                listen_port: 3333,
                target_host: Some("db.internal".to_owned()),
                target_port: Some(5432),
            }],
        }],
    });

    assert_eq!(
        encode_event(&event).unwrap(),
        concat!(
            r#"{"event":"externals.changed","data":{"processes":[{"pid":4242,"process":"ssh","destination":"nova","forwards":[{"kind":"local","listenPort":3333,"targetHost":"db.internal","targetPort":5432}]}]}}"#,
            "\n"
        )
    );
}

#[test]
fn should_serialize_the_external_requests_as_documented() {
    let requests = [
        Request::ExternalAdopt(TunnelIdParams {
            id: TUNNEL_ID.to_owned(),
        }),
        Request::ExternalKill(ProcessParams { pid: 4242 }),
        Request::ExternalIgnore(ProcessParams { pid: 4242 }),
    ];

    assert_eq!(
        requests.map(|request| to_json(&request)),
        [
            json!({"method":"external.adopt","params":{"id":TUNNEL_ID}}),
            json!({"method":"external.kill","params":{"pid":4242}}),
            json!({"method":"external.ignore","params":{"pid":4242}}),
        ]
    );
}

#[test]
fn should_serialize_an_adopt_response_as_documented() {
    let adopted = AdoptResult {
        ids: vec![TUNNEL_ID.to_owned()],
    };

    let line = encode_response(Some(8), &Ok(ResponsePayload::Adopt(adopted))).unwrap();

    assert_eq!(
        line,
        "{\"id\":8,\"result\":{\"ids\":[\"01J9ZK3Q8M5N7R2T4V6X8Y0ABC\"]}}\n"
    );
}

#[test]
fn should_serialize_the_new_error_codes_as_documented() {
    let invalid_state = encode_response(Some(9), &Err(IpcError::invalid_state("not external")));
    let kill_failed = encode_response(Some(10), &Err(IpcError::kill_failed("still running")));

    assert_eq!(
        [invalid_state.unwrap(), kill_failed.unwrap()],
        [
            "{\"id\":9,\"error\":{\"code\":\"invalid_state\",\"message\":\"not external\"}}\n",
            "{\"id\":10,\"error\":{\"code\":\"kill_failed\",\"message\":\"still running\"}}\n",
        ]
    );
}

#[test]
fn should_list_thirty_six_methods() {
    assert_eq!(METHOD_NAMES.len(), 36);
}

#[test]
fn should_serialize_every_x11_state_with_a_kind_tag() {
    assert_eq!(
        [
            to_json(&X11StateDto::Stopped),
            to_json(&X11StateDto::Starting),
            to_json(&X11StateDto::Active {
                display: "localhost:10.0".to_owned()
            }),
            to_json(&X11StateDto::Reconnecting {
                attempt: 2,
                next_retry_at: 1_700_000_005_000
            }),
            to_json(&X11StateDto::Failed {
                message: "refused".to_owned()
            }),
        ],
        [
            json!({"kind":"stopped"}),
            json!({"kind":"starting"}),
            json!({"kind":"active","display":"localhost:10.0"}),
            json!({"kind":"reconnecting","attempt":2,"nextRetryAt":1_700_000_005_000_u64}),
            json!({"kind":"failed","message":"refused"}),
        ]
    );
}

#[test]
fn should_serialize_an_x11_anchor_with_its_state() {
    assert_eq!(
        to_json(&x11_anchor()),
        json!({"host":"nova","state":{"kind":"active","display":"localhost:10.0"}})
    );
}

#[test]
fn should_serialize_an_x11_launcher_as_documented() {
    assert_eq!(
        to_json(&x11_launcher()),
        json!({"host":"nova","name":"xterm","command":"xterm -ls"})
    );
}

#[test]
fn should_serialize_a_snapshot_with_its_x11_anchors_and_launchers() {
    let snapshot = SnapshotDto {
        tunnels: Vec::new(),
        settings: settings(),
        tray: TrayStatusDto::Active,
        externals: Vec::new(),
        x11_anchors: vec![x11_anchor()],
        x11_launchers: vec![x11_launcher()],
        openers: Vec::new(),
    };

    let value = to_json(&snapshot);

    assert_eq!(
        (value["x11Anchors"].clone(), value["x11Launchers"].clone()),
        (
            json!([{"host":"nova","state":{"kind":"active","display":"localhost:10.0"}}]),
            json!([{"host":"nova","name":"xterm","command":"xterm -ls"}])
        )
    );
}

#[test]
fn should_deserialize_a_snapshot_without_x11_keys_as_empty() {
    let mut snapshot = to_json(&SnapshotDto {
        tunnels: Vec::new(),
        settings: settings(),
        tray: TrayStatusDto::Active,
        externals: Vec::new(),
        x11_anchors: vec![x11_anchor()],
        x11_launchers: vec![x11_launcher()],
        openers: Vec::new(),
    });
    let fields = snapshot.as_object_mut().unwrap();
    fields.remove("x11Anchors");
    fields.remove("x11Launchers");

    let parsed: SnapshotDto = serde_json::from_value(snapshot).unwrap();

    assert_eq!(
        (parsed.x11_anchors, parsed.x11_launchers),
        (Vec::new(), Vec::new())
    );
}

#[test]
fn should_serialize_an_x11_changed_event_as_documented() {
    let event = EventMessage::X11Changed(X11ChangedEvent {
        host: "nova".to_owned(),
        state: X11StateDto::Active {
            display: "localhost:10.0".to_owned(),
        },
    });

    assert_eq!(
        encode_event(&event).unwrap(),
        concat!(
            r#"{"event":"x11.changed","data":{"host":"nova","state":{"kind":"active","display":"localhost:10.0"}}}"#,
            "\n"
        )
    );
}

#[test]
fn should_serialize_the_x11_requests_as_documented() {
    let requests = [
        Request::X11Start(x11_host()),
        Request::X11Stop(x11_host()),
        Request::X11Launch(x11_launch()),
    ];

    assert_eq!(
        requests.map(|request| to_json(&request)),
        [
            json!({"method":"x11.start","params":{"host":"nova"}}),
            json!({"method":"x11.stop","params":{"host":"nova"}}),
            json!({"method":"x11.launch","params":{"host":"nova","name":"xterm"}}),
        ]
    );
}
