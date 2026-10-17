use std::io;
use std::net::IpAddr;
use std::path::PathBuf;

use tracing::warn;

use super::cookie::MIT_MAGIC_COOKIE_NAME;
use super::display::{DisplayEndpoint, LocalDisplay};
use super::environment::X11Environment;
use super::error::X11Error;

const FAMILY_INTERNET: u16 = 0;
const FAMILY_INTERNET6: u16 = 6;
const FAMILY_LOCAL: u16 = 256;
const FAMILY_WILD: u16 = 65535;
const DEFAULT_FILE_NAME: &str = ".Xauthority";
const LOCALHOST_NAME: &str = "localhost";

/// One record of an `Xauthority` file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct XauthorityEntry {
    pub(crate) family: u16,
    pub(crate) address: Vec<u8>,
    pub(crate) number: Vec<u8>,
    pub(crate) name: Vec<u8>,
    pub(crate) data: Vec<u8>,
}

/// What could be read of an `Xauthority` file.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ParsedXauthority {
    pub(crate) entries: Vec<XauthorityEntry>,
    pub(crate) truncated_at: Option<usize>,
}

struct ByteReader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl ByteReader<'_> {
    fn is_finished(&self) -> bool {
        self.position >= self.bytes.len()
    }

    fn read_u16(&mut self) -> Option<u16> {
        let end = self.position.checked_add(2)?;
        let pair = self.bytes.get(self.position..end)?;
        self.position = end;
        Some(u16::from_be_bytes([pair[0], pair[1]]))
    }

    fn read_block(&mut self) -> Option<Vec<u8>> {
        let length = usize::from(self.read_u16()?);
        let end = self.position.checked_add(length)?;
        let block = self.bytes.get(self.position..end)?;
        self.position = end;
        Some(block.to_vec())
    }

    fn read_entry(&mut self) -> Option<XauthorityEntry> {
        Some(XauthorityEntry {
            family: self.read_u16()?,
            address: self.read_block()?,
            number: self.read_block()?,
            name: self.read_block()?,
            data: self.read_block()?,
        })
    }
}

/// Reads every complete entry; stops at the first one that is cut short.
pub(crate) fn parse_xauthority(bytes: &[u8]) -> ParsedXauthority {
    let mut reader = ByteReader { bytes, position: 0 };
    let mut entries = Vec::new();
    while !reader.is_finished() {
        let entry_start = reader.position;
        match reader.read_entry() {
            Some(entry) => entries.push(entry),
            None => {
                return ParsedXauthority {
                    entries,
                    truncated_at: Some(entry_start),
                };
            }
        }
    }
    ParsedXauthority {
        entries,
        truncated_at: None,
    }
}

/// The cookie that grants access to `display`, if the entries hold one.
pub(crate) fn find_cookie(
    entries: &[XauthorityEntry],
    display: &LocalDisplay,
    host_name: &str,
) -> Option<Vec<u8>> {
    let number = display.number.to_string();
    entries
        .iter()
        .filter(|entry| entry.name == MIT_MAGIC_COOKIE_NAME)
        .filter(|entry| entry.number == number.as_bytes())
        .find(|entry| matches_address(entry, display, host_name))
        .map(|entry| entry.data.clone())
}

fn matches_address(entry: &XauthorityEntry, display: &LocalDisplay, host_name: &str) -> bool {
    if entry.family == FAMILY_WILD {
        return true;
    }
    match &display.endpoint {
        DisplayEndpoint::UnixSocket(_) => is_local_entry(entry, host_name),
        DisplayEndpoint::Tcp { host, .. } => matches_tcp_host(entry, host, host_name),
    }
}

fn matches_tcp_host(entry: &XauthorityEntry, host: &str, host_name: &str) -> bool {
    if let Ok(address) = host.parse::<IpAddr>() {
        return matches_ip_address(entry, address)
            || (address.is_loopback() && is_local_entry(entry, host_name));
    }
    let is_this_machine =
        host.eq_ignore_ascii_case(LOCALHOST_NAME) || host.eq_ignore_ascii_case(host_name);
    is_this_machine && is_local_entry(entry, host_name)
}

fn matches_ip_address(entry: &XauthorityEntry, address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => entry.family == FAMILY_INTERNET && entry.address == address.octets(),
        IpAddr::V6(address) => {
            entry.family == FAMILY_INTERNET6 && entry.address == address.octets()
        }
    }
}

fn is_local_entry(entry: &XauthorityEntry, host_name: &str) -> bool {
    entry.family == FAMILY_LOCAL
        && !host_name.is_empty()
        && entry.address.eq_ignore_ascii_case(host_name.as_bytes())
}

