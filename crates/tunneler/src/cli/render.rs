//! Pure rendering of tunnels as text and JSON.
//!
//! One tunnel per line, no box drawing, and every state is a symbol plus a
//! word, so nothing depends on color. The JSON forms are the IPC data
//! transfer objects, printed compactly on a single line.

use ipc::protocol::{
    FailureReasonDto, MetricsDto, SettingsDto, SnapshotDto, TargetHealthDto, TunnelDefinitionDto,
    TunnelDto, TunnelKindDto, TunnelStateDto,
};
use serde::Serialize;

use super::style::{Style, Tone};

const DEFAULT_TARGET_HOST: &str = "127.0.0.1";
const SECONDS_PER_MINUTE: u64 = 60;
const SECONDS_PER_HOUR: u64 = 3600;
const SECONDS_PER_DAY: u64 = 86_400;
const MILLISECONDS_PER_SECOND: u64 = 1000;
const BYTES_PER_UNIT: u64 = 1024;
const BYTE_UNITS: [&str; 4] = ["KiB", "MiB", "GiB", "TiB"];
const NO_TUNNELS_MESSAGE: &str = "No tunnels configured.";

/// The reason of a failure, as a sentence fragment.
pub fn describe_failure(reason: &FailureReasonDto) -> String {
    match reason {
        FailureReasonDto::PortInUse { address } => format!("port already in use on {address}"),
        FailureReasonDto::ListenFailed { address, message } => {
            format!("cannot listen on {address}: {message}")
        }
        FailureReasonDto::Connect { message } => message.clone(),
        FailureReasonDto::UnsupportedKind { tunnel_kind } => {
            format!("{} tunnels are not supported yet", kind_word(*tunnel_kind))
        }
    }
}

const fn kind_word(kind: TunnelKindDto) -> &'static str {
    match kind {
        TunnelKindDto::Local => "local",
        TunnelKindDto::Remote => "remote",
        TunnelKindDto::Dynamic => "dynamic",
    }
}

struct StateLabel {
    symbol: &'static str,
    words: String,
    tone: Tone,
}

fn state_label(state: &TunnelStateDto, style: Style) -> StateLabel {
    let (unicode_symbol, ascii_symbol, words, tone) = match state {
        TunnelStateDto::Active => ("✔", "[ok]", "active".to_owned(), Tone::Good),
        TunnelStateDto::Reconnecting { attempt, .. } => (
            "↻",
            "[~]",
            format!("reconnecting (attempt {attempt})"),
            Tone::Warning,
        ),
        TunnelStateDto::Starting => ("…", "[..]", "starting".to_owned(), Tone::Warning),
        TunnelStateDto::Stopped => ("○", "[ ]", "stopped".to_owned(), Tone::Muted),
        TunnelStateDto::Failed { reason } => (
            "✖",
            "[x]",
            format!("failed: {}", describe_failure(reason)),
            Tone::Bad,
        ),
    };
    StateLabel {
        symbol: if style.unicode {
            unicode_symbol
        } else {
            ascii_symbol
        },
        words,
        tone,
    }
}

/// The symbol and the words of a state, such as `✔ active` or `[x] failed: …`.
pub fn describe_state(state: &TunnelStateDto, style: Style) -> String {
    let label = state_label(state, style);
    style.paint(label.tone, &format!("{} {}", label.symbol, label.words))
}

/// A time of day in UTC, `HH:MM:SS`.
pub fn format_clock_time(epoch_milliseconds: u64) -> String {
    let seconds_of_day = epoch_milliseconds / MILLISECONDS_PER_SECOND % SECONDS_PER_DAY;
    let hours = seconds_of_day / SECONDS_PER_HOUR;
    let minutes = seconds_of_day % SECONDS_PER_HOUR / SECONDS_PER_MINUTE;
    let seconds = seconds_of_day % SECONDS_PER_MINUTE;
    format!("{hours:02}:{minutes:02}:{seconds:02}")
}

