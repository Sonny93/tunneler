//! The requests of the configuration transfer: export, import preview and
//! apply, and the classification of the routes of the other import sources.

use std::sync::Arc;

use engine::{ImportFileError, MAX_IMPORT_FILE_BYTES, MAX_IMPORT_TUNNELS};
use ipc::protocol::{
    ConfigExportParams, ConfigImportApplyParams, ConfigImportParams, ImportClassifyParams,
    InvalidImportFileDto, InvalidImportReasonDto, IpcError, MAX_CONFIG_IMPORT_BYTES,
    MAX_CONFIG_IMPORT_TUNNELS, MAX_LINE_BYTES, ResponsePayload, encode_response,
};

use super::{DaemonHandler, engine_error_to_ipc, run_blocking};
use crate::daemon::config_transfer_mapping::{
    conflicts_to_result, export_filter_from_params, export_to_result, import_route_from_dto,
    indexes_from_dto, invalid_import_file_to_ipc, plan_to_dto, report_to_dto,
};

const PLAN_TOO_LARGE_MESSAGE: &str =
    "the plan is too large to show: import a file with fewer tunnels or shorter hosts";
const EXPORT_TOO_LARGE_MESSAGE: &str = "the export is larger than 256 KiB; filter by host or tag";

const _: () = assert!(
    MAX_CONFIG_IMPORT_BYTES == MAX_IMPORT_FILE_BYTES,
    "the byte caps of the wire and of the engine must match"
);
const _: () = assert!(
    MAX_CONFIG_IMPORT_TUNNELS == MAX_IMPORT_TUNNELS,
    "the tunnel caps of the wire and of the engine must match"
);

impl DaemonHandler {
    pub(super) fn classify_imports(&self, params: ImportClassifyParams) -> ResponsePayload {
        let routes: Vec<_> = params
            .candidates
            .into_iter()
            .map(import_route_from_dto)
            .collect();
        let conflicts = self.engine.classify_import_routes(&routes);
        ResponsePayload::ImportClassify(conflicts_to_result(&conflicts))
    }

    pub(super) fn export_config(
        &self,
        params: ConfigExportParams,
    ) -> Result<ResponsePayload, IpcError> {
        let export = self
            .engine
            .export_configuration(&export_filter_from_params(params));
        ensure_export_within_cap(&export.contents)?;
        ensure_fits_in_line(
            ResponsePayload::ConfigExport(export_to_result(export)),
            MAX_LINE_BYTES,
            || IpcError::validation_failed(&[EXPORT_TOO_LARGE_MESSAGE.to_owned()]),
        )
    }

    pub(super) async fn preview_config_import(
        &self,
        params: ConfigImportParams,
    ) -> Result<ResponsePayload, IpcError> {
        ensure_within_size_cap(&params.contents)?;
        let known_hosts = self.list_known_hosts().await?;
        let plan = self
            .engine
            .preview_configuration_import(&params.contents, &params.host_map, &known_hosts)
            .await
            .map_err(engine_error_to_ipc)?;
        ensure_fits_in_line(
            ResponsePayload::ConfigImportPlan(plan_to_dto(&plan)),
            MAX_LINE_BYTES,
            plan_too_large,
        )
    }

    pub(super) async fn apply_config_import(
        &self,
        params: ConfigImportApplyParams,
    ) -> Result<ResponsePayload, IpcError> {
        ensure_within_size_cap(&params.contents)?;
        let known_hosts = self.list_known_hosts().await?;
        let report = self
            .engine
            .apply_configuration_import(
                &params.contents,
                &params.host_map,
                &known_hosts,
                &indexes_from_dto(&params.indexes),
            )
            .await
            .map_err(engine_error_to_ipc)?;
        ensure_fits_in_line(
            ResponsePayload::ConfigImportReport(report_to_dto(&report)),
            MAX_LINE_BYTES,
            plan_too_large,
        )
    }

    /// The aliases `hosts.list` gives, read off the async runtime; nothing is
    /// resolved.
    async fn list_known_hosts(&self) -> Result<Vec<String>, IpcError> {
        let hosts = Arc::clone(&self.hosts);
        run_blocking(move || hosts.list_hosts())
            .await?
            .map_err(|error| IpcError::internal(error.to_string()))
    }
}

fn ensure_within_size_cap(contents: &str) -> Result<(), IpcError> {
    if contents.len() <= MAX_IMPORT_FILE_BYTES {
        return Ok(());
    }
    Err(invalid_import_file_to_ipc(&ImportFileError::TooLarge {
        found: contents.len(),
        limit: MAX_IMPORT_FILE_BYTES,
    }))
}

fn ensure_export_within_cap(contents: &str) -> Result<(), IpcError> {
    if contents.len() <= MAX_IMPORT_FILE_BYTES {
        return Ok(());
    }
    Err(IpcError::validation_failed(&[
        EXPORT_TOO_LARGE_MESSAGE.to_owned()
    ]))
}

fn plan_too_large() -> IpcError {
    IpcError::invalid_import_file(
        PLAN_TOO_LARGE_MESSAGE,
        &InvalidImportFileDto::new(InvalidImportReasonDto::TooLarge),
    )
}

/// Returns the payload when its response line stays under `max_line_bytes`,
/// and the error otherwise: the client would reject a longer line and fail
/// every pending call of the connection.
fn ensure_fits_in_line(
    payload: ResponsePayload,
    max_line_bytes: usize,
    too_large: impl FnOnce() -> IpcError,
) -> Result<ResponsePayload, IpcError> {
    let response = Ok(payload);
    let line = encode_response(Some(u64::MAX), &response)
        .map_err(|error| IpcError::internal(error.to_string()))?;
    if line.len() >= max_line_bytes {
        return Err(too_large());
    }
    response
}

#[cfg(test)]
mod tests {
    use ipc::protocol::{ConfigExportResult, ErrorCode};

    use super::*;

    fn export_payload(contents: &str) -> ResponsePayload {
        ResponsePayload::ConfigExport(ConfigExportResult {
            contents: contents.to_owned(),
            tunnel_count: 1,
        })
    }

    #[test]
    fn should_answer_too_large_as_an_invalid_import_file_when_the_plan_line_reaches_the_cap() {
        let error =
            ensure_fits_in_line(export_payload(&"a".repeat(100)), 50, plan_too_large).unwrap_err();

        assert_eq!(
            (error.code, error.data),
            (
                ErrorCode::InvalidImportFile,
                Some(serde_json::json!({ "reason": "tooLarge" }))
            )
        );
    }

    #[test]
    fn should_keep_the_payload_when_its_line_stays_under_the_cap() {
        let payload = export_payload("a");

        let kept = ensure_fits_in_line(payload.clone(), MAX_LINE_BYTES, plan_too_large).unwrap();

        assert_eq!(kept, payload);
    }

    #[test]
    fn should_answer_validation_failed_naming_the_filters_when_the_export_exceeds_the_cap() {
        let error = ensure_export_within_cap(&"a".repeat(MAX_IMPORT_FILE_BYTES + 1)).unwrap_err();

        assert_eq!(error.code, ErrorCode::ValidationFailed);
        assert_eq!(
            error.data,
            Some(serde_json::json!([EXPORT_TOO_LARGE_MESSAGE]))
        );
    }

    #[test]
    fn should_accept_an_export_exactly_at_the_cap() {
        assert!(ensure_export_within_cap(&"a".repeat(MAX_IMPORT_FILE_BYTES)).is_ok());
    }
}