/// Reads the cookie of `display` from `$XAUTHORITY`, else `~/.Xauthority`.
///
/// A missing file or a file without a matching entry gives no cookie.
/// Blocking: callers in async code run it through `run_blocking`.
pub(crate) fn load_cookie(
    display: &LocalDisplay,
    environment: &dyn X11Environment,
) -> Result<Option<Vec<u8>>, X11Error> {
    let Some(path) = authority_path(environment) else {
        return Ok(None);
    };
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(source) => return Err(X11Error::ReadXauthority { path, source }),
    };
    let parsed = parse_xauthority(&bytes);
    if let Some(offset) = parsed.truncated_at {
        warn!(path = %path.display(), offset, "the Xauthority file ends with a truncated entry");
    }
    Ok(find_cookie(
        &parsed.entries,
        display,
        &environment.host_name(),
    ))
}

fn authority_path(environment: &dyn X11Environment) -> Option<PathBuf> {
    environment
        .xauthority_path()
        .or_else(|| Some(environment.home_directory()?.join(DEFAULT_FILE_NAME)))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::x11::environment::DisplayPlatform;

    const HOST_NAME: &str = "testbed";
    const COOKIE: [u8; 16] = [
        0x6a, 0x1f, 0x03, 0x9c, 0x22, 0x7e, 0xb4, 0x58, 0xd1, 0x0a, 0x95, 0x3c, 0xe7, 0x42, 0x81,
        0x1d,
    ];
    const OTHER_COOKIE: [u8; 16] = [0xaa; 16];

    /// The file `xauth add` writes for `testbed/unix:0`, `192.0.2.50:1` and
    /// `[fe80::1]:2`, in this order.
    const XAUTH_SAMPLE: [u8; 162] = [
        0x01, 0x00, 0x00, 0x07, 0x74, 0x65, 0x73, 0x74, 0x62, 0x65, 0x64, 0x00, 0x01, 0x30, 0x00,
        0x12, 0x4d, 0x49, 0x54, 0x2d, 0x4d, 0x41, 0x47, 0x49, 0x43, 0x2d, 0x43, 0x4f, 0x4f, 0x4b,
        0x49, 0x45, 0x2d, 0x31, 0x00, 0x10, 0x6a, 0x1f, 0x03, 0x9c, 0x22, 0x7e, 0xb4, 0x58, 0xd1,
        0x0a, 0x95, 0x3c, 0xe7, 0x42, 0x81, 0x1d, 0x00, 0x00, 0x00, 0x04, 0xc0, 0x00, 0x02, 0x32,
        0x00, 0x01, 0x31, 0x00, 0x12, 0x4d, 0x49, 0x54, 0x2d, 0x4d, 0x41, 0x47, 0x49, 0x43, 0x2d,
        0x43, 0x4f, 0x4f, 0x4b, 0x49, 0x45, 0x2d, 0x31, 0x00, 0x10, 0xaa, 0xbb, 0xcc, 0xdd, 0xee,
        0xff, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0x00, 0x06, 0x00, 0x10,
        0xfe, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x01, 0x00, 0x01, 0x32, 0x00, 0x12, 0x4d, 0x49, 0x54, 0x2d, 0x4d, 0x41, 0x47, 0x49, 0x43,
        0x2d, 0x43, 0x4f, 0x4f, 0x4b, 0x49, 0x45, 0x2d, 0x31, 0x00, 0x10, 0x00, 0x11, 0x22, 0x33,
        0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff,
    ];

    const LOCAL_ENTRY_LENGTH: usize = 52;

    fn entry(family: u16, address: &[u8], number: &str, cookie: &[u8]) -> XauthorityEntry {
        XauthorityEntry {
            family,
            address: address.to_vec(),
            number: number.as_bytes().to_vec(),
            name: MIT_MAGIC_COOKIE_NAME.to_vec(),
            data: cookie.to_vec(),
        }
    }

    fn encode(entries: &[XauthorityEntry]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for entry in entries {
            bytes.extend(entry.family.to_be_bytes());
            for block in [&entry.address, &entry.number, &entry.name, &entry.data] {
                bytes.extend(u16::try_from(block.len()).unwrap().to_be_bytes());
                bytes.extend(block.iter());
            }
        }
        bytes
    }

    fn unix_display(number: u16) -> LocalDisplay {
        LocalDisplay {
            number,
            endpoint: DisplayEndpoint::UnixSocket(PathBuf::from("/tmp/.X11-unix/X0")),
        }
    }

    fn tcp_display(host: &str, number: u16) -> LocalDisplay {
        LocalDisplay {
            number,
            endpoint: DisplayEndpoint::Tcp {
                host: host.to_owned(),
                port: 6000 + number,
            },
        }
    }

    #[test]
    fn should_parse_every_entry_when_the_bytes_come_from_xauth() {
        let address = "fe80::1".parse::<std::net::Ipv6Addr>().unwrap();
        let expected = [
            entry(FAMILY_LOCAL, b"testbed", "0", &COOKIE),
            entry(
                FAMILY_INTERNET,
                &[192, 0, 2, 50],
                "1",
                &[
                    0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66,
                    0x77, 0x88, 0x99,
                ],
            ),
            entry(
                FAMILY_INTERNET6,
                &address.octets(),
                "2",
                &[
                    0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc,
                    0xdd, 0xee, 0xff,
                ],
            ),
        ];

        assert_eq!(parse_xauthority(&XAUTH_SAMPLE).entries, expected);
    }

    #[test]
    fn should_report_no_truncation_when_the_file_is_complete() {
        assert_eq!(parse_xauthority(&XAUTH_SAMPLE).truncated_at, None);
    }

    #[test]
    fn should_parse_every_entry_when_the_file_holds_several_families() {
        let entries = [
            entry(FAMILY_INTERNET, &[198, 51, 100, 10], "1", &COOKIE),
            entry(FAMILY_INTERNET6, &[0xfe; 16], "2", &COOKIE),
            entry(FAMILY_LOCAL, b"testbed", "0", &COOKIE),
            entry(FAMILY_WILD, b"", "3", &COOKIE),
        ];

        assert_eq!(parse_xauthority(&encode(&entries)).entries, entries);
    }

    #[test]
    fn should_keep_the_complete_entries_when_the_file_is_truncated() {
        let bytes = &XAUTH_SAMPLE[..XAUTH_SAMPLE.len() - 1];

        assert_eq!(parse_xauthority(bytes).entries.len(), 2);
    }

    #[test]
    fn should_report_where_the_truncation_starts_when_the_file_is_cut() {
        let bytes = &XAUTH_SAMPLE[..XAUTH_SAMPLE.len() - 1];

        assert_eq!(
            parse_xauthority(bytes).truncated_at,
            Some(LOCAL_ENTRY_LENGTH + 49)
        );
    }

    #[test]
    fn should_report_no_entry_when_the_file_is_empty() {
        assert_eq!(parse_xauthority(&[]).entries, []);
    }

    #[test]
    fn should_find_the_cookie_when_a_local_entry_matches_a_unix_display() {
        let entries = [entry(FAMILY_LOCAL, b"testbed", "0", &COOKIE)];

        assert_eq!(
            find_cookie(&entries, &unix_display(0), HOST_NAME),
            Some(COOKIE.to_vec())
        );
    }

    #[test]
    fn should_find_the_cookie_when_a_wild_entry_matches_a_unix_display() {
        let entries = [entry(FAMILY_WILD, b"", "0", &COOKIE)];

        assert_eq!(
            find_cookie(&entries, &unix_display(0), HOST_NAME),
            Some(COOKIE.to_vec())
        );
    }

    #[test]
    fn should_ignore_the_entry_when_the_host_name_differs_for_a_unix_display() {
        let entries = [entry(FAMILY_LOCAL, b"elsewhere", "0", &COOKIE)];

        assert_eq!(find_cookie(&entries, &unix_display(0), HOST_NAME), None);
    }

    #[test]
    fn should_ignore_the_entry_when_the_display_number_differs() {
        let entries = [entry(FAMILY_LOCAL, b"testbed", "1", &COOKIE)];

        assert_eq!(find_cookie(&entries, &unix_display(0), HOST_NAME), None);
    }

    #[test]
    fn should_ignore_the_entry_when_the_protocol_is_not_mit_magic_cookie() {
        let mut other = entry(FAMILY_LOCAL, b"testbed", "0", &COOKIE);
        other.name = b"XDM-AUTHORIZATION-1".to_vec();

        assert_eq!(find_cookie(&[other], &unix_display(0), HOST_NAME), None);
    }

    #[test]
    fn should_find_the_cookie_when_an_internet_entry_matches_the_tcp_address() {
        let entries = [entry(FAMILY_INTERNET, &[192, 0, 2, 50], "0", &COOKIE)];

        assert_eq!(
            find_cookie(&entries, &tcp_display("192.0.2.50", 0), HOST_NAME),
            Some(COOKIE.to_vec())
        );
    }

    #[test]
    fn should_find_the_cookie_when_an_internet6_entry_matches_the_tcp_address() {
        let address = "fe80::1".parse::<std::net::Ipv6Addr>().unwrap();
        let entries = [entry(FAMILY_INTERNET6, &address.octets(), "0", &COOKIE)];

        assert_eq!(
            find_cookie(&entries, &tcp_display("fe80::1", 0), HOST_NAME),
            Some(COOKIE.to_vec())
        );
    }

    #[test]
    fn should_ignore_the_entry_when_the_internet_address_differs() {
        let entries = [entry(FAMILY_INTERNET, &[192, 0, 2, 60], "0", &COOKIE)];

        assert_eq!(
            find_cookie(&entries, &tcp_display("192.0.2.50", 0), HOST_NAME),
            None
        );
    }

    #[test]
    fn should_accept_the_local_entry_when_the_tcp_display_is_the_loopback_address() {
        let entries = [entry(FAMILY_LOCAL, b"testbed", "0", &COOKIE)];

        assert_eq!(
            find_cookie(&entries, &tcp_display("127.0.0.1", 0), HOST_NAME),
            Some(COOKIE.to_vec())
        );
    }

    #[test]
    fn should_accept_the_local_entry_when_the_tcp_display_is_localhost() {
        let entries = [entry(FAMILY_LOCAL, b"testbed", "10", &COOKIE)];

        assert_eq!(
            find_cookie(&entries, &tcp_display("localhost", 10), HOST_NAME),
            Some(COOKIE.to_vec())
        );
    }

    #[test]
    fn should_ignore_the_local_entry_when_the_tcp_display_is_a_remote_address() {
        let entries = [entry(FAMILY_LOCAL, b"testbed", "0", &COOKIE)];

        assert_eq!(
            find_cookie(&entries, &tcp_display("192.0.2.50", 0), HOST_NAME),
            None
        );
    }

    #[test]
    fn should_return_the_first_match_when_several_entries_match() {
        let entries = [
            entry(FAMILY_LOCAL, b"testbed", "0", &COOKIE),
            entry(FAMILY_WILD, b"", "0", &OTHER_COOKIE),
        ];

        assert_eq!(
            find_cookie(&entries, &unix_display(0), HOST_NAME),
            Some(COOKIE.to_vec())
        );
    }

    struct FileEnvironment {
        xauthority: Option<PathBuf>,
        home: Option<PathBuf>,
    }

    impl X11Environment for FileEnvironment {
        fn display(&self) -> Option<String> {
            None
        }

        fn xauthority_path(&self) -> Option<PathBuf> {
            self.xauthority.clone()
        }

        fn home_directory(&self) -> Option<PathBuf> {
            self.home.clone()
        }

        fn host_name(&self) -> String {
            HOST_NAME.to_owned()
        }

        fn platform(&self) -> DisplayPlatform {
            DisplayPlatform::Unix
        }
    }

    fn write_authority(directory: &Path, name: &str) -> PathBuf {
        let path = directory.join(name);
        std::fs::write(&path, XAUTH_SAMPLE).unwrap();
        path
    }

    #[test]
    fn should_load_the_cookie_from_the_xauthority_variable_when_it_is_set() {
        let directory = tempfile::tempdir().unwrap();
        let environment = FileEnvironment {
            xauthority: Some(write_authority(directory.path(), "custom")),
            home: None,
        };

        let cookie = load_cookie(&unix_display(0), &environment).unwrap();

        assert_eq!(cookie, Some(COOKIE.to_vec()));
    }

    #[test]
    fn should_load_the_cookie_from_the_home_directory_when_the_variable_is_unset() {
        let directory = tempfile::tempdir().unwrap();
        write_authority(directory.path(), ".Xauthority");
        let environment = FileEnvironment {
            xauthority: None,
            home: Some(directory.path().to_path_buf()),
        };

        let cookie = load_cookie(&unix_display(0), &environment).unwrap();

        assert_eq!(cookie, Some(COOKIE.to_vec()));
    }

    #[test]
    fn should_find_no_cookie_when_the_file_is_missing() {
        let directory = tempfile::tempdir().unwrap();
        let environment = FileEnvironment {
            xauthority: Some(directory.path().join("absent")),
            home: None,
        };

        assert_eq!(load_cookie(&unix_display(0), &environment).unwrap(), None);
    }

    #[test]
    fn should_find_no_cookie_when_no_path_is_known() {
        let environment = FileEnvironment {
            xauthority: None,
            home: None,
        };

        assert_eq!(load_cookie(&unix_display(0), &environment).unwrap(), None);
    }

    #[test]
    fn should_fail_when_the_path_is_a_directory() {
        let directory = tempfile::tempdir().unwrap();
        let environment = FileEnvironment {
            xauthority: Some(directory.path().to_path_buf()),
            home: None,
        };

        assert!(matches!(
            load_cookie(&unix_display(0), &environment),
            Err(X11Error::ReadXauthority { .. })
        ));
    }
}
