//! Reading the output of `ds ls --json`.
//!
//! The reading is lenient: a key it does not know is ignored, and an older
//! `ds` that lacks `project`, `app_url`, `worktree` or `forced` still parses.
//! Every string is cleaned, see [`sanitize_text`].

use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;

use super::host_name::host_name_of_url;
use super::listing::{DevslotInstance, DevslotPort, DevslotPortStatus, DevslotWarning};
use super::project_tag::devslot_project_tag;
use super::sanitize::{MAX_PATH_BYTES, MAX_TEXT_BYTES, sanitize_text};
use crate::store::find_tag_problem;

const FIRST_SUPPORTED_VERSION: u64 = 1;

/// The most instances a listing may hold; a larger one is refused, as a
/// longer list is no longer usable in a window.
pub const MAX_DEVSLOT_INSTANCES: usize = 256;

/// The most ports an instance may list; a larger listing is refused.
pub const MAX_DEVSLOT_PORTS_PER_INSTANCE: usize = 32;

/// The most ports a listing may hold in all its instances; a larger listing
/// is refused.
pub const MAX_DEVSLOT_PORTS: usize = 1024;

/// Why the output of `ds ls --json` cannot be used.
#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum ListingError {
    /// The output is not JSON of the expected shape.
    #[error("the output is not a valid listing: {0}")]
    Malformed(String),
    /// The output has no `version`, or it is below 1.
    #[error("the output has no version, or a version below {FIRST_SUPPORTED_VERSION}")]
    UnsupportedVersion,
    /// The listing holds more instances than [`MAX_DEVSLOT_INSTANCES`].
    #[error("the listing holds more than {MAX_DEVSLOT_INSTANCES} instances")]
    TooManyInstances,
    /// An instance lists more ports than [`MAX_DEVSLOT_PORTS_PER_INSTANCE`].
    #[error("an instance lists more than {MAX_DEVSLOT_PORTS_PER_INSTANCE} ports")]
    TooManyPorts,
    /// The listing holds more ports than [`MAX_DEVSLOT_PORTS`].
    #[error("the listing holds more than {MAX_DEVSLOT_PORTS} ports")]
    TooManyPortsInTotal,
}

/// The instances of a listing and what was left out.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ParsedListing {
    pub(crate) instances: Vec<DevslotInstance>,
    pub(crate) warnings: Vec<DevslotWarning>,
}

#[derive(Deserialize)]
struct RawListing {
    version: Option<u64>,
    #[serde(default)]
    instances: Vec<RawInstance>,
}

#[derive(Deserialize)]
struct RawInstance {
    name: String,
    project: Option<String>,
    path: String,
    worktree: Option<bool>,
    slot: u32,
    app_url: Option<String>,
    services_running: Option<bool>,
    #[serde(default)]
    ports: Vec<RawPort>,
}

#[derive(Deserialize)]
struct RawPort {
    #[serde(default)]
    var: String,
    #[serde(default)]
    port: Value,
    #[serde(default)]
    kind: String,
    url: Option<String>,
    service: Option<String>,
    target: Option<u64>,
    forced: Option<bool>,
}

/// Reads the output of `ds ls --json`.
pub(crate) fn parse_listing(output: &[u8]) -> Result<ParsedListing, ListingError> {
    let raw: RawListing = serde_json::from_slice(output).map_err(|error| {
        ListingError::Malformed(sanitize_text(&error.to_string(), MAX_TEXT_BYTES))
    })?;
    if raw
        .version
        .is_none_or(|version| version < FIRST_SUPPORTED_VERSION)
    {
        return Err(ListingError::UnsupportedVersion);
    }
    if raw.instances.len() > MAX_DEVSLOT_INSTANCES {
        return Err(ListingError::TooManyInstances);
    }
    if raw
        .instances
        .iter()
        .any(|instance| instance.ports.len() > MAX_DEVSLOT_PORTS_PER_INSTANCE)
    {
        return Err(ListingError::TooManyPorts);
    }
    let total_ports: usize = raw
        .instances
        .iter()
        .map(|instance| instance.ports.len())
        .sum();
    if total_ports > MAX_DEVSLOT_PORTS {
        return Err(ListingError::TooManyPortsInTotal);
    }
    let (instances, warnings): (Vec<_>, Vec<_>) =
        raw.instances.into_iter().map(convert_instance).unzip();
    Ok(ParsedListing {
        instances,
        warnings: warnings.into_iter().flatten().collect(),
    })
}

