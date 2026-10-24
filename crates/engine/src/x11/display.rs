use std::fmt;
use std::path::PathBuf;

use super::environment::{DisplayPlatform, X11Environment};
use super::error::X11Error;

const UNIX_SOCKET_DIRECTORY: &str = "/tmp/.X11-unix";
const TCP_BASE_PORT: u32 = 6000;
const WINDOWS_DEFAULT_DISPLAY: &str = "127.0.0.1:0.0";
const LOCAL_HOST_MARKER: &str = "unix";
const UNIX_HOST_SUFFIX: &str = "/unix";
const LOOPBACK_HOST: &str = "127.0.0.1";
const LOOPBACK_HOSTS: [&str; 3] = [LOOPBACK_HOST, "localhost", "::1"];

/// How to reach the local X server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DisplayEndpoint {
    UnixSocket(PathBuf),
    Tcp { host: String, port: u16 },
}

/// The local X display forwarded connections are sent to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LocalDisplay {
    pub(crate) number: u16,
    pub(crate) endpoint: DisplayEndpoint,
}

impl LocalDisplay {
    /// Whether an X server started on this machine would serve the display:
    /// a Unix socket, or TCP on a loopback host.
    pub(crate) fn is_local(&self) -> bool {
        match &self.endpoint {
            DisplayEndpoint::UnixSocket(_) => true,
            DisplayEndpoint::Tcp { host, .. } => LOOPBACK_HOSTS.contains(&host.as_str()),
        }
    }
}

impl fmt::Display for LocalDisplay {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.endpoint {
            DisplayEndpoint::UnixSocket(path) => write!(formatter, "{}", path.display()),
            DisplayEndpoint::Tcp { host, port } => write!(formatter, "{host}:{port}"),
        }
    }
}

/// Reads a display name: `[host[/unix]]:N[.S]`, `unix:N` or `[ipv6]:N`.
///
/// An empty host, `unix` and `host/unix` name the Unix socket of display `N`
/// on Unix platforms, and the loopback address on Windows. Any other host is
/// reached over TCP on port `6000 + N`.
pub(crate) fn parse_display(
    text: &str,
    platform: DisplayPlatform,
) -> Result<LocalDisplay, X11Error> {
    let invalid = |reason| X11Error::InvalidDisplay {
        text: text.to_owned(),
        reason,
    };
    let (host, screen_part) = text
        .rsplit_once(':')
        .ok_or_else(|| invalid("expected host:number"))?;
    let number = parse_display_number(screen_part).map_err(invalid)?;
    let endpoint = if is_local_host(host) {
        local_endpoint(number, platform)
    } else {
        tcp_endpoint(host, number).map_err(invalid)?
    };
    Ok(LocalDisplay { number, endpoint })
}

fn parse_display_number(screen_part: &str) -> Result<u16, &'static str> {
    let (number_text, screen_text) = match screen_part.split_once('.') {
        Some((number_text, screen_text)) => (number_text, Some(screen_text)),
        None => (screen_part, None),
    };
    if let Some(screen_text) = screen_text {
        screen_text
            .parse::<u16>()
            .map_err(|_| "the screen is not a number")?;
    }
    number_text
        .parse::<u16>()
        .map_err(|_| "the display number is not a number")
}

fn is_local_host(host: &str) -> bool {
    host.is_empty() || host == LOCAL_HOST_MARKER || host.ends_with(UNIX_HOST_SUFFIX)
}

fn local_endpoint(number: u16, platform: DisplayPlatform) -> DisplayEndpoint {
    match platform {
        DisplayPlatform::Unix => {
            DisplayEndpoint::UnixSocket(PathBuf::from(format!("{UNIX_SOCKET_DIRECTORY}/X{number}")))
        }
        DisplayPlatform::Windows => DisplayEndpoint::Tcp {
            host: LOOPBACK_HOST.to_owned(),
            port: u16::try_from(TCP_BASE_PORT + u32::from(number)).unwrap_or(u16::MAX),
        },
    }
}

