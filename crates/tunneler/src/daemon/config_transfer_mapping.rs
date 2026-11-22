//! Conversions between the engine types of the configuration transfer and the
//! wire types of `config.export`, `config.import_preview`,
//! `config.import_apply` and `import.classify`.

use engine::{
    ConfigExport, ConfigExportFilter, ConfigImportEntry, ConfigImportEntryWarning,
    ConfigImportPlan, ConfigImportReport, ConfigImportRisk, ConfigImportSource, ConfigImportStatus,
    ImportFileError, ImportRoute, NewTunnel, RouteConflict,
};
use ipc::protocol::{
    ConfigExportParams, ConfigExportResult, ConfigImportAddedDto, ConfigImportEntryDto,
    ConfigImportEntryWarningDto, ConfigImportPlanDto, ConfigImportReportDto, ConfigImportRiskDto,
    ConfigImportSkippedDto, ConfigImportSourceDto, ConfigImportStatusDto, ImportClassifyResult,
    ImportConflictDto, ImportRouteDto, InvalidImportFileDto, InvalidImportReasonDto, IpcError,
    TunnelSpecDto,
};

use super::mapping::{kind_from_dto, kind_to_dto, open_options_to_dto};

const COUNT_OVERFLOW_FALLBACK: u32 = u32::MAX;

fn saturating_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(COUNT_OVERFLOW_FALLBACK)
}

fn saturating_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

pub(crate) fn export_filter_from_params(params: ConfigExportParams) -> ConfigExportFilter {
    ConfigExportFilter {
        hosts: params.hosts,
        tags: params.tags,
    }
}

pub(crate) fn export_to_result(export: ConfigExport) -> ConfigExportResult {
    ConfigExportResult {
        contents: export.contents,
        tunnel_count: saturating_u64(export.tunnel_count),
    }
}

pub(crate) fn import_route_from_dto(route: ImportRouteDto) -> ImportRoute {
    ImportRoute {
        host: route.host,
        kind: kind_from_dto(route.kind),
        listen_port: route.listen_port,
        listen_address: route.listen_address,
        target_host: route.target_host,
        target_port: route.target_port,
    }
}

pub(crate) fn conflicts_to_result(conflicts: &[Option<RouteConflict>]) -> ImportClassifyResult {
    ImportClassifyResult {
        conflicts: conflicts
            .iter()
            .map(|conflict| conflict.as_ref().map(conflict_to_dto))
            .collect(),
    }
}

fn conflict_to_dto(conflict: &RouteConflict) -> ImportConflictDto {
    match conflict {
        RouteConflict::AlreadyConfigured { .. } => ImportConflictDto::AlreadyConfigured,
        RouteConflict::ListenConflict { holder_id } => ImportConflictDto::ListenConflict {
            holder_id: holder_id.to_string(),
        },
        RouteConflict::DuplicateInImport { .. } => ImportConflictDto::DuplicateInImport,
    }
}

pub(crate) fn plan_to_dto(plan: &ConfigImportPlan) -> ConfigImportPlanDto {
    ConfigImportPlanDto {
        source: source_to_dto(plan.source),
        entries: plan.entries.iter().map(entry_to_dto).collect(),
        unknown_hosts: plan.unknown_hosts.clone(),
    }
}

fn source_to_dto(source: ConfigImportSource) -> ConfigImportSourceDto {
    match source {
        ConfigImportSource::Export => ConfigImportSourceDto::Export,
        ConfigImportSource::Store => ConfigImportSourceDto::Store,
    }
}

fn entry_to_dto(entry: &ConfigImportEntry) -> ConfigImportEntryDto {
    ConfigImportEntryDto {
        index: saturating_u32(entry.index),
        tunnel: new_tunnel_to_spec(&entry.tunnel),
        file_host: entry.file_host.clone(),
        effective_listen_address: entry.effective_listen_address.clone(),
        status: status_to_dto(&entry.status),
        risks: entry.risks.iter().map(risk_to_dto).collect(),
        warnings: entry.warnings.iter().map(entry_warning_to_dto).collect(),
        selected_by_default: entry.selected_by_default,
    }
}