fn convert_instance(raw: RawInstance) -> (DevslotInstance, Vec<DevslotWarning>) {
    let name = sanitize_text(&raw.name, MAX_TEXT_BYTES);
    let project = clean_optional(raw.project).filter(|project| !project.is_empty());
    let project_tag = project
        .as_deref()
        .and_then(devslot_project_tag)
        .filter(|tag| find_tag_problem(tag).is_none());
    let mut warnings = Vec::new();
    if project.is_some() && project_tag.is_none() {
        warnings.push(DevslotWarning::ProjectWithoutTag {
            instance: name.clone(),
        });
    }
    let (ports, port_warnings): (Vec<_>, Vec<_>) = raw
        .ports
        .into_iter()
        .map(|port| convert_port(port, &name))
        .partition(Result::is_ok);
    warnings.extend(port_warnings.into_iter().filter_map(Result::err));
    let instance = DevslotInstance {
        name,
        project,
        project_tag,
        path: sanitize_text(&raw.path, MAX_PATH_BYTES),
        is_worktree: raw.worktree,
        slot: raw.slot,
        app_url: clean_optional(raw.app_url),
        services_running: raw.services_running,
        ports: ports.into_iter().filter_map(Result::ok).collect(),
    };
    (instance, warnings)
}

fn convert_port(raw: RawPort, instance: &str) -> Result<DevslotPort, DevslotWarning> {
    let var = sanitize_text(&raw.var, MAX_TEXT_BYTES);
    let Some(port) = valid_port(&raw.port) else {
        return Err(DevslotWarning::InvalidPort {
            instance: instance.to_owned(),
            var,
        });
    };
    let url = clean_optional(raw.url);
    let host_name = url.as_deref().and_then(host_name_of_url);
    Ok(DevslotPort {
        var,
        port,
        kind: sanitize_text(&raw.kind, MAX_TEXT_BYTES),
        url,
        host_name,
        service: clean_optional(raw.service),
        target: raw.target.and_then(|target| u16::try_from(target).ok()),
        is_forced: raw.forced.unwrap_or(false),
        status: DevslotPortStatus::New,
    })
}

fn valid_port(value: &Value) -> Option<u16> {
    let port = u16::try_from(value.as_u64()?).ok()?;
    (port >= 1).then_some(port)
}

