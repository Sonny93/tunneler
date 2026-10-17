//! The only URLs the bridge opens in the browser: a tunnel's own listen address.

use std::net::IpAddr;

use crate::bridge_error::BridgeError;

const LOCALHOST: &str = "localhost";
const IPV6_BRACKETS: [char; 2] = ['[', ']'];

/// `http://{host}:{port}/`, with an IPv6 host in brackets.
///
/// Only `localhost` and IP literals are accepted, so the front end cannot
/// have an arbitrary site opened.
pub fn browser_url(host: &str, port: u16) -> Result<String, BridgeError> {
    let unbracketed = host.trim_matches(IPV6_BRACKETS);
    if unbracketed.eq_ignore_ascii_case(LOCALHOST) {
        return Ok(format!("http://{LOCALHOST}:{port}/"));
    }
    match unbracketed.parse::<IpAddr>() {
        Ok(IpAddr::V6(address)) => Ok(format!("http://[{address}]:{port}/")),
        Ok(IpAddr::V4(address)) => Ok(format!("http://{address}:{port}/")),
        Err(_) => Err(BridgeError::invalid_target(host)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(host: &str) -> Option<String> {
        browser_url(host, 3333).ok()
    }

    #[test]
    fn should_build_url_when_host_is_localhost() {
        assert_eq!(url("localhost"), Some("http://localhost:3333/".to_owned()));
    }

    #[test]
    fn should_build_url_when_host_is_an_ipv4_address() {
        assert_eq!(url("127.0.0.1"), Some("http://127.0.0.1:3333/".to_owned()));
    }

    #[test]
    fn should_bracket_host_when_it_is_an_ipv6_address() {
        assert_eq!(url("::1"), Some("http://[::1]:3333/".to_owned()));
    }

    #[test]
    fn should_not_bracket_twice_when_ipv6_host_is_already_bracketed() {
        assert_eq!(url("[::1]"), Some("http://[::1]:3333/".to_owned()));
    }

    #[test]
    fn should_refuse_host_when_it_is_a_domain_name() {
        assert_eq!(url("example.org"), None);
    }

    #[test]
    fn should_refuse_host_when_it_smuggles_a_url_suffix() {
        assert_eq!(url("127.0.0.1@evil.example"), None);
    }
}