fn new_tunnel_to_spec(tunnel: &NewTunnel) -> TunnelSpecDto {
    TunnelSpecDto {
        host: tunnel.host.clone(),
        kind: kind_to_dto(tunnel.kind),
        listen_port: tunnel.listen_port,
        listen_address: tunnel.listen_address.clone(),
        target_host: tunnel.target_host.clone(),
        target_port: tunnel.target_port,
        tags: tunnel.tags.clone(),
        open: open_options_to_dto(&tunnel.open),
        standby: tunnel.standby,
    }
}

fn status_to_dto(status: &ConfigImportStatus) -> ConfigImportStatusDto {
    match status {
        ConfigImportStatus::Importable => ConfigImportStatusDto::Importable,
        ConfigImportStatus::AlreadyConfigured {
            existing_id,
            ignored_tags,
        } => ConfigImportStatusDto::AlreadyConfigured {
            existing_id: existing_id.to_string(),
            ignored_tags: ignored_tags.clone(),
        },
        ConfigImportStatus::ListenConflict { holder_id } => ConfigImportStatusDto::ListenConflict {
            holder_id: holder_id.to_string(),
        },
        ConfigImportStatus::DuplicateInFile { earlier_index } => {
            ConfigImportStatusDto::DuplicateInFile {
                earlier_index: saturating_u32(*earlier_index),
            }
        }
        ConfigImportStatus::Invalid { messages } => ConfigImportStatusDto::Invalid {
            messages: messages.clone(),
        },
    }
}

fn risk_to_dto(risk: &ConfigImportRisk) -> ConfigImportRiskDto {
    match risk {
        ConfigImportRisk::NonLoopbackListen { address } => ConfigImportRiskDto::NonLoopbackListen {
            address: address.clone(),
        },
        ConfigImportRisk::RemoteNonLoopbackTarget { target_host } => {
            ConfigImportRiskDto::RemoteNonLoopbackTarget {
                target_host: target_host.clone(),
            }
        }
        ConfigImportRisk::StartsOnLaunch { tags } => {
            ConfigImportRiskDto::StartsOnLaunch { tags: tags.clone() }
        }
    }
}

fn entry_warning_to_dto(warning: &ConfigImportEntryWarning) -> ConfigImportEntryWarningDto {
    match warning {
        ConfigImportEntryWarning::UnknownHost { host } => {
            ConfigImportEntryWarningDto::UnknownHost { host: host.clone() }
        }
        ConfigImportEntryWarning::OpenerDropped { opener } => {
            ConfigImportEntryWarningDto::OpenerDropped {
                opener: opener.clone(),
            }
        }
    }
}

pub(crate) fn report_to_dto(report: &ConfigImportReport) -> ConfigImportReportDto {
    ConfigImportReportDto {
        added: report
            .added
            .iter()
            .map(|added| ConfigImportAddedDto {
                index: saturating_u32(added.index),
                id: added.id.to_string(),
            })
            .collect(),
        skipped: report
            .skipped
            .iter()
            .map(|skipped| ConfigImportSkippedDto {
                index: saturating_u32(skipped.index),
                status: status_to_dto(&skipped.status),
            })
            .collect(),
    }
}

/// The requested indexes as positions in the file; an index that does not fit
/// a position cannot name an entry, so it is left out.
pub(crate) fn indexes_from_dto(indexes: &[u32]) -> Vec<usize> {
    indexes
        .iter()
        .filter_map(|index| usize::try_from(*index).ok())
        .collect()
}

pub(crate) fn invalid_import_file_to_ipc(error: &ImportFileError) -> IpcError {
    IpcError::invalid_import_file(error.to_string(), &invalid_import_file_to_dto(error))
}

fn invalid_import_file_to_dto(error: &ImportFileError) -> InvalidImportFileDto {
    match error {
        ImportFileError::Syntax { line, message } => InvalidImportFileDto {
            line: line.map(saturating_u32),
            message: Some(message.clone()),
            ..InvalidImportFileDto::new(InvalidImportReasonDto::Syntax)
        },
        ImportFileError::NotTunnelerFile => {
            InvalidImportFileDto::new(InvalidImportReasonDto::NotTunnelerFile)
        }
        ImportFileError::NewerVersion { found, supported } => InvalidImportFileDto {
            found: Some(*found),
            supported: Some(*supported),
            ..InvalidImportFileDto::new(InvalidImportReasonDto::NewerVersion)
        },
        ImportFileError::UnsafeCharacters { entry, field } => InvalidImportFileDto {
            entry: Some(saturating_u32(*entry)),
            field: Some((*field).to_owned()),
            ..InvalidImportFileDto::new(InvalidImportReasonDto::UnsafeCharacters)
        },
        ImportFileError::TooLarge { limit, .. } => InvalidImportFileDto {
            limit: Some(saturating_u64(*limit)),
            ..InvalidImportFileDto::new(InvalidImportReasonDto::TooLarge)
        },
        ImportFileError::TooManyEntries { limit, .. } => InvalidImportFileDto {
            limit: Some(saturating_u64(*limit)),
            ..InvalidImportFileDto::new(InvalidImportReasonDto::TooManyEntries)
        },
    }
}

