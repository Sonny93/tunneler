#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;

use ipc::protocol::{
    ConfigExportParams, ConfigExportResult, ConfigImportAddedDto, ConfigImportApplyParams,
    ConfigImportEntryDto, ConfigImportEntryWarningDto, ConfigImportParams, ConfigImportPlanDto,
    ConfigImportReportDto, ConfigImportRiskDto, ConfigImportSkippedDto, ConfigImportSourceDto,
    ConfigImportStatusDto, ErrorCode, ImportClassifyParams, ImportClassifyResult,
    ImportConflictDto, ImportRouteDto, InvalidImportFileDto, InvalidImportReasonDto, IpcError,
    MAX_CONFIG_IMPORT_BYTES, MAX_CONFIG_IMPORT_TUNNELS, MAX_IMPORT_CLASSIFY_CANDIDATES,
    OpenOptionsDto, Request, RequestEnvelope, ResponsePayload, TunnelKindDto, TunnelSpecDto,
    encode_response,
};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};

const EXISTING_ID: &str = "01J9ZK3Q8M5N7R2T4V6X8Y0ABC";

fn tunnel_spec() -> TunnelSpecDto {
    TunnelSpecDto {
        host: "nova".to_owned(),
        kind: TunnelKindDto::Local,
        listen_port: 3333,
        listen_address: None,
        target_host: None,
        target_port: None,
        tags: vec!["dev".to_owned()],
        open: OpenOptionsDto::default(),
        standby: None,
    }
}

fn entry(index: u32, status: ConfigImportStatusDto) -> ConfigImportEntryDto {
    let is_importable = status == ConfigImportStatusDto::Importable;
    ConfigImportEntryDto {
        index,
        tunnel: tunnel_spec(),
        file_host: "oldbox".to_owned(),
        effective_listen_address: "127.0.0.1".to_owned(),
        status,
        risks: Vec::new(),
        warnings: Vec::new(),
        selected_by_default: is_importable,
    }
}

fn route(listen_port: u16) -> ImportRouteDto {
    ImportRouteDto {
        host: "nova".to_owned(),
        kind: TunnelKindDto::Local,
        listen_port,
        listen_address: None,
        target_host: None,
        target_port: None,
    }
}

fn parse(line: &str) -> Result<RequestEnvelope, ipc::protocol::RequestRejection> {
    RequestEnvelope::parse(line.as_bytes())
}

fn response_json(payload: ResponsePayload) -> Value {
    let line = encode_response(Some(1), &Ok(payload)).unwrap();
    serde_json::from_str(&line).unwrap()
}

#[test]
fn should_cap_an_import_file_at_256_kib_and_1000_tunnels() {
    assert_eq!(
        (
            MAX_CONFIG_IMPORT_BYTES,
            MAX_CONFIG_IMPORT_TUNNELS,
            MAX_IMPORT_CLASSIFY_CANDIDATES
        ),
        (262_144, 1000, 1000)
    );
}

#[test]
fn should_serialize_config_export_as_documented() {
    let envelope = RequestEnvelope {
        id: 1,
        request: Request::ConfigExport(ConfigExportParams {
            hosts: vec!["nova".to_owned()],
            tags: vec!["dev".to_owned(), "mail".to_owned()],
        }),
    };

    assert_eq!(
        serde_json::to_string(&envelope).unwrap(),
        r#"{"id":1,"method":"config.export","params":{"hosts":["nova"],"tags":["dev","mail"]}}"#
    );
}

