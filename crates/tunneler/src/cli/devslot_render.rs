//! How the result of `tunneler devslot` is shown: one line per port, then
//! what was opened and what the user should look at.

use ipc::protocol::{
    DevslotFailedPortDto, DevslotRemovedRouteDto, DevslotSkipReasonDto, DevslotSkippedPortDto,
    DevslotUpPortDto, DevslotUpResultDto,
};

use super::style::{Style, Tone};

const LISTEN_ADDRESS: &str = "127.0.0.1";

/// The result as one line of JSON.
pub fn render_devslot_up_json(result: &DevslotUpResultDto) -> Result<String, serde_json::Error> {
    serde_json::to_string(result)
}

/// The line shown before the call, so that the wait is not silent.
pub fn describe_devslot_start(instance: &str, host: &str, style: Style) -> String {
    format!("Opening {instance} on {host}{}", style.ellipsis())
}

/// The result as lines of text; `is_open_requested` tells whether the browser
/// was asked for, so that a missing address can be explained.
pub fn render_devslot_up(
    result: &DevslotUpResultDto,
    is_open_requested: bool,
    style: Style,
) -> String {
    let mut lines = Vec::new();
    lines.extend(
        result
            .removed
            .iter()
            .map(|route| describe_removed_route(route, style)),
    );
    lines.extend(
        result
            .started
            .iter()
            .map(|port| describe_started_port(port, style)),
    );
    lines.extend(
        result
            .existing
            .iter()
            .map(|port| describe_existing_port(port, style)),
    );
    lines.extend(
        result
            .skipped
            .iter()
            .map(|port| describe_skipped_port(port, style)),
    );
    lines.extend(
        result
            .failed
            .iter()
            .map(|port| describe_failed_port(port, style)),
    );
    if is_without_web_port(result) {
        lines.push(format!(
            "{} lists no web port: nothing to tunnel",
            result.instance
        ));
    }
    lines.extend(describe_opening(result, is_open_requested));
    lines.extend(describe_review_hint(result));
    lines.extend(describe_shared_warning(result, style));
    lines.join("\n")
}

fn is_without_web_port(result: &DevslotUpResultDto) -> bool {
    result.started.is_empty()
        && result.existing.is_empty()
        && result.skipped.is_empty()
        && result.failed.is_empty()
}

fn describe_route(remote_port: u16, listen_port: u16, style: Style) -> String {
    format!(
        "{remote_port} {} {LISTEN_ADDRESS}:{listen_port}",
        style.arrow()
    )
}

fn describe_removed_route(route: &DevslotRemovedRouteDto, style: Style) -> String {
    let kind = if route.is_temporary {
        "temporary"
    } else {
        "saved"
    };
    format!(
        "{}: removed, it was stale ({kind})",
        describe_route(route.remote_port, route.listen_port, style)
    )
}

fn describe_started_port(port: &DevslotUpPortDto, style: Style) -> String {
    let route = describe_route(port.remote_port, port.listen_port, style);
    let started = style.paint(Tone::Good, "started");
    if port.remote_port == port.listen_port {
        return format!("{route}: {started}");
    }
    format!(
        "{route}: {started} (port {} is in use on this computer)",
        port.remote_port
    )
}

fn describe_existing_port(port: &DevslotUpPortDto, style: Style) -> String {
    format!(
        "{}: {}",
        describe_route(port.remote_port, port.listen_port, style),
        style.paint(Tone::Good, "already set up")
    )
}

fn describe_skipped_port(port: &DevslotSkippedPortDto, style: Style) -> String {
    let reason = match port.reason {
        DevslotSkipReasonDto::LocalPortBusy => {
            "it is in use on this computer and no free port was found"
        }
        DevslotSkipReasonDto::NeedsReview => "a stale tunnel holds it and needs your decision",
        DevslotSkipReasonDto::PrivilegedPort => "ports below 1024 are not tunneled here",
        DevslotSkipReasonDto::ExistingTunnel => {
            "a tunnel you configured already forwards it: start it yourself"
        }
    };
    format!(
        "port {}: {}, {reason}",
        port.remote_port,
        style.paint(Tone::Warning, "skipped")
    )
}

fn describe_failed_port(port: &DevslotFailedPortDto, style: Style) -> String {
    format!(
        "port {}: {}: {}",
        port.remote_port,
        style.paint(Tone::Bad, "failed"),
        port.message
    )
}