#[cfg(test)]
mod tests {
    use engine::{ConfigImportAdded, ConfigImportSkipped, OpenOptions, TunnelId, TunnelKind};
    use ipc::protocol::{ErrorCode, TunnelKindDto};
    use serde_json::json;

    use super::*;

    fn tunnel_id(text: &str) -> TunnelId {
        text.parse().unwrap()
    }

    const FIRST_ID: &str = "01J0000000000000000000000A";
    const SECOND_ID: &str = "01J0000000000000000000000B";

    fn new_tunnel() -> NewTunnel {
        NewTunnel {
            host: "nova".to_owned(),
            kind: TunnelKind::Remote,
            listen_port: 9000,
            listen_address: Some("0.0.0.0".to_owned()),
            target_host: Some("db".to_owned()),
            target_port: Some(5432),
            tags: vec!["dev".to_owned()],
            open: OpenOptions {
                path: Some("/admin".to_owned()),
                ..OpenOptions::default()
            },
            standby: Some(true),
        }
    }

    fn entry(status: ConfigImportStatus) -> ConfigImportEntry {
        ConfigImportEntry {
            index: 4,
            tunnel: new_tunnel(),
            file_host: "oldbox".to_owned(),
            effective_listen_address: "0.0.0.0".to_owned(),
            status,
            risks: Vec::new(),
            warnings: Vec::new(),
            selected_by_default: false,
        }
    }

    fn plan_of(entry: ConfigImportEntry) -> ConfigImportPlan {
        ConfigImportPlan {
            source: ConfigImportSource::Store,
            entries: vec![entry],
            unknown_hosts: vec!["oldbox".to_owned()],
        }
    }

    #[test]
    fn should_carry_every_field_of_a_plan_entry_when_converting_a_plan() {
        let dto = plan_to_dto(&plan_of(entry(ConfigImportStatus::Importable)));

        assert_eq!(
            dto,
            ConfigImportPlanDto {
                source: ConfigImportSourceDto::Store,
                entries: vec![ConfigImportEntryDto {
                    index: 4,
                    tunnel: TunnelSpecDto {
                        host: "nova".to_owned(),
                        kind: TunnelKindDto::Remote,
                        listen_port: 9000,
                        listen_address: Some("0.0.0.0".to_owned()),
                        target_host: Some("db".to_owned()),
                        target_port: Some(5432),
                        tags: vec!["dev".to_owned()],
                        open: ipc::protocol::OpenOptionsDto {
                            path: Some("/admin".to_owned()),
                            ..ipc::protocol::OpenOptionsDto::default()
                        },
                        standby: Some(true),
                    },
                    file_host: "oldbox".to_owned(),
                    effective_listen_address: "0.0.0.0".to_owned(),
                    status: ConfigImportStatusDto::Importable,
                    risks: Vec::new(),
                    warnings: Vec::new(),
                    selected_by_default: false,
                }],
                unknown_hosts: vec!["oldbox".to_owned()],
            }
        );
    }

    #[test]
    fn should_map_the_export_source_when_converting_a_plan() {
        let plan = ConfigImportPlan {
            source: ConfigImportSource::Export,
            entries: Vec::new(),
            unknown_hosts: Vec::new(),
        };

        assert_eq!(plan_to_dto(&plan).source, ConfigImportSourceDto::Export);
    }