/// A byte count with a binary unit, such as `1.5 KiB`.
pub fn format_bytes(bytes: u64) -> String {
    if bytes < BYTES_PER_UNIT {
        return format!("{bytes} B");
    }
    let mut amount = bytes_as_float(bytes) / bytes_as_float(BYTES_PER_UNIT);
    let mut unit = BYTE_UNITS[0];
    for next_unit in &BYTE_UNITS[1..] {
        if amount < bytes_as_float(BYTES_PER_UNIT) {
            break;
        }
        amount /= bytes_as_float(BYTES_PER_UNIT);
        unit = next_unit;
    }
    format!("{amount:.1} {unit}")
}

#[allow(
    clippy::cast_precision_loss,
    reason = "a byte count shown with one decimal does not need exact precision"
)]
const fn bytes_as_float(bytes: u64) -> f64 {
    bytes as f64
}

fn default_bind_address(kind: TunnelKindDto, settings: &SettingsDto) -> &str {
    match kind {
        TunnelKindDto::Local | TunnelKindDto::Dynamic => &settings.default_bind_address,
        TunnelKindDto::Remote => &settings.default_remote_bind_address,
    }
}

fn describe_listening_side(definition: &TunnelDefinitionDto, settings: &SettingsDto) -> String {
    let spec = &definition.spec;
    match &spec.listen_address {
        Some(address) if address != default_bind_address(spec.kind, settings) => {
            format!("{address}:{}", spec.listen_port)
        }
        _ => spec.listen_port.to_string(),
    }
}

/// Where a tunnel listens and what it reaches, such as `3333 → nova:3333`.
///
/// The bind address shows only when it differs from the default, and the
/// target host only when it is not `127.0.0.1`.
pub fn describe_route(
    definition: &TunnelDefinitionDto,
    settings: &SettingsDto,
    style: Style,
) -> String {
    let spec = &definition.spec;
    let listening = describe_listening_side(definition, settings);
    let arrow = style.arrow();
    let target_host = spec.target_host.as_deref().unwrap_or(DEFAULT_TARGET_HOST);
    let target_port = spec.target_port.unwrap_or(spec.listen_port);
    match spec.kind {
        TunnelKindDto::Local if target_host == DEFAULT_TARGET_HOST => {
            format!("{listening} {arrow} {}:{target_port}", spec.host)
        }
        TunnelKindDto::Local => format!(
            "{listening} {arrow} {target_host}:{target_port} via {}",
            spec.host
        ),
        TunnelKindDto::Remote => {
            format!(
                "{}:{listening} {arrow} {target_host}:{target_port}",
                spec.host
            )
        }
        TunnelKindDto::Dynamic => format!("{listening} {arrow} SOCKS via {}", spec.host),
    }
}

fn describe_tags(tags: &[String]) -> Option<String> {
    if tags.is_empty() {
        return None;
    }
    Some(format!("[{}]", tags.join(", ")))
}

fn describe_health(health: &TargetHealthDto, style: Style) -> Option<String> {
    let TargetHealthDto::Unreachable { since, .. } = health else {
        return None;
    };
    let text = format!(
        "{} target unreachable since {} UTC",
        style.separator(),
        format_clock_time(*since)
    );
    Some(style.paint(Tone::Warning, &text))
}

fn describe_metrics(metrics: &MetricsDto, style: Style) -> Option<String> {
    if metrics.total_connections == 0 {
        return None;
    }
    Some(format!(
        "{} {} open, {} total connections {} {} in, {} out",
        style.separator(),
        metrics.active_connections,
        metrics.total_connections,
        style.separator(),
        format_bytes(metrics.bytes_in),
        format_bytes(metrics.bytes_out),
    ))
}

fn render_tunnel_line(tunnel: &TunnelDto, settings: &SettingsDto, style: Style) -> String {
    let definition = &tunnel.definition;
    let mut parts = vec![
        describe_state(&tunnel.state, style),
        definition.spec.name.clone(),
        describe_route(definition, settings, style),
    ];
    parts.extend(describe_tags(&definition.spec.tags));
    let mut line = parts.join("  ");
    for suffix in [
        describe_health(&tunnel.health, style),
        describe_metrics(&tunnel.metrics, style),
    ]
    .into_iter()
    .flatten()
    {
        line.push(' ');
        line.push_str(&suffix);
    }
    line
}

