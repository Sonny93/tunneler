use std::fs::{File, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::Path;

use ipc::ClientError;
use ipc::protocol::{ConfigExportParams, ConfigExportResult, ErrorCode};
use serde::Serialize;

use super::super::args::ExportArguments;
use super::super::import_render::count_tunnels;
use super::super::session::{ConnectionMode, connect};
use super::super::{CliError, Context};
use crate::logging;

const EXPORT_REFUSED_MESSAGE: &str = "the daemon refused the export";

#[cfg(unix)]
const OWNER_ONLY_FILE_MODE: u32 = 0o600;

/// `tunneler export`: the saved tunnels as a configuration file, on the
/// standard output or in a file.
pub async fn export(context: &Context, arguments: &ExportArguments) -> Result<(), CliError> {
    logging::init_cli_logging();
    let client = connect(&context.endpoint, ConnectionMode::default()).await?;
    let result = client
        .export_config(ConfigExportParams {
            hosts: arguments.hosts.clone(),
            tags: arguments.tags.clone(),
        })
        .await
        .map_err(describe_export_failure)?;
    match &arguments.output {
        Some(path) => {
            write_export_file(path, &result.contents, arguments.force)?;
            println!("{}", render_file_written(path, &result, arguments.json)?);
        }
        None => print_export(&result, arguments.json)?,
    }
    Ok(())
}

fn describe_export_failure(error: ClientError) -> CliError {
    match error {
        ClientError::Remote {
            code: ErrorCode::ValidationFailed,
            data,
            ..
        } => CliError::ExportTooLarge {
            message: first_message(data.as_ref()),
        },
        other => CliError::Client(other),
    }
}

fn first_message(data: Option<&serde_json::Value>) -> String {
    data.and_then(|messages| messages.get(0))
        .and_then(serde_json::Value::as_str)
        .unwrap_or(EXPORT_REFUSED_MESSAGE)
        .to_owned()
}

fn print_export(result: &ConfigExportResult, is_json: bool) -> Result<(), CliError> {
    if is_json {
        println!("{}", serde_json::to_string(result)?);
        return Ok(());
    }
    print!("{}", result.contents);
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FileWrittenJson {
    path: String,
    tunnel_count: u64,
}

fn render_file_written(
    path: &Path,
    result: &ConfigExportResult,
    is_json: bool,
) -> Result<String, CliError> {
    if is_json {
        return Ok(serde_json::to_string(&FileWrittenJson {
            path: path.display().to_string(),
            tunnel_count: result.tunnel_count,
        })?);
    }
    let count = usize::try_from(result.tunnel_count).unwrap_or(usize::MAX);
    Ok(format!(
        "Exported {} to {}",
        count_tunnels(count),
        path.display()
    ))
}

/// Writes the file. Without `is_forced` the file must not exist, and a
/// symbolic link in its place is refused as well.
fn write_export_file(path: &Path, contents: &str, is_forced: bool) -> Result<(), CliError> {
    let mut file = open_output_file(path, is_forced)?;
    file.write_all(contents.as_bytes())
        .map_err(|source| CliError::OutputWrite {
            path: path.to_owned(),
            source,
        })
}

fn open_output_file(path: &Path, is_forced: bool) -> Result<File, CliError> {
    let mut options = OpenOptions::new();
    options.write(true);
    if is_forced {
        options.create(true).truncate(true);
    } else {
        options.create_new(true);
    }
    restrict_to_owner(&mut options);
    options.open(path).map_err(|source| {
        if source.kind() == ErrorKind::AlreadyExists {
            return CliError::OutputExists {
                path: path.to_owned(),
            };
        }
        CliError::OutputWrite {
            path: path.to_owned(),
            source,
        }
    })
}

#[cfg(unix)]
fn restrict_to_owner(options: &mut OpenOptions) {
    use std::os::unix::fs::OpenOptionsExt;

    options.mode(OWNER_ONLY_FILE_MODE);
}

#[cfg(not(unix))]
fn restrict_to_owner(_options: &mut OpenOptions) {}

#[cfg(test)]
mod tests {
    use super::*;

    fn result_of(count: u64) -> ConfigExportResult {
        ConfigExportResult {
            contents: "format = \"tunneler-export\"\n".to_owned(),
            tunnel_count: count,
        }
    }

    #[test]
    fn should_create_the_file_with_the_contents_when_it_does_not_exist() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("backup.toml");

        write_export_file(&path, "a = 1\n", false).unwrap();

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "a = 1\n");
    }

    #[test]
    fn should_refuse_to_replace_an_existing_file_without_force() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("backup.toml");
        std::fs::write(&path, "precious").unwrap();

        let outcome = write_export_file(&path, "a = 1\n", false);

        assert!(matches!(outcome, Err(CliError::OutputExists { .. })));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "precious");
    }

    #[test]
    fn should_replace_an_existing_file_when_force_is_given() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("backup.toml");
        std::fs::write(&path, "a much longer previous content").unwrap();

        write_export_file(&path, "a = 1\n", true).unwrap();

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "a = 1\n");
    }

    #[cfg(unix)]
    #[test]
    fn should_refuse_to_write_through_a_symbolic_link_without_force() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("target.toml");
        std::fs::write(&target, "precious").unwrap();
        let link = directory.path().join("link.toml");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        let outcome = write_export_file(&link, "a = 1\n", false);

        assert!(matches!(outcome, Err(CliError::OutputExists { .. })));
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "precious");
    }

    #[cfg(unix)]
    #[test]
    fn should_make_the_new_file_readable_by_its_owner_only() {
        use std::os::unix::fs::PermissionsExt;

        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("backup.toml");

        write_export_file(&path, "a = 1\n", false).unwrap();

        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, OWNER_ONLY_FILE_MODE);
    }

    #[test]
    fn should_exit_with_1_and_suggest_filters_when_the_daemon_refuses_a_too_large_export() {
        let error = describe_export_failure(ClientError::Remote {
            code: ErrorCode::ValidationFailed,
            message: "the submitted values are invalid".to_owned(),
            data: Some(serde_json::json!([
                "the export is larger than 256 KiB; filter by host or tag"
            ])),
        });

        assert_eq!(error.exit_code(), 1);
        assert_eq!(
            error.to_string(),
            "the export is larger than 256 KiB; filter by host or tag"
        );
        assert!(error.hint().unwrap().contains("--host or --tag"));
    }

    #[test]
    fn should_say_how_many_tunnels_went_to_which_file() {
        let text = render_file_written(Path::new("backup.toml"), &result_of(3), false).unwrap();

        assert_eq!(text, "Exported 3 tunnels to backup.toml");
    }

    #[test]
    fn should_print_the_path_and_the_count_as_json_when_a_file_is_written() {
        let text = render_file_written(Path::new("backup.toml"), &result_of(1), true).unwrap();

        assert_eq!(text, r#"{"path":"backup.toml","tunnelCount":1}"#);
    }
}