    #[test]
    fn should_map_every_status_to_its_wire_status() {
        let statuses = [
            ConfigImportStatus::Importable,
            ConfigImportStatus::AlreadyConfigured {
                existing_id: tunnel_id(FIRST_ID),
                ignored_tags: vec!["mail".to_owned()],
            },
            ConfigImportStatus::ListenConflict {
                holder_id: tunnel_id(SECOND_ID),
            },
            ConfigImportStatus::DuplicateInFile { earlier_index: 1 },
            ConfigImportStatus::Invalid {
                messages: vec!["bad port".to_owned()],
            },
        ];

        let converted: Vec<ConfigImportStatusDto> = statuses.iter().map(status_to_dto).collect();

        assert_eq!(
            converted,
            vec![
                ConfigImportStatusDto::Importable,
                ConfigImportStatusDto::AlreadyConfigured {
                    existing_id: FIRST_ID.to_owned(),
                    ignored_tags: vec!["mail".to_owned()]
                },
                ConfigImportStatusDto::ListenConflict {
                    holder_id: SECOND_ID.to_owned()
                },
                ConfigImportStatusDto::DuplicateInFile { earlier_index: 1 },
                ConfigImportStatusDto::Invalid {
                    messages: vec!["bad port".to_owned()]
                },
            ]
        );
    }

    #[test]
    fn should_map_every_risk_and_warning_to_its_wire_form() {
        let mut planned = entry(ConfigImportStatus::Importable);
        planned.risks = vec![
            ConfigImportRisk::NonLoopbackListen {
                address: "0.0.0.0".to_owned(),
            },
            ConfigImportRisk::RemoteNonLoopbackTarget {
                target_host: "db".to_owned(),
            },
            ConfigImportRisk::StartsOnLaunch {
                tags: vec!["dev".to_owned()],
            },
        ];
        planned.warnings = vec![
            ConfigImportEntryWarning::UnknownHost {
                host: "nova".to_owned(),
            },
            ConfigImportEntryWarning::OpenerDropped {
                opener: "vlc".to_owned(),
            },
        ];

        let dto = entry_to_dto(&planned);

        assert_eq!(
            (dto.risks, dto.warnings),
            (
                vec![
                    ConfigImportRiskDto::NonLoopbackListen {
                        address: "0.0.0.0".to_owned()
                    },
                    ConfigImportRiskDto::RemoteNonLoopbackTarget {
                        target_host: "db".to_owned()
                    },
                    ConfigImportRiskDto::StartsOnLaunch {
                        tags: vec!["dev".to_owned()]
                    },
                ],
                vec![
                    ConfigImportEntryWarningDto::UnknownHost {
                        host: "nova".to_owned()
                    },
                    ConfigImportEntryWarningDto::OpenerDropped {
                        opener: "vlc".to_owned()
                    },
                ]
            )
        );
    }

    #[test]
    fn should_carry_the_default_selection_when_converting_an_entry() {
        let mut planned = entry(ConfigImportStatus::Importable);
        planned.selected_by_default = true;

        assert!(entry_to_dto(&planned).selected_by_default);
    }

    #[test]
    fn should_convert_the_report_with_ids_as_text_and_indexes_as_numbers() {
        let report = ConfigImportReport {
            added: vec![ConfigImportAdded {
                index: 0,
                id: tunnel_id(FIRST_ID),
            }],
            skipped: vec![ConfigImportSkipped {
                index: 2,
                status: ConfigImportStatus::DuplicateInFile { earlier_index: 0 },
            }],
        };

        assert_eq!(
            report_to_dto(&report),
            ConfigImportReportDto {
                added: vec![ConfigImportAddedDto {
                    index: 0,
                    id: FIRST_ID.to_owned()
                }],
                skipped: vec![ConfigImportSkippedDto {
                    index: 2,
                    status: ConfigImportStatusDto::DuplicateInFile { earlier_index: 0 }
                }],
            }
        );
    }

    #[test]
    fn should_saturate_a_count_that_does_not_fit_when_converting_an_index() {
        assert_eq!(saturating_u32(usize::MAX), u32::MAX);
    }

    #[test]
    fn should_convert_the_requested_indexes_to_positions() {
        assert_eq!(indexes_from_dto(&[0, 7, 3]), vec![0, 7, 3]);
    }