fn describe_opening(result: &DevslotUpResultDto, is_open_requested: bool) -> Option<String> {
    if let Some(address) = &result.opened {
        return Some(format!("Opening {address}"));
    }
    if let Some(message) = &result.open_error {
        return Some(format!("Cannot open the browser: {message}"));
    }
    is_open_requested.then(|| "Nothing opened: no application port is up".to_owned())
}

fn describe_review_hint(result: &DevslotUpResultDto) -> Option<String> {
    (result.needs_review > 0).then(|| {
        format!(
            "{} stale tunnel(s) need your decision: open the Tunneler window and review them in the import panel (From devslot)",
            result.needs_review
        )
    })
}

fn describe_shared_warning(result: &DevslotUpResultDto, style: Style) -> Option<String> {
    result.app_port_shared.then(|| {
        format!(
            "{}: another instance lists the port of the application, so the browser may reach that one instead of {}",
            style.paint(Tone::Warning, "warning"),
            result.instance
        )
    })
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    const UNICODE: Style = Style {
        color: false,
        unicode: true,
    };
    const ASCII: Style = Style {
        color: false,
        unicode: false,
    };
    const COLORED: Style = Style {
        color: true,
        unicode: true,
    };

    fn up_port(remote_port: u16, listen_port: u16) -> DevslotUpPortDto {
        DevslotUpPortDto {
            remote_port,
            listen_port,
            tunnel_id: "01J9ZK3Q8M5N7R2T4V6X8Y0ABC".to_owned(),
        }
    }

    fn empty_result() -> DevslotUpResultDto {
        DevslotUpResultDto {
            host: "vega".to_owned(),
            instance: "web-shop-098a".to_owned(),
            started: Vec::new(),
            existing: Vec::new(),
            skipped: Vec::new(),
            failed: Vec::new(),
            removed: Vec::new(),
            needs_review: 0,
            app_url: None,
            app_port_shared: false,
            opened: None,
            open_error: None,
        }
    }

    fn result_with_started(opened: Option<&str>) -> DevslotUpResultDto {
        DevslotUpResultDto {
            started: vec![up_port(20050, 20050)],
            opened: opened.map(str::to_owned),
            ..empty_result()
        }
    }

    #[test]
    fn should_announce_the_instance_and_the_host_with_an_ellipsis() {
        assert_eq!(
            describe_devslot_start("web-shop-098a", "vega", UNICODE),
            "Opening web-shop-098a on vega…"
        );
    }

    #[test]
    fn should_announce_with_three_dots_when_the_terminal_has_no_unicode() {
        assert_eq!(
            describe_devslot_start("web-shop-098a", "vega", ASCII),
            "Opening web-shop-098a on vega..."
        );
    }

    #[test]
    fn should_show_a_started_port_as_its_route_and_the_word_started() {
        assert_eq!(
            render_devslot_up(&result_with_started(None), false, UNICODE),
            "20050 → 127.0.0.1:20050: started"
        );
    }

    #[test]
    fn should_use_an_ascii_arrow_when_the_terminal_has_no_unicode() {
        assert_eq!(
            render_devslot_up(&result_with_started(None), false, ASCII),
            "20050 -> 127.0.0.1:20050: started"
        );
    }

    #[test]
    fn should_say_that_the_local_port_differs_when_it_is_not_the_remote_one() {
        let result = DevslotUpResultDto {
            started: vec![up_port(20050, 30050)],
            ..empty_result()
        };

        assert_eq!(
            render_devslot_up(&result, false, UNICODE),
            "20050 → 127.0.0.1:30050: started (port 20050 is in use on this computer)"
        );
    }

    #[test]
    fn should_show_a_port_that_already_had_a_tunnel_as_already_set_up() {
        let result = DevslotUpResultDto {
            existing: vec![up_port(20051, 20051)],
            ..empty_result()
        };

        assert_eq!(
            render_devslot_up(&result, false, UNICODE),
            "20051 → 127.0.0.1:20051: already set up"
        );
    }

    #[test]
    fn should_give_the_reason_of_each_skipped_port() {
        let result = DevslotUpResultDto {
            skipped: vec![
                DevslotSkippedPortDto {
                    remote_port: 20052,
                    reason: DevslotSkipReasonDto::LocalPortBusy,
                },
                DevslotSkippedPortDto {
                    remote_port: 20053,
                    reason: DevslotSkipReasonDto::NeedsReview,
                },
                DevslotSkippedPortDto {
                    remote_port: 80,
                    reason: DevslotSkipReasonDto::PrivilegedPort,
                },
            ],
            ..empty_result()
        };

        assert_eq!(
            render_devslot_up(&result, false, UNICODE),
            "port 20052: skipped, it is in use on this computer and no free port was found\n\
             port 20053: skipped, a stale tunnel holds it and needs your decision\n\
             port 80: skipped, ports below 1024 are not tunneled here"
        );
    }

    #[test]
    fn should_show_a_failed_port_with_its_message() {
        let result = DevslotUpResultDto {
            failed: vec![DevslotFailedPortDto {
                remote_port: 20054,
                message: "the tunnel failed to start".to_owned(),
            }],
            ..empty_result()
        };

        assert_eq!(
            render_devslot_up(&result, false, UNICODE),
            "port 20054: failed: the tunnel failed to start"
        );
    }

    #[test]
    fn should_show_a_removed_stale_route_first() {
        let result = DevslotUpResultDto {
            removed: vec![DevslotRemovedRouteDto {
                listen_port: 20999,
                remote_port: 20999,
                is_temporary: true,
            }],
            ..result_with_started(None)
        };

        assert_eq!(
            render_devslot_up(&result, false, UNICODE),
            "20999 → 127.0.0.1:20999: removed, it was stale (temporary)\n\
             20050 → 127.0.0.1:20050: started"
        );
    }

    #[test]
    fn should_say_what_is_opened_when_the_browser_was_launched() {
        assert_eq!(
            render_devslot_up(
                &result_with_started(Some("http://a.localhost:20050/")),
                true,
                UNICODE
            ),
            "20050 → 127.0.0.1:20050: started\nOpening http://a.localhost:20050/"
        );
    }

    #[test]
    fn should_say_why_nothing_opened_when_the_browser_could_not_be_launched() {
        let result = DevslotUpResultDto {
            open_error: Some("no browser".to_owned()),
            ..result_with_started(None)
        };

        assert_eq!(
            render_devslot_up(&result, true, UNICODE),
            "20050 → 127.0.0.1:20050: started\nCannot open the browser: no browser"
        );
    }

    #[test]
    fn should_say_that_nothing_opened_when_no_application_port_is_up() {
        assert_eq!(
            render_devslot_up(&result_with_started(None), true, UNICODE),
            "20050 → 127.0.0.1:20050: started\nNothing opened: no application port is up"
        );
    }

    #[test]
    fn should_say_nothing_about_opening_when_the_browser_was_not_asked_for() {
        assert!(!render_devslot_up(&result_with_started(None), false, UNICODE).contains("open"));
    }

    #[test]
    fn should_point_to_the_import_panel_when_stale_tunnels_wait_for_a_decision() {
        let result = DevslotUpResultDto {
            needs_review: 2,
            ..result_with_started(None)
        };

        assert!(
            render_devslot_up(&result, false, UNICODE)
                .ends_with("2 stale tunnel(s) need your decision: open the Tunneler window and review them in the import panel (From devslot)")
        );
    }

    #[test]
    fn should_warn_that_the_browser_may_reach_another_instance_when_the_app_port_is_shared() {
        let result = DevslotUpResultDto {
            app_port_shared: true,
            ..result_with_started(None)
        };

        assert!(render_devslot_up(&result, false, UNICODE).ends_with(
            "warning: another instance lists the port of the application, so the browser may reach that one instead of web-shop-098a"
        ));
    }

    #[test]
    fn should_say_there_is_nothing_to_tunnel_when_the_instance_lists_no_web_port() {
        assert_eq!(
            render_devslot_up(&empty_result(), false, UNICODE),
            "web-shop-098a lists no web port: nothing to tunnel"
        );
    }

    #[test]
    fn should_paint_the_word_started_when_color_is_on() {
        let colored = render_devslot_up(&result_with_started(None), false, COLORED);

        assert!(colored.contains("\x1b[32mstarted\x1b[0m"));
    }

    #[test]
    fn should_render_the_json_as_one_line_with_camel_case_keys() {
        let line = render_devslot_up_json(&result_with_started(None)).unwrap();

        assert!(!line.contains('\n') && line.contains("\"appPortShared\":false"));
    }
}