fn tcp_endpoint(host: &str, number: u16) -> Result<DisplayEndpoint, &'static str> {
    let port = u16::try_from(TCP_BASE_PORT + u32::from(number))
        .map_err(|_| "the display number gives a port above 65535")?;
    let unbracketed = host
        .strip_prefix('[')
        .and_then(|inner| inner.strip_suffix(']'))
        .unwrap_or(host);
    Ok(DisplayEndpoint::Tcp {
        host: unbracketed.to_owned(),
        port,
    })
}

/// Picks the local display: the host override, then the settings, then
/// `$DISPLAY`, then the platform default (Windows only).
pub(crate) fn resolve_display(
    host_override: Option<&str>,
    settings_display: Option<&str>,
    environment: &dyn X11Environment,
) -> Result<LocalDisplay, X11Error> {
    let platform = environment.platform();
    if let Some(text) = host_override
        .or(settings_display)
        .map(str::to_owned)
        .or_else(|| environment.display())
    {
        return parse_display(&text, platform);
    }
    match platform {
        DisplayPlatform::Windows => parse_display(WINDOWS_DEFAULT_DISPLAY, platform),
        DisplayPlatform::Unix => Err(X11Error::NoLocalDisplay),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    struct FakeEnvironment {
        display: Option<String>,
        platform: DisplayPlatform,
    }

    impl X11Environment for FakeEnvironment {
        fn display(&self) -> Option<String> {
            self.display.clone()
        }

        fn xauthority_path(&self) -> Option<PathBuf> {
            None
        }

        fn home_directory(&self) -> Option<PathBuf> {
            None
        }

        fn host_name(&self) -> String {
            "testbed".to_owned()
        }

        fn platform(&self) -> DisplayPlatform {
            self.platform
        }
    }

    fn environment(display: Option<&str>, platform: DisplayPlatform) -> FakeEnvironment {
        FakeEnvironment {
            display: display.map(str::to_owned),
            platform,
        }
    }

    fn unix_socket(number: u16) -> DisplayEndpoint {
        DisplayEndpoint::UnixSocket(PathBuf::from(format!("/tmp/.X11-unix/X{number}")))
    }

    fn tcp(host: &str, port: u16) -> DisplayEndpoint {
        DisplayEndpoint::Tcp {
            host: host.to_owned(),
            port,
        }
    }

    fn endpoint(text: &str, platform: DisplayPlatform) -> DisplayEndpoint {
        parse_display(text, platform).unwrap().endpoint
    }

    fn is_local(text: &str) -> bool {
        parse_display(text, DisplayPlatform::Unix)
            .unwrap()
            .is_local()
    }

    #[test]
    fn should_be_local_when_the_display_is_a_unix_socket() {
        assert!(is_local(":0"));
    }

    #[test]
    fn should_be_local_when_the_tcp_host_is_a_loopback_address() {
        assert!(is_local("127.0.0.1:0"));
        assert!(is_local("localhost:1"));
        assert!(is_local("[::1]:2"));
    }

    #[test]
    fn should_not_be_local_when_the_tcp_host_is_another_machine() {
        assert!(!is_local("workstation:0"));
        assert!(!is_local("192.168.1.20:0"));
    }

    #[test]
    fn should_use_the_unix_socket_when_the_host_is_empty() {
        assert_eq!(endpoint(":0", DisplayPlatform::Unix), unix_socket(0));
    }

    #[test]
    fn should_ignore_the_screen_when_it_follows_the_display_number() {
        assert_eq!(endpoint(":1.2", DisplayPlatform::Unix), unix_socket(1));
    }

    #[test]
    fn should_use_the_unix_socket_when_the_host_is_unix() {
        assert_eq!(endpoint("unix:3", DisplayPlatform::Unix), unix_socket(3));
    }

    #[test]
    fn should_use_the_unix_socket_when_the_host_ends_with_slash_unix() {
        assert_eq!(
            endpoint("workstation/unix:4.0", DisplayPlatform::Unix),
            unix_socket(4)
        );
    }

    #[test]
    fn should_use_the_loopback_address_when_the_host_is_empty_on_windows() {
        assert_eq!(
            endpoint(":0.0", DisplayPlatform::Windows),
            tcp("127.0.0.1", 6000)
        );
    }

    #[test]
    fn should_use_tcp_when_the_host_is_a_name() {
        assert_eq!(
            endpoint("localhost:10.0", DisplayPlatform::Unix),
            tcp("localhost", 6010)
        );
    }

    #[test]
    fn should_use_tcp_when_the_host_is_an_ipv4_address() {
        assert_eq!(
            endpoint("192.0.2.50:2", DisplayPlatform::Unix),
            tcp("192.0.2.50", 6002)
        );
    }

    #[test]
    fn should_strip_the_brackets_when_the_host_is_an_ipv6_address() {
        assert_eq!(endpoint("[::1]:0", DisplayPlatform::Unix), tcp("::1", 6000));
    }

    #[test]
    fn should_keep_the_display_number_when_the_host_is_a_name() {
        assert_eq!(
            parse_display("localhost:10.0", DisplayPlatform::Unix)
                .unwrap()
                .number,
            10
        );
    }

    #[test]
    fn should_reject_the_display_when_it_has_no_colon() {
        assert!(matches!(
            parse_display("localhost", DisplayPlatform::Unix),
            Err(X11Error::InvalidDisplay { .. })
        ));
    }

    #[test]
    fn should_reject_the_display_when_the_number_is_not_numeric() {
        assert!(matches!(
            parse_display(":zero", DisplayPlatform::Unix),
            Err(X11Error::InvalidDisplay { .. })
        ));
    }

    #[test]
    fn should_reject_the_display_when_the_screen_is_not_numeric() {
        assert!(matches!(
            parse_display(":0.main", DisplayPlatform::Unix),
            Err(X11Error::InvalidDisplay { .. })
        ));
    }

    #[test]
    fn should_reject_the_display_when_the_tcp_port_overflows() {
        assert!(matches!(
            parse_display("localhost:60000", DisplayPlatform::Unix),
            Err(X11Error::InvalidDisplay { .. })
        ));
    }

    #[test]
    fn should_prefer_the_host_override_when_every_source_is_set() {
        let environment = environment(Some(":3"), DisplayPlatform::Unix);

        let display = resolve_display(Some(":1"), Some(":2"), &environment).unwrap();

        assert_eq!(display.number, 1);
    }

    #[test]
    fn should_prefer_the_settings_when_there_is_no_host_override() {
        let environment = environment(Some(":3"), DisplayPlatform::Unix);

        let display = resolve_display(None, Some(":2"), &environment).unwrap();

        assert_eq!(display.number, 2);
    }

    #[test]
    fn should_use_the_environment_when_nothing_is_configured() {
        let environment = environment(Some(":3"), DisplayPlatform::Unix);

        let display = resolve_display(None, None, &environment).unwrap();

        assert_eq!(display.number, 3);
    }

    #[test]
    fn should_fail_when_no_display_is_known_on_unix() {
        let environment = environment(None, DisplayPlatform::Unix);

        assert!(matches!(
            resolve_display(None, None, &environment),
            Err(X11Error::NoLocalDisplay)
        ));
    }

    #[test]
    fn should_default_to_the_loopback_display_when_no_display_is_known_on_windows() {
        let environment = environment(None, DisplayPlatform::Windows);

        let display = resolve_display(None, None, &environment).unwrap();

        assert_eq!(display.endpoint, tcp("127.0.0.1", 6000));
    }

    #[test]
    fn should_describe_a_tcp_display_with_its_host_and_port() {
        let display = parse_display("localhost:10.0", DisplayPlatform::Unix).unwrap();

        assert_eq!(display.to_string(), "localhost:6010");
    }
}
