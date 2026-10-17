#![allow(clippy::unwrap_used, clippy::expect_used)]

use ipc::protocol::{
    AdoptResult, ClientKind, ConfigInvalidEvent, CredentialAnswer, CredentialRequestDto,
    CredentialRequestEvent, CredentialResponseParams, EventMessage, ExternalForwardDto,
    ExternalProcessDto, ExternalsChangedEvent, FailureReasonDto, Hello, HostKeyAlertReasonDto,
    HostKeyDecisionDto, HostKeyRequestEvent, HostKeyResponseParams, HostsResult, IpcError,
    LogEntryDto, LogLevelDto, MAX_LINE_BYTES, METHOD_NAMES, MetricsDto, PROTOCOL_VERSION,
    PortOwnerDto, ProcessParams, PromptDto, PromptResolvedEvent, QuitParams, Request,
    RequestEnvelope, ResponsePayload, Secret, ServerFrame, SettingsDto, SnapshotDto, TagParams,
    TargetHealthDto, TrayChangedEvent, TrayStatusDto, TunnelChangedEvent, TunnelDefinitionDto,
    TunnelDto, TunnelIdParams, TunnelKindDto, TunnelLogEvent, TunnelMetricsEvent, TunnelSpecDto,
    TunnelStateDto, X11AnchorDto, X11ChangedEvent, X11HostParams, X11LaunchParams, X11LauncherDto,
    X11StateDto, encode_event, encode_response,
};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};

const TUNNEL_ID: &str = "01J9ZK3Q8M5N7R2T4V6X8Y0ABC";

fn spec() -> TunnelSpecDto {
    TunnelSpecDto {
        name: "adonis".to_owned(),
        host: "nova".to_owned(),
        kind: TunnelKindDto::Local,
        listen_port: 3333,
        listen_address: None,
        target_host: Some("db.internal".to_owned()),
        target_port: Some(5432),
        tags: vec!["dev".to_owned()],
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
        health_probe: true,
        restore_on_launch: true,
        start_on_launch_tags: vec!["dev".to_owned()],
        launch_at_login: false,
        x11_display: None,
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
        Request::TunnelLogs(tunnel_id),
        Request::HostsList {},
        Request::TagStart(tag.clone()),
        Request::TagStop(tag),
        Request::SettingsUpdate(settings()),
        Request::CredentialRespond(CredentialResponseParams {
            prompt_id: "p1".to_owned(),
            answer: Some(CredentialAnswer::Single(Secret::new("hunter2".to_owned()))),
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
        r#"{"id":1,"method":"hello","params":{"protocolVersion":2,"clientKind":"cli","clientVersion":"0.1.0","subscribeEvents":false,"canPrompt":false}}"#
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
        r#"{"id":2,"method":"tunnel.create","params":{"name":"adonis","host":"nova","kind":"local","listenPort":3333,"targetHost":"db.internal","targetPort":5432,"tags":["dev"]}}"#
    );
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

#[test]
fn should_serialize_a_credential_answer_as_a_plain_string() {
    let envelope = RequestEnvelope {
        id: 3,
        request: Request::CredentialRespond(CredentialResponseParams {
            prompt_id: "p1".to_owned(),
            answer: Some(CredentialAnswer::Single(Secret::new("hunter2".to_owned()))),
        }),
    };

    assert_eq!(
        serde_json::to_string(&envelope).unwrap(),
        r#"{"id":3,"method":"credential.respond","params":{"promptId":"p1","answer":"hunter2"}}"#
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
            answer: None
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
    };

    let line = encode_response(Some(4), &Ok(ResponsePayload::Snapshot(snapshot))).unwrap();

    assert_eq!(
        line,
        concat!(
            r#"{"id":4,"result":{"tunnels":[{"id":"01J9ZK3Q8M5N7R2T4V6X8Y0ABC","name":"adonis","host":"nova","kind":"local","listenPort":3333,"targetHost":"db.internal","targetPort":5432,"tags":["dev"],"#,
            r#""state":{"kind":"reconnecting","attempt":2,"nextRetryAt":1700000005000},"#,
            r#""health":{"kind":"unreachable","since":1700000000000,"error":"connection refused"},"#,
            r#""metrics":{"activeConnections":1,"totalConnections":4,"bytesIn":10,"bytesOut":20,"lastActivity":1700000001000}}],"#,
            r#""settings":{"defaultBindAddress":"127.0.0.1","defaultRemoteBindAddress":"127.0.0.1","keepaliveIntervalMs":30000,"keepaliveMax":3,"sessionIdleGraceMs":60000,"reconnectInitialDelayMs":1000,"reconnectMaxDelayMs":60000,"healthProbe":true,"restoreOnLaunch":true,"startOnLaunchTags":["dev"],"launchAtLogin":false},"tray":{"kind":"active"},"externals":[],"x11Anchors":[],"x11Launchers":[]}}"#,
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
            r#"{"id":1,"error":{"code":"protocol_mismatch","message":"the daemon speaks protocol 2, the client protocol 9","data":{"client":9,"daemon":2}}}"#,
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
fn should_list_twenty_three_methods() {
    assert_eq!(METHOD_NAMES.len(), 23);
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