/// One line per tunnel: its state, name, route, tags, target health and
/// traffic.
pub fn render_status(snapshot: &SnapshotDto, style: Style) -> String {
    if snapshot.tunnels.is_empty() {
        return NO_TUNNELS_MESSAGE.to_owned();
    }
    snapshot
        .tunnels
        .iter()
        .map(|tunnel| render_tunnel_line(tunnel, &snapshot.settings, style))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The line that opens the output of `status`.
pub fn render_daemon_summary(daemon_version: &str, snapshot: &SnapshotDto) -> String {
    let active = snapshot
        .tunnels
        .iter()
        .filter(|tunnel| tunnel.state == TunnelStateDto::Active)
        .count();
    format!(
        "Tunneler daemon {daemon_version}: {active} of {} tunnels active",
        snapshot.tunnels.len()
    )
}

/// One line per configured tunnel: name, kind, route and tags.
pub fn render_list(snapshot: &SnapshotDto, style: Style) -> String {
    if snapshot.tunnels.is_empty() {
        return NO_TUNNELS_MESSAGE.to_owned();
    }
    snapshot
        .tunnels
        .iter()
        .map(|tunnel| {
            let definition = &tunnel.definition;
            let mut parts = vec![
                definition.spec.name.clone(),
                kind_word(definition.spec.kind).to_owned(),
                describe_route(definition, &snapshot.settings, style),
            ];
            parts.extend(describe_tags(&definition.spec.tags));
            parts.join("  ")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StatusJson<'a> {
    daemon_version: &'a str,
    #[serde(flatten)]
    snapshot: &'a SnapshotDto,
}

#[derive(Serialize)]
struct ListJson<'a> {
    tunnels: Vec<&'a TunnelDefinitionDto>,
}

/// The `status --json` output: `{"daemonVersion", "tunnels": [TunnelDto…],
/// "settings": SettingsDto}` on a single line.
pub fn render_status_json(
    daemon_version: &str,
    snapshot: &SnapshotDto,
) -> Result<String, serde_json::Error> {
    serde_json::to_string(&StatusJson {
        daemon_version,
        snapshot,
    })
}

/// The `list --json` output: `{"tunnels": [TunnelDefinitionDto…]}` on a single
/// line.
pub fn render_list_json(snapshot: &SnapshotDto) -> Result<String, serde_json::Error> {
    serde_json::to_string(&ListJson {
        tunnels: snapshot
            .tunnels
            .iter()
            .map(|tunnel| &tunnel.definition)
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use ipc::protocol::TunnelSpecDto;
    use pretty_assertions::assert_eq;

    use super::*;

    const UNICODE_PLAIN: Style = Style {
        color: false,
        unicode: true,
    };
    const UNICODE_COLOR: Style = Style {
        color: true,
        unicode: true,
    };
    const ASCII_PLAIN: Style = Style {
        color: false,
        unicode: false,
    };
    const UNREACHABLE_SINCE: u64 = 52_995_000;

    fn settings() -> SettingsDto {
        SettingsDto {
            default_bind_address: "127.0.0.1".to_owned(),
            default_remote_bind_address: "localhost".to_owned(),
            keepalive_interval_ms: 15_000,
            keepalive_max: 3,
            session_idle_grace_ms: 30_000,
            reconnect_initial_delay_ms: 1000,
            reconnect_max_delay_ms: 60_000,
            health_probe: true,
            restore_on_launch: true,
            start_on_launch_tags: Vec::new(),
            launch_at_login: false,
            x11_display: None,
        }
    }

    fn tunnel(state: TunnelStateDto) -> TunnelDto {
        TunnelDto {
            definition: TunnelDefinitionDto {
                id: "01JA".to_owned(),
                spec: TunnelSpecDto {
                    name: "Adonis".to_owned(),
                    host: "nova".to_owned(),
                    kind: TunnelKindDto::Local,
                    listen_port: 3333,
                    listen_address: None,
                    target_host: None,
                    target_port: None,
                    tags: vec!["dev".to_owned(), "adonis".to_owned()],
                },
            },
            state,
            health: TargetHealthDto::Unknown,
            metrics: MetricsDto::default(),
        }
    }

    fn snapshot_of(tunnels: Vec<TunnelDto>) -> SnapshotDto {
        SnapshotDto {
            tunnels,
            settings: settings(),
        }
    }

    fn render_one(state: TunnelStateDto, style: Style) -> String {
        render_status(&snapshot_of(vec![tunnel(state)]), style)
    }

    fn failed(reason: FailureReasonDto) -> TunnelStateDto {
        TunnelStateDto::Failed { reason }
    }

    fn port_in_use() -> FailureReasonDto {
        FailureReasonDto::PortInUse {
            address: "127.0.0.1:3333".to_owned(),
        }
    }

    #[test]
    fn should_render_an_active_tunnel_with_a_check_mark_when_unicode_is_on() {
        assert_eq!(
            render_one(TunnelStateDto::Active, UNICODE_PLAIN),
            "✔ active  Adonis  3333 → nova:3333  [dev, adonis]"
        );
    }

    #[test]
    fn should_render_an_active_tunnel_with_ascii_when_unicode_is_off() {
        assert_eq!(
            render_one(TunnelStateDto::Active, ASCII_PLAIN),
            "[ok] active  Adonis  3333 -> nova:3333  [dev, adonis]"
        );
    }

    #[test]
    fn should_render_the_symbol_and_the_word_in_color_when_color_is_on() {
        assert_eq!(
            render_one(TunnelStateDto::Active, UNICODE_COLOR),
            "\x1b[32m✔ active\x1b[0m  Adonis  3333 → nova:3333  [dev, adonis]"
        );
    }

    #[test]
    fn should_render_a_reconnecting_tunnel_with_its_attempt() {
        let state = TunnelStateDto::Reconnecting {
            attempt: 2,
            next_retry_at: 0,
        };

        assert_eq!(
            render_one(state.clone(), UNICODE_PLAIN),
            "↻ reconnecting (attempt 2)  Adonis  3333 → nova:3333  [dev, adonis]"
        );
        assert_eq!(
            render_one(state, ASCII_PLAIN),
            "[~] reconnecting (attempt 2)  Adonis  3333 -> nova:3333  [dev, adonis]"
        );
    }

    #[test]
    fn should_render_a_starting_tunnel() {
        assert_eq!(
            render_one(TunnelStateDto::Starting, UNICODE_PLAIN),
            "… starting  Adonis  3333 → nova:3333  [dev, adonis]"
        );
        assert_eq!(
            render_one(TunnelStateDto::Starting, ASCII_PLAIN),
            "[..] starting  Adonis  3333 -> nova:3333  [dev, adonis]"
        );
    }

    #[test]
    fn should_render_a_stopped_tunnel() {
        assert_eq!(
            render_one(TunnelStateDto::Stopped, UNICODE_PLAIN),
            "○ stopped  Adonis  3333 → nova:3333  [dev, adonis]"
        );
        assert_eq!(
            render_one(TunnelStateDto::Stopped, ASCII_PLAIN),
            "[ ] stopped  Adonis  3333 -> nova:3333  [dev, adonis]"
        );
    }

    #[test]
    fn should_render_a_failed_tunnel_with_its_reason() {
        assert_eq!(
            render_one(failed(port_in_use()), UNICODE_PLAIN),
            "✖ failed: port already in use on 127.0.0.1:3333  Adonis  3333 → nova:3333  [dev, adonis]"
        );
        assert_eq!(
            render_one(failed(port_in_use()), ASCII_PLAIN),
            "[x] failed: port already in use on 127.0.0.1:3333  Adonis  3333 -> nova:3333  [dev, adonis]"
        );
    }

    #[test]
    fn should_paint_a_failure_in_red_when_color_is_on() {
        assert!(
            render_one(failed(port_in_use()), UNICODE_COLOR)
                .starts_with("\x1b[31m✖ failed: port already in use on 127.0.0.1:3333\x1b[0m")
        );
    }

    #[test]
    fn should_describe_every_kind_of_failure() {
        assert_eq!(
            [
                describe_failure(&FailureReasonDto::ListenFailed {
                    address: "0.0.0.0:80".to_owned(),
                    message: "permission denied".to_owned(),
                }),
                describe_failure(&FailureReasonDto::Connect {
                    message: "authentication failed".to_owned(),
                }),
                describe_failure(&FailureReasonDto::UnsupportedKind {
                    tunnel_kind: TunnelKindDto::Dynamic,
                }),
            ],
            [
                "cannot listen on 0.0.0.0:80: permission denied",
                "authentication failed",
                "dynamic tunnels are not supported yet",
            ]
        );
    }

    #[test]
    fn should_show_the_unreachable_target_since_when_the_health_is_down() {
        let mut down = tunnel(TunnelStateDto::Active);
        down.health = TargetHealthDto::Unreachable {
            since: UNREACHABLE_SINCE,
            error: "refused".to_owned(),
        };

        assert_eq!(
            render_status(&snapshot_of(vec![down.clone()]), UNICODE_PLAIN),
            "✔ active  Adonis  3333 → nova:3333  [dev, adonis] · target unreachable since 14:43:15 UTC"
        );
        assert_eq!(
            render_status(&snapshot_of(vec![down]), ASCII_PLAIN),
            "[ok] active  Adonis  3333 -> nova:3333  [dev, adonis] - target unreachable since 14:43:15 UTC"
        );
    }

    #[test]
    fn should_paint_the_unreachable_suffix_in_yellow_when_color_is_on() {
        let mut down = tunnel(TunnelStateDto::Active);
        down.health = TargetHealthDto::Unreachable {
            since: UNREACHABLE_SINCE,
            error: "refused".to_owned(),
        };

        assert!(
            render_status(&snapshot_of(vec![down]), UNICODE_COLOR)
                .ends_with("\x1b[33m· target unreachable since 14:43:15 UTC\x1b[0m")
        );
    }

    #[test]
    fn should_show_nothing_about_health_when_the_target_is_reachable() {
        let mut healthy = tunnel(TunnelStateDto::Active);
        healthy.health = TargetHealthDto::Reachable;

        assert_eq!(
            render_status(&snapshot_of(vec![healthy]), UNICODE_PLAIN),
            "✔ active  Adonis  3333 → nova:3333  [dev, adonis]"
        );
    }

    #[test]
    fn should_show_the_traffic_when_connections_were_accepted() {
        let mut busy = tunnel(TunnelStateDto::Active);
        busy.metrics = MetricsDto {
            active_connections: 1,
            total_connections: 12,
            bytes_in: 1536,
            bytes_out: 200,
            last_activity: Some(1),
        };

        assert_eq!(
            render_status(&snapshot_of(vec![busy]), UNICODE_PLAIN),
            "✔ active  Adonis  3333 → nova:3333  [dev, adonis] · 1 open, 12 total connections · 1.5 KiB in, 200 B out"
        );
    }

    #[test]
    fn should_omit_the_tags_when_the_tunnel_has_none() {
        let mut untagged = tunnel(TunnelStateDto::Stopped);
        untagged.definition.spec.tags.clear();

        assert_eq!(
            render_status(&snapshot_of(vec![untagged]), UNICODE_PLAIN),
            "○ stopped  Adonis  3333 → nova:3333"
        );
    }

    #[test]
    fn should_show_the_bind_address_when_it_differs_from_the_default() {
        let mut exposed = tunnel(TunnelStateDto::Stopped);
        exposed.definition.spec.listen_address = Some("0.0.0.0".to_owned());

        assert_eq!(
            describe_route(&exposed.definition, &settings(), UNICODE_PLAIN),
            "0.0.0.0:3333 → nova:3333"
        );
    }

    #[test]
    fn should_hide_the_bind_address_when_it_is_the_default() {
        let mut explicit = tunnel(TunnelStateDto::Stopped);
        explicit.definition.spec.listen_address = Some("127.0.0.1".to_owned());

        assert_eq!(
            describe_route(&explicit.definition, &settings(), UNICODE_PLAIN),
            "3333 → nova:3333"
        );
    }

    #[test]
    fn should_show_the_target_host_and_port_when_the_target_is_not_loopback() {
        let mut forwarded = tunnel(TunnelStateDto::Stopped);
        forwarded.definition.spec.target_host = Some("db.internal".to_owned());
        forwarded.definition.spec.target_port = Some(5432);

        assert_eq!(
            describe_route(&forwarded.definition, &settings(), ASCII_PLAIN),
            "3333 -> db.internal:5432 via nova"
        );
    }

    #[test]
    fn should_hide_the_target_host_when_it_is_the_loopback_address() {
        let mut loopback = tunnel(TunnelStateDto::Stopped);
        loopback.definition.spec.target_host = Some("127.0.0.1".to_owned());
        loopback.definition.spec.target_port = Some(8080);

        assert_eq!(
            describe_route(&loopback.definition, &settings(), UNICODE_PLAIN),
            "3333 → nova:8080"
        );
    }

    #[test]
    fn should_describe_remote_and_dynamic_routes() {
        let mut remote = tunnel(TunnelStateDto::Stopped);
        remote.definition.spec.kind = TunnelKindDto::Remote;
        let mut dynamic = tunnel(TunnelStateDto::Stopped);
        dynamic.definition.spec.kind = TunnelKindDto::Dynamic;

        assert_eq!(
            (
                describe_route(&remote.definition, &settings(), UNICODE_PLAIN),
                describe_route(&dynamic.definition, &settings(), UNICODE_PLAIN)
            ),
            (
                "nova:3333 → 127.0.0.1:3333".to_owned(),
                "3333 → SOCKS via nova".to_owned()
            )
        );
    }

    #[test]
    fn should_say_so_when_there_are_no_tunnels() {
        assert_eq!(
            render_status(&snapshot_of(Vec::new()), UNICODE_PLAIN),
            "No tunnels configured."
        );
    }

    #[test]
    fn should_render_one_line_per_tunnel() {
        let second = {
            let mut other = tunnel(TunnelStateDto::Stopped);
            "Mail".clone_into(&mut other.definition.spec.name);
            other
        };

        let rendered = render_status(
            &snapshot_of(vec![tunnel(TunnelStateDto::Active), second]),
            UNICODE_PLAIN,
        );

        assert_eq!(rendered.lines().count(), 2);
    }

    #[test]
    fn should_render_the_definitions_when_listing() {
        assert_eq!(
            render_list(
                &snapshot_of(vec![tunnel(TunnelStateDto::Active)]),
                UNICODE_PLAIN
            ),
            "Adonis  local  3333 → nova:3333  [dev, adonis]"
        );
    }

    #[test]
    fn should_summarize_the_active_tunnels() {
        let snapshot = snapshot_of(vec![
            tunnel(TunnelStateDto::Active),
            tunnel(TunnelStateDto::Stopped),
        ]);

        assert_eq!(
            render_daemon_summary("0.1.0", &snapshot),
            "Tunneler daemon 0.1.0: 1 of 2 tunnels active"
        );
    }

    #[test]
    fn should_format_a_time_of_day_in_utc() {
        assert_eq!(format_clock_time(UNREACHABLE_SINCE), "14:43:15");
    }

    #[test]
    fn should_wrap_the_time_of_day_after_midnight() {
        assert_eq!(
            format_clock_time(SECONDS_PER_DAY * MILLISECONDS_PER_SECOND + 1000),
            "00:00:01"
        );
    }

    #[test]
    fn should_format_byte_counts_with_binary_units() {
        assert_eq!(
            [0, 1023, 1024, 1536, 1_048_576, 5 * 1_073_741_824].map(format_bytes),
            ["0 B", "1023 B", "1.0 KiB", "1.5 KiB", "1.0 MiB", "5.0 GiB"]
        );
    }

    #[test]
    fn should_print_the_status_json_on_one_line_with_the_daemon_version() {
        let json = render_status_json("0.1.0", &snapshot_of(vec![tunnel(TunnelStateDto::Active)]))
            .unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(
            (
                json.contains('\n'),
                &parsed["daemonVersion"],
                &parsed["tunnels"][0]["name"],
                &parsed["tunnels"][0]["state"]["kind"],
            ),
            (false, &"0.1.0".into(), &"Adonis".into(), &"active".into())
        );
    }

    #[test]
    fn should_print_only_the_definitions_in_the_list_json() {
        let json = render_list_json(&snapshot_of(vec![tunnel(TunnelStateDto::Active)])).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(
            (
                &parsed["tunnels"][0]["listenPort"],
                parsed["tunnels"][0].get("state")
            ),
            (&3333.into(), None)
        );
    }
}