#[test]
fn should_default_the_export_filters_to_empty_when_they_are_omitted() {
    let envelope = parse(r#"{"id":1,"method":"config.export","params":{}}"#).unwrap();

    assert_eq!(
        envelope.request,
        Request::ConfigExport(ConfigExportParams::default())
    );
}

#[test]
fn should_default_the_export_filters_to_empty_when_params_are_omitted() {
    let envelope = parse(r#"{"id":1,"method":"config.export"}"#).unwrap();

    assert_eq!(
        envelope.request,
        Request::ConfigExport(ConfigExportParams::default())
    );
}

#[test]
fn should_serialize_an_export_response_as_documented() {
    let result = ConfigExportResult {
        contents: "format = \"tunneler-export\"\n".to_owned(),
        tunnel_count: 2,
    };

    assert_eq!(
        response_json(ResponsePayload::ConfigExport(result)),
        json!({
            "id": 1,
            "result": { "contents": "format = \"tunneler-export\"\n", "tunnelCount": 2 }
        })
    );
}

#[test]
fn should_serialize_config_import_preview_as_documented() {
    let envelope = RequestEnvelope {
        id: 2,
        request: Request::ConfigImportPreview(ConfigImportParams {
            contents: "x".to_owned(),
            host_map: BTreeMap::from([("oldbox".to_owned(), "nova".to_owned())]),
        }),
    };

    assert_eq!(
        serde_json::to_string(&envelope).unwrap(),
        r#"{"id":2,"method":"config.import_preview","params":{"contents":"x","hostMap":{"oldbox":"nova"}}}"#
    );
}

#[test]
fn should_default_the_host_map_to_empty_when_it_is_omitted_from_a_preview() {
    let envelope =
        parse(r#"{"id":2,"method":"config.import_preview","params":{"contents":"x"}}"#).unwrap();

    assert_eq!(
        envelope.request,
        Request::ConfigImportPreview(ConfigImportParams {
            contents: "x".to_owned(),
            host_map: BTreeMap::new(),
        })
    );
}

#[test]
fn should_answer_invalid_request_when_a_preview_has_no_contents() {
    let rejection = parse(r#"{"id":2,"method":"config.import_preview","params":{}}"#).unwrap_err();

    assert_eq!(
        (rejection.id, rejection.error.code),
        (Some(2), ErrorCode::InvalidRequest)
    );
}

#[test]
fn should_serialize_config_import_apply_as_documented() {
    let envelope = RequestEnvelope {
        id: 3,
        request: Request::ConfigImportApply(ConfigImportApplyParams {
            contents: "x".to_owned(),
            host_map: BTreeMap::new(),
            indexes: vec![0, 2],
        }),
    };

    assert_eq!(
        serde_json::to_string(&envelope).unwrap(),
        r#"{"id":3,"method":"config.import_apply","params":{"contents":"x","indexes":[0,2]}}"#
    );
}

#[test]
fn should_default_the_host_map_to_empty_when_it_is_omitted_from_an_apply() {
    let envelope =
        parse(r#"{"id":3,"method":"config.import_apply","params":{"contents":"x","indexes":[1]}}"#)
            .unwrap();

    assert_eq!(
        envelope.request,
        Request::ConfigImportApply(ConfigImportApplyParams {
            contents: "x".to_owned(),
            host_map: BTreeMap::new(),
            indexes: vec![1],
        })
    );
}

#[test]
fn should_answer_invalid_request_when_an_apply_has_no_indexes() {
    let rejection =
        parse(r#"{"id":3,"method":"config.import_apply","params":{"contents":"x"}}"#).unwrap_err();

    assert_eq!(
        (rejection.id, rejection.error.code),
        (Some(3), ErrorCode::InvalidRequest)
    );
}

#[test]
fn should_serialize_an_import_plan_response_as_documented() {
    let mut risky = entry(0, ConfigImportStatusDto::Importable);
    risky.tunnel.listen_address = Some("0.0.0.0".to_owned());
    risky.effective_listen_address = "0.0.0.0".to_owned();
    risky.risks = vec![
        ConfigImportRiskDto::NonLoopbackListen {
            address: "0.0.0.0".to_owned(),
        },
        ConfigImportRiskDto::RemoteNonLoopbackTarget {
            target_host: "198.51.100.7".to_owned(),
        },
        ConfigImportRiskDto::StartsOnLaunch {
            tags: vec!["dev".to_owned()],
        },
    ];
    risky.warnings = vec![
        ConfigImportEntryWarningDto::UnknownHost {
            host: "oldbox".to_owned(),
        },
        ConfigImportEntryWarningDto::OpenerDropped {
            opener: "remmina".to_owned(),
        },
    ];
    risky.selected_by_default = false;
    let plan = ConfigImportPlanDto {
        source: ConfigImportSourceDto::Export,
        entries: vec![risky, entry(1, ConfigImportStatusDto::Importable)],
        unknown_hosts: vec!["oldbox".to_owned()],
    };

    assert_eq!(
        response_json(ResponsePayload::ConfigImportPlan(plan)),
        json!({
            "id": 1,
            "result": {
                "source": "export",
                "entries": [
                    {
                        "index": 0,
                        "tunnel": {
                            "host": "nova",
                            "kind": "local",
                            "listenPort": 3333,
                            "listenAddress": "0.0.0.0",
                            "tags": ["dev"]
                        },
                        "fileHost": "oldbox",
                        "effectiveListenAddress": "0.0.0.0",
                        "status": { "kind": "importable" },
                        "risks": [
                            { "kind": "nonLoopbackListen", "address": "0.0.0.0" },
                            { "kind": "remoteNonLoopbackTarget", "targetHost": "198.51.100.7" },
                            { "kind": "startsOnLaunch", "tags": ["dev"] }
                        ],
                        "warnings": [
                            { "kind": "unknownHost", "host": "oldbox" },
                            { "kind": "openerDropped", "opener": "remmina" }
                        ],
                        "selectedByDefault": false
                    },
                    {
                        "index": 1,
                        "tunnel": {
                            "host": "nova",
                            "kind": "local",
                            "listenPort": 3333,
                            "tags": ["dev"]
                        },
                        "fileHost": "oldbox",
                        "effectiveListenAddress": "127.0.0.1",
                        "status": { "kind": "importable" },
                        "risks": [],
                        "warnings": [],
                        "selectedByDefault": true
                    }
                ],
                "unknownHosts": ["oldbox"]
            }
        })
    );
}

#[test]
fn should_serialize_the_store_source_as_a_lowercase_word() {
    assert_eq!(
        serde_json::to_value(ConfigImportSourceDto::Store).unwrap(),
        json!("store")
    );
}

#[test]
fn should_serialize_every_import_status_as_documented() {
    let statuses = [
        ConfigImportStatusDto::Importable,
        ConfigImportStatusDto::AlreadyConfigured {
            existing_id: EXISTING_ID.to_owned(),
            ignored_tags: vec!["mail".to_owned()],
        },
        ConfigImportStatusDto::ListenConflict {
            holder_id: EXISTING_ID.to_owned(),
        },
        ConfigImportStatusDto::DuplicateInFile { earlier_index: 4 },
        ConfigImportStatusDto::Invalid {
            messages: vec!["port 0 is not allowed".to_owned()],
        },
    ];

    assert_eq!(
        serde_json::to_value(statuses).unwrap(),
        json!([
            { "kind": "importable" },
            { "kind": "alreadyConfigured", "existingId": EXISTING_ID, "ignoredTags": ["mail"] },
            { "kind": "listenConflict", "holderId": EXISTING_ID },
            { "kind": "duplicateInFile", "earlierIndex": 4 },
            { "kind": "invalid", "messages": ["port 0 is not allowed"] }
        ])
    );
}

#[test]
fn should_round_trip_every_import_status() {
    let statuses = vec![
        ConfigImportStatusDto::Importable,
        ConfigImportStatusDto::AlreadyConfigured {
            existing_id: EXISTING_ID.to_owned(),
            ignored_tags: Vec::new(),
        },
        ConfigImportStatusDto::ListenConflict {
            holder_id: EXISTING_ID.to_owned(),
        },
        ConfigImportStatusDto::DuplicateInFile { earlier_index: 0 },
        ConfigImportStatusDto::Invalid {
            messages: Vec::new(),
        },
    ];

    let text = serde_json::to_string(&statuses).unwrap();

    assert_eq!(
        serde_json::from_str::<Vec<ConfigImportStatusDto>>(&text).unwrap(),
        statuses
    );
}

#[test]
fn should_serialize_an_import_report_as_documented() {
    let report = ConfigImportReportDto {
        added: vec![ConfigImportAddedDto {
            index: 0,
            id: EXISTING_ID.to_owned(),
        }],
        skipped: vec![ConfigImportSkippedDto {
            index: 2,
            status: ConfigImportStatusDto::ListenConflict {
                holder_id: EXISTING_ID.to_owned(),
            },
        }],
    };

    assert_eq!(
        response_json(ResponsePayload::ConfigImportReport(report)),
        json!({
            "id": 1,
            "result": {
                "added": [{ "index": 0, "id": EXISTING_ID }],
                "skipped": [
                    {
                        "index": 2,
                        "status": { "kind": "listenConflict", "holderId": EXISTING_ID }
                    }
                ]
            }
        })
    );
}

#[test]
fn should_serialize_import_classify_as_documented() {
    let envelope = RequestEnvelope {
        id: 4,
        request: Request::ImportClassify(ImportClassifyParams {
            candidates: vec![
                ImportRouteDto {
                    host: "nova".to_owned(),
                    kind: TunnelKindDto::Local,
                    listen_port: 3333,
                    listen_address: Some("127.0.0.1".to_owned()),
                    target_host: Some("db.internal".to_owned()),
                    target_port: Some(5432),
                },
                ImportRouteDto {
                    host: "nova".to_owned(),
                    kind: TunnelKindDto::Dynamic,
                    listen_port: 1080,
                    listen_address: None,
                    target_host: None,
                    target_port: None,
                },
            ],
        }),
    };

    assert_eq!(
        serde_json::to_value(&envelope).unwrap(),
        json!({
            "id": 4,
            "method": "import.classify",
            "params": {
                "candidates": [
                    {
                        "host": "nova",
                        "kind": "local",
                        "listenPort": 3333,
                        "listenAddress": "127.0.0.1",
                        "targetHost": "db.internal",
                        "targetPort": 5432
                    },
                    { "host": "nova", "kind": "dynamic", "listenPort": 1080 }
                ]
            }
        })
    );
}

#[test]
fn should_accept_import_classify_with_the_maximum_number_of_candidates() {
    let envelope = RequestEnvelope {
        id: 4,
        request: Request::ImportClassify(ImportClassifyParams {
            candidates: vec![route(3333); MAX_IMPORT_CLASSIFY_CANDIDATES],
        }),
    };
    let line = serde_json::to_vec(&envelope).unwrap();

    assert_eq!(RequestEnvelope::parse(&line).unwrap(), envelope);
}

#[test]
fn should_reject_import_classify_when_there_are_too_many_candidates() {
    let envelope = RequestEnvelope {
        id: 5,
        request: Request::ImportClassify(ImportClassifyParams {
            candidates: vec![route(3333); MAX_IMPORT_CLASSIFY_CANDIDATES + 1],
        }),
    };
    let line = serde_json::to_vec(&envelope).unwrap();

    let rejection = RequestEnvelope::parse(&line).unwrap_err();

    assert_eq!(
        (rejection.id, rejection.error.code),
        (Some(5), ErrorCode::InvalidRequest)
    );
}

#[test]
fn should_serialize_an_import_classify_response_as_documented() {
    let result = ImportClassifyResult {
        conflicts: vec![
            None,
            Some(ImportConflictDto::AlreadyConfigured),
            Some(ImportConflictDto::ListenConflict {
                holder_id: EXISTING_ID.to_owned(),
            }),
            Some(ImportConflictDto::DuplicateInImport),
        ],
    };

    assert_eq!(
        response_json(ResponsePayload::ImportClassify(result)),
        json!({
            "id": 1,
            "result": {
                "conflicts": [
                    null,
                    { "kind": "alreadyConfigured" },
                    { "kind": "listenConflict", "holderId": EXISTING_ID },
                    { "kind": "duplicateInImport" }
                ]
            }
        })
    );
}

#[test]
fn should_serialize_the_invalid_import_file_code_as_documented() {
    assert_eq!(
        serde_json::to_value(ErrorCode::InvalidImportFile).unwrap(),
        json!("invalid_import_file")
    );
}

#[test]
fn should_serialize_every_invalid_import_file_reason_with_its_data() {
    let mut syntax = InvalidImportFileDto::new(InvalidImportReasonDto::Syntax);
    syntax.line = Some(3);
    syntax.message = Some("expected `=`".to_owned());
    let not_tunneler_file = InvalidImportFileDto::new(InvalidImportReasonDto::NotTunnelerFile);
    let mut newer_version = InvalidImportFileDto::new(InvalidImportReasonDto::NewerVersion);
    newer_version.found = Some(2);
    newer_version.supported = Some(1);
    let mut unsafe_characters = InvalidImportFileDto::new(InvalidImportReasonDto::UnsafeCharacters);
    unsafe_characters.entry = Some(4);
    unsafe_characters.field = Some("host".to_owned());
    let mut too_large = InvalidImportFileDto::new(InvalidImportReasonDto::TooLarge);
    too_large.limit = Some(262_144);
    let mut too_many_entries = InvalidImportFileDto::new(InvalidImportReasonDto::TooManyEntries);
    too_many_entries.limit = Some(1000);

    let errors = [
        syntax,
        not_tunneler_file,
        newer_version,
        unsafe_characters,
        too_large,
        too_many_entries,
    ]
    .map(|details| IpcError::invalid_import_file("refused", &details));

    assert_eq!(
        errors.map(|error| (error.code.as_str(), error.data.unwrap())),
        [
            (
                "invalid_import_file",
                json!({ "reason": "syntax", "line": 3, "message": "expected `=`" })
            ),
            (
                "invalid_import_file",
                json!({ "reason": "notTunnelerFile" })
            ),
            (
                "invalid_import_file",
                json!({ "reason": "newerVersion", "found": 2, "supported": 1 })
            ),
            (
                "invalid_import_file",
                json!({ "reason": "unsafeCharacters", "entry": 4, "field": "host" })
            ),
            (
                "invalid_import_file",
                json!({ "reason": "tooLarge", "limit": 262_144 })
            ),
            (
                "invalid_import_file",
                json!({ "reason": "tooManyEntries", "limit": 1000 })
            ),
        ]
    );
}

#[test]
fn should_round_trip_the_invalid_import_file_data() {
    let mut details = InvalidImportFileDto::new(InvalidImportReasonDto::UnsafeCharacters);
    details.entry = Some(1);
    details.field = Some("tags".to_owned());
    let error = IpcError::invalid_import_file("refused", &details);

    let parsed: InvalidImportFileDto = serde_json::from_value(error.data.unwrap()).unwrap();

    assert_eq!(parsed, details);
}