    #[test]
    fn should_convert_every_field_of_an_import_route() {
        let route = import_route_from_dto(ImportRouteDto {
            host: "nova".to_owned(),
            kind: TunnelKindDto::Dynamic,
            listen_port: 1080,
            listen_address: Some("127.0.0.1".to_owned()),
            target_host: Some("db".to_owned()),
            target_port: Some(5432),
        });

        assert_eq!(
            route,
            ImportRoute {
                host: "nova".to_owned(),
                kind: TunnelKind::Dynamic,
                listen_port: 1080,
                listen_address: Some("127.0.0.1".to_owned()),
                target_host: Some("db".to_owned()),
                target_port: Some(5432),
            }
        );
    }

    #[test]
    fn should_map_every_route_conflict_and_keep_the_order_when_classifying() {
        let conflicts = [
            None,
            Some(RouteConflict::AlreadyConfigured {
                existing_id: tunnel_id(FIRST_ID),
            }),
            Some(RouteConflict::ListenConflict {
                holder_id: tunnel_id(SECOND_ID),
            }),
            Some(RouteConflict::DuplicateInImport { earlier_index: 0 }),
        ];

        assert_eq!(
            conflicts_to_result(&conflicts).conflicts,
            vec![
                None,
                Some(ImportConflictDto::AlreadyConfigured),
                Some(ImportConflictDto::ListenConflict {
                    holder_id: SECOND_ID.to_owned()
                }),
                Some(ImportConflictDto::DuplicateInImport),
            ]
        );
    }

    #[test]
    fn should_convert_an_export_with_its_count_as_a_number() {
        let result = export_to_result(ConfigExport {
            contents: "format = \"tunneler-export\"\n".to_owned(),
            tunnel_count: 3,
        });

        assert_eq!(
            (result.contents.as_str(), result.tunnel_count),
            ("format = \"tunneler-export\"\n", 3)
        );
    }

    #[test]
    fn should_carry_both_filters_when_converting_the_export_parameters() {
        let filter = export_filter_from_params(ConfigExportParams {
            hosts: vec!["nova".to_owned()],
            tags: vec!["dev".to_owned()],
        });

        assert_eq!(
            (filter.hosts, filter.tags),
            (vec!["nova".to_owned()], vec!["dev".to_owned()])
        );
    }

    fn data_of(error: &ImportFileError) -> serde_json::Value {
        let ipc_error = invalid_import_file_to_ipc(error);
        assert_eq!(ipc_error.code, ErrorCode::InvalidImportFile);
        ipc_error.data.unwrap()
    }

    #[test]
    fn should_give_the_line_and_the_message_when_the_file_has_a_syntax_error() {
        let error = ImportFileError::Syntax {
            line: Some(3),
            message: "expected `=`".to_owned(),
        };

        assert_eq!(
            data_of(&error),
            json!({ "reason": "syntax", "line": 3, "message": "expected `=`" })
        );
    }

    #[test]
    fn should_give_only_the_reason_when_the_file_is_not_a_tunneler_file() {
        assert_eq!(
            data_of(&ImportFileError::NotTunnelerFile),
            json!({ "reason": "notTunnelerFile" })
        );
    }

    #[test]
    fn should_give_the_found_and_supported_versions_when_the_file_is_too_new() {
        let error = ImportFileError::NewerVersion {
            found: 2,
            supported: 1,
        };

        assert_eq!(
            data_of(&error),
            json!({ "reason": "newerVersion", "found": 2, "supported": 1 })
        );
    }

    #[test]
    fn should_give_the_entry_and_the_field_but_no_value_when_characters_are_unsafe() {
        let error = ImportFileError::UnsafeCharacters {
            entry: 4,
            field: "host",
        };

        assert_eq!(
            data_of(&error),
            json!({ "reason": "unsafeCharacters", "entry": 4, "field": "host" })
        );
    }

    #[test]
    fn should_give_the_byte_limit_when_the_file_is_too_large() {
        let error = ImportFileError::TooLarge {
            found: 300_000,
            limit: 262_144,
        };

        assert_eq!(
            data_of(&error),
            json!({ "reason": "tooLarge", "limit": 262_144 })
        );
    }

    #[test]
    fn should_give_the_entry_limit_when_the_file_has_too_many_tunnels() {
        let error = ImportFileError::TooManyEntries {
            found: 1001,
            limit: 1000,
        };

        assert_eq!(
            data_of(&error),
            json!({ "reason": "tooManyEntries", "limit": 1000 })
        );
    }
}