fn clean_optional(text: Option<String>) -> Option<String> {
    text.map(|text| sanitize_text(&text, MAX_TEXT_BYTES))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    const FULL_EXAMPLE: &str = r#"{
      "version": 1,
      "shared": [],
      "instances": [
        {
          "name": "web-shop-feat-auth-3f2a",
          "project": "web-shop",
          "path": "/home/me/dev/web-shop.feat-auth",
          "worktree": true,
          "slot": 1,
          "host": "web-shop-feat-auth-3f2a.localhost",
          "app_url": "http://web-shop-feat-auth-3f2a.localhost:20050",
          "services_running": true,
          "ports": [
            { "var": "PORT", "port": 20050, "kind": "web", "url": "http://web-shop-feat-auth-3f2a.localhost:20050" },
            { "var": "VITE_HMR_PORT", "port": 20051, "kind": "web", "url": "http://localhost:20051" },
            { "var": "DB_PORT", "port": 20052, "kind": "service", "service": "postgres", "target": 5432 }
          ]
        }
      ]
    }"#;

    fn parse(text: &str) -> ParsedListing {
        parse_listing(text.as_bytes()).unwrap()
    }

    fn only_instance(text: &str) -> DevslotInstance {
        parse(text).instances.remove(0)
    }

    #[test]
    fn should_read_every_field_when_the_listing_is_the_contract_example() {
        let instance = only_instance(FULL_EXAMPLE);

        assert_eq!(
            (
                instance.name.as_str(),
                instance.project.as_deref(),
                instance.project_tag.as_deref(),
                instance.path.as_str(),
                instance.is_worktree,
                instance.slot,
                instance.app_url.as_deref(),
                instance.services_running,
                instance.ports.len()
            ),
            (
                "web-shop-feat-auth-3f2a",
                Some("web-shop"),
                Some("ds-web-shop"),
                "/home/me/dev/web-shop.feat-auth",
                Some(true),
                1,
                Some("http://web-shop-feat-auth-3f2a.localhost:20050"),
                Some(true),
                3
            )
        );
    }

    #[test]
    fn should_read_the_ports_when_the_listing_is_the_contract_example() {
        let ports = only_instance(FULL_EXAMPLE).ports;

        assert_eq!(
            (
                ports[0].host_name.as_deref(),
                ports[1].host_name.as_deref(),
                ports[2].service.as_deref(),
                ports[2].target,
                ports[2].kind.as_str(),
                ports[0].is_forced
            ),
            (
                Some("web-shop-feat-auth-3f2a.localhost"),
                None,
                Some("postgres"),
                Some(5432),
                "service",
                false
            )
        );
    }

    #[test]
    fn should_parse_an_older_output_when_it_lacks_the_recent_keys() {
        let text = r#"{"version":1,"instances":[{"name":"a","path":"/a","slot":2,
            "ports":[{"var":"PORT","port":20100,"kind":"web","url":"http://a.localhost:20100"}]}]}"#;

        let instance = only_instance(text);

        assert_eq!(
            (
                instance.project,
                instance.app_url,
                instance.is_worktree,
                instance.ports[0].is_forced
            ),
            (None, None, None, false)
        );
    }

    #[test]
    fn should_read_null_values_as_absent() {
        let text = r#"{"version":1,"instances":[{"name":"a","project":null,"path":"/a",
            "worktree":null,"slot":2,"app_url":null,"services_running":null,"ports":[]}]}"#;

        let instance = only_instance(text);

        assert_eq!(
            (
                instance.project,
                instance.is_worktree,
                instance.app_url,
                instance.services_running
            ),
            (None, None, None, None)
        );
    }

    #[test]
    fn should_ignore_the_keys_it_does_not_know() {
        let text = r#"{"version":1,"later":{"x":1},"instances":[{"name":"a","path":"/a","slot":2,
            "extra":true,"ports":[{"var":"P","port":20100,"kind":"web","other":1}]}]}"#;

        assert_eq!(parse(text).instances.len(), 1);
    }

    #[test]
    fn should_accept_a_later_version_when_it_only_adds_keys() {
        assert!(parse_listing(br#"{"version":2,"instances":[]}"#).is_ok());
    }

    #[test]
    fn should_reject_the_output_when_the_version_is_zero() {
        assert_eq!(
            parse_listing(br#"{"version":0,"instances":[]}"#),
            Err(ListingError::UnsupportedVersion)
        );
    }

    #[test]
    fn should_reject_the_output_when_the_version_is_missing() {
        assert_eq!(
            parse_listing(br#"{"instances":[]}"#),
            Err(ListingError::UnsupportedVersion)
        );
    }

    #[test]
    fn should_reject_the_output_when_it_is_not_json() {
        assert!(matches!(
            parse_listing(b"command not found"),
            Err(ListingError::Malformed(_))
        ));
    }

    #[test]
    fn should_skip_the_port_and_warn_when_its_number_is_out_of_range() {
        let text = r#"{"version":1,"instances":[{"name":"a","path":"/a","slot":2,"ports":[
            {"var":"ZERO","port":0,"kind":"web"},
            {"var":"BIG","port":70000,"kind":"web"},
            {"var":"TEXT","port":"80","kind":"web"},
            {"var":"OK","port":20100,"kind":"web"}]}]}"#;

        let parsed = parse(text);

        assert_eq!(
            (parsed.instances[0].ports.len(), parsed.warnings,),
            (
                1,
                vec![
                    DevslotWarning::InvalidPort {
                        instance: "a".to_owned(),
                        var: "ZERO".to_owned()
                    },
                    DevslotWarning::InvalidPort {
                        instance: "a".to_owned(),
                        var: "BIG".to_owned()
                    },
                    DevslotWarning::InvalidPort {
                        instance: "a".to_owned(),
                        var: "TEXT".to_owned()
                    },
                ]
            )
        );
    }

    #[test]
    fn should_cap_the_strings_when_they_are_oversized() {
        let long = "a".repeat(5000);
        let text = format!(
            r#"{{"version":1,"instances":[{{"name":"{long}","project":"{long}","path":"/{long}","slot":1,
            "app_url":"http://{long}","ports":[{{"var":"{long}","port":20100,"kind":"{long}","url":"http://{long}","service":"{long}"}}]}}]}}"#
        );

        let instance = only_instance(&text);

        assert_eq!(
            (
                instance.name.len(),
                instance.project.unwrap().len(),
                instance.path.len(),
                instance.app_url.unwrap().len(),
                instance.ports[0].var.len(),
                instance.ports[0].kind.len(),
                instance.ports[0].url.as_ref().unwrap().len(),
                instance.ports[0].service.as_ref().unwrap().len(),
            ),
            (256, 256, 1024, 256, 256, 256, 256, 256)
        );
    }

    #[test]
    fn should_strip_control_and_bidirectional_characters_from_every_string() {
        let text = "{\"version\":1,\"instances\":[{\"name\":\"a\\u0007b\",\"project\":\"p\\u202Eq\",\"path\":\"/a\\u0000\\u200Fb\",\"slot\":1,\
            \"ports\":[{\"var\":\"V\\nR\",\"port\":20100,\"kind\":\"we\\u061Cb\"}]}]}";

        let instance = only_instance(text);

        assert_eq!(
            (
                instance.name.as_str(),
                instance.project.as_deref(),
                instance.path.as_str(),
                instance.ports[0].var.as_str(),
                instance.ports[0].kind.as_str()
            ),
            ("ab", Some("pq"), "/ab", "VR", "web")
        );
    }

    #[test]
    fn should_give_the_project_tag_of_the_cleaned_project() {
        let text = r#"{"version":1,"instances":[{"name":"a","project":"my project","path":"/a","slot":1,"ports":[]}]}"#;

        assert_eq!(
            only_instance(text).project_tag,
            Some("ds-my-project".to_owned())
        );
    }

    #[test]
    fn should_give_no_project_when_it_is_an_empty_string() {
        let text = r#"{"version":1,"instances":[{"name":"a","project":"","path":"/a","slot":1,"ports":[]}]}"#;

        let parsed = parse(text);

        assert_eq!(
            (
                parsed.instances[0].project.clone(),
                parsed.instances[0].project_tag.clone(),
                parsed.warnings
            ),
            (None, None, Vec::new())
        );
    }

    #[test]
    fn should_mark_a_port_forced_when_the_listing_says_so() {
        let text = r#"{"version":1,"instances":[{"name":"a","path":"/a","slot":1,"ports":[
            {"var":"P","port":3000,"kind":"web","forced":true}]}]}"#;

        assert!(only_instance(text).ports[0].is_forced);
    }

    fn listing_with(instances: usize, ports: usize) -> String {
        let port_list: Vec<String> = (0..ports)
            .map(|index| {
                format!(
                    r#"{{"var":"P{index}","port":{},"kind":"web"}}"#,
                    20000 + index
                )
            })
            .collect();
        let instance_list: Vec<String> = (0..instances)
            .map(|index| {
                format!(
                    r#"{{"name":"i{index}","path":"/a","slot":1,"ports":[{}]}}"#,
                    port_list.join(",")
                )
            })
            .collect();
        format!(
            r#"{{"version":1,"instances":[{}]}}"#,
            instance_list.join(",")
        )
    }

    #[test]
    fn should_accept_the_listing_when_it_is_exactly_at_both_caps() {
        let parsed = parse(&listing_with(
            MAX_DEVSLOT_INSTANCES,
            MAX_DEVSLOT_PORTS / MAX_DEVSLOT_INSTANCES,
        ));

        assert_eq!(parsed.instances.len(), MAX_DEVSLOT_INSTANCES);
    }

    #[test]
    fn should_accept_an_instance_when_it_lists_exactly_the_cap_of_ports() {
        let parsed = parse(&listing_with(1, MAX_DEVSLOT_PORTS_PER_INSTANCE));

        assert_eq!(
            parsed.instances[0].ports.len(),
            MAX_DEVSLOT_PORTS_PER_INSTANCE
        );
    }

    #[test]
    fn should_refuse_the_listing_when_it_holds_more_ports_than_the_total_cap() {
        assert_eq!(
            parse_listing(
                listing_with(
                    MAX_DEVSLOT_INSTANCES,
                    MAX_DEVSLOT_PORTS / MAX_DEVSLOT_INSTANCES + 1
                )
                .as_bytes()
            ),
            Err(ListingError::TooManyPortsInTotal)
        );
    }

    #[test]
    fn should_refuse_the_listing_when_it_holds_more_instances_than_the_cap() {
        assert_eq!(
            parse_listing(listing_with(MAX_DEVSLOT_INSTANCES + 1, 1).as_bytes()),
            Err(ListingError::TooManyInstances)
        );
    }

    #[test]
    fn should_refuse_the_listing_when_an_instance_lists_more_ports_than_the_cap() {
        assert_eq!(
            parse_listing(listing_with(1, MAX_DEVSLOT_PORTS_PER_INSTANCE + 1).as_bytes()),
            Err(ListingError::TooManyPorts)
        );
    }
}
