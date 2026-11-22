//! Reads the `tunneler://devslot/<host>/<instance>` link the app was started
//! with.
//!
//! Any web page can start the app with a link, so the parser accepts one exact
//! shape and turns everything else into [`DevslotLinkDto::Invalid`], whose text
//! is never relayed. The link is read from the raw argument, not from a
//! normalizing URL parser: normalization (dot segments, stripped control
//! characters) would let a link that is not in the contract look like one.

use std::ffi::OsString;

use ipc::protocol::{DevslotHostName, DevslotInstanceName, DevslotLinkDto};

const SCHEME_PREFIX: &str = "tunneler:";
const LINK_PREFIX: &str = "tunneler://";
const AUTHORITY: &str = "devslot";

/// The link among the process arguments (the program name included), when
/// there is one. Only the first argument that names the `tunneler` scheme
/// counts.
pub fn find_launch_link(arguments: impl Iterator<Item = OsString>) -> Option<DevslotLinkDto> {
    arguments
        .skip(1)
        .map(|argument| argument.to_string_lossy().into_owned())
        .find(|argument| has_scheme(argument))
        .map(|argument| parse_devslot_link(&argument))
}

/// Parses a link; anything outside the contract is
/// [`DevslotLinkDto::Invalid`].
pub fn parse_devslot_link(link: &str) -> DevslotLinkDto {
    try_parse_devslot_link(link).unwrap_or(DevslotLinkDto::Invalid)
}

fn has_scheme(argument: &str) -> bool {
    argument
        .as_bytes()
        .get(..SCHEME_PREFIX.len())
        .is_some_and(|start| start.eq_ignore_ascii_case(SCHEME_PREFIX.as_bytes()))
}

fn try_parse_devslot_link(link: &str) -> Option<DevslotLinkDto> {
    let rest = strip_link_prefix(link)?;
    if rest.contains(['?', '#']) {
        return None;
    }
    let (authority, path) = rest.split_once('/')?;
    if !authority.eq_ignore_ascii_case(AUTHORITY) {
        return None;
    }
    let [host, instance] = split_segments(path)?;
    Some(DevslotLinkDto::DevslotUp {
        host: DevslotHostName::try_from(decode_segment(host)?).ok()?,
        instance: DevslotInstanceName::try_from(decode_segment(instance)?).ok()?,
    })
}

fn strip_link_prefix(link: &str) -> Option<&str> {
    let start = link.as_bytes().get(..LINK_PREFIX.len())?;
    if !start.eq_ignore_ascii_case(LINK_PREFIX.as_bytes()) {
        return None;
    }
    link.get(LINK_PREFIX.len()..)
}

fn split_segments(path: &str) -> Option<[&str; 2]> {
    let path = path.strip_suffix('/').unwrap_or(path);
    let mut segments = path.split('/');
    let first = segments.next()?;
    let second = segments.next()?;
    if segments.next().is_some() || first.is_empty() || second.is_empty() {
        return None;
    }
    Some([first, second])
}

/// Decodes the percent escapes of a segment, once, as UTF-8.
fn decode_segment(segment: &str) -> Option<String> {
    let bytes = segment.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = hexadecimal_value(*bytes.get(index + 1)?)?;
            let low = hexadecimal_value(*bytes.get(index + 2)?)?;
            decoded.push(high * 16 + low);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).ok()
}

fn hexadecimal_value(digit: u8) -> Option<u8> {
    match digit {
        b'0'..=b'9' => Some(digit - b'0'),
        b'a'..=b'f' => Some(digit - b'a' + 10),
        b'A'..=b'F' => Some(digit - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn link_to(host: &str, instance: &str) -> DevslotLinkDto {
        DevslotLinkDto::DevslotUp {
            host: DevslotHostName::parse(host).unwrap(),
            instance: DevslotInstanceName::parse(instance).unwrap(),
        }
    }

    fn assert_rejected(link: &str) {
        assert_eq!(parse_devslot_link(link), DevslotLinkDto::Invalid, "{link}");
    }

    #[test]
    fn should_accept_the_contract_example() {
        assert_eq!(
            parse_devslot_link("tunneler://devslot/vega/web-shop-098a"),
            link_to("vega", "web-shop-098a")
        );
    }

    #[test]
    fn should_accept_a_scheme_and_an_authority_in_any_case() {
        assert_eq!(
            parse_devslot_link("TUNNELER://DevSlot/vega/web-shop-098a"),
            link_to("vega", "web-shop-098a")
        );
    }

    #[test]
    fn should_accept_one_trailing_slash() {
        assert_eq!(
            parse_devslot_link("tunneler://devslot/vega/web-shop-098a/"),
            link_to("vega", "web-shop-098a")
        );
    }

    #[test]
    fn should_decode_the_percent_escapes_of_each_segment_once() {
        assert_eq!(
            parse_devslot_link("tunneler://devslot/ve%67a/web%2Dshop%2e098a"),
            link_to("vega", "web-shop.098a")
        );
    }

    #[test]
    fn should_reject_a_scheme_other_than_tunneler() {
        assert_rejected("https://devslot/vega/web-shop");
        assert_rejected("tunneler:devslot/vega/web-shop");
    }

    #[test]
    fn should_reject_an_authority_other_than_devslot() {
        assert_rejected("tunneler://devslots/vega/web-shop");
        assert_rejected("tunneler:///vega/web-shop");
    }

    #[test]
    fn should_reject_a_path_without_exactly_two_segments() {
        assert_rejected("tunneler://devslot");
        assert_rejected("tunneler://devslot/");
        assert_rejected("tunneler://devslot/vega");
        assert_rejected("tunneler://devslot/vega/web-shop/extra");
    }

    #[test]
    fn should_reject_an_empty_segment() {
        assert_rejected("tunneler://devslot//web-shop");
        assert_rejected("tunneler://devslot/vega//");
        assert_rejected("tunneler://devslot/vega/web-shop//");
    }

    #[test]
    fn should_reject_a_query_a_fragment_userinfo_or_a_port() {
        assert_rejected("tunneler://devslot/vega/web-shop?x=1");
        assert_rejected("tunneler://devslot/vega/web-shop#top");
        assert_rejected("tunneler://devslot/vega/web-shop/?");
        assert_rejected("tunneler://user@devslot/vega/web-shop");
        assert_rejected("tunneler://devslot:80/vega/web-shop");
    }

    #[test]
    fn should_accept_a_host_of_64_characters_and_reject_one_of_65() {
        let accepted = format!("tunneler://devslot/{}/web-shop", "a".repeat(64));
        let rejected = format!("tunneler://devslot/{}/web-shop", "a".repeat(65));

        assert_eq!(
            parse_devslot_link(&accepted),
            link_to(&"a".repeat(64), "web-shop")
        );
        assert_rejected(&rejected);
    }

    #[test]
    fn should_accept_an_instance_of_255_characters_and_reject_one_of_256() {
        let accepted = format!("tunneler://devslot/vega/{}", "a".repeat(255));
        let rejected = format!("tunneler://devslot/vega/{}", "a".repeat(256));

        assert_eq!(
            parse_devslot_link(&accepted),
            link_to("vega", &"a".repeat(255))
        );
        assert_rejected(&rejected);
    }

    #[test]
    fn should_reject_a_segment_that_starts_with_a_dash_or_a_dot() {
        assert_rejected("tunneler://devslot/-vega/web-shop");
        assert_rejected("tunneler://devslot/.vega/web-shop");
        assert_rejected("tunneler://devslot/vega/-web-shop");
        assert_rejected("tunneler://devslot/vega/.web-shop");
        assert_rejected("tunneler://devslot/%2Dvega/web-shop");
        assert_rejected("tunneler://devslot/vega/%2eweb-shop");
    }

    #[test]
    fn should_reject_an_escape_that_decodes_to_a_slash() {
        assert_rejected("tunneler://devslot/vega/web%2Fshop");
        assert_rejected("tunneler://devslot/ve%2fga/web-shop");
    }

    #[test]
    fn should_reject_an_escape_that_decodes_to_a_query_or_a_fragment_character() {
        assert_rejected("tunneler://devslot/vega/web%3Fshop");
        assert_rejected("tunneler://devslot/vega/web%23shop");
    }

    #[test]
    fn should_reject_a_malformed_escape() {
        assert_rejected("tunneler://devslot/vega/web%");
        assert_rejected("tunneler://devslot/vega/web%2");
        assert_rejected("tunneler://devslot/vega/web%zz");
    }

    #[test]
    fn should_reject_an_escape_that_is_not_valid_utf8() {
        assert_rejected("tunneler://devslot/vega/web%FF");
        assert_rejected("tunneler://devslot/vega/web%C3");
    }

    #[test]
    fn should_reject_non_ascii_characters_encoded_or_not() {
        assert_rejected("tunneler://devslot/vega/caf\u{e9}");
        assert_rejected("tunneler://devslot/vega/caf%C3%A9");
        assert_rejected("tunneler://devslot/vega/web%E2%80%AEshop");
        assert_rejected("tunneler://devslot/vega/web\u{202e}shop");
    }

    #[test]
    fn should_reject_control_characters_and_spaces() {
        assert_rejected("tunneler://devslot/vega/web%00shop");
        assert_rejected("tunneler://devslot/vega/web%0Ashop");
        assert_rejected("tunneler://devslot/vega/web\tshop");
        assert_rejected("tunneler://devslot/vega/web shop");
        assert_rejected("tunneler://devslot/vega/web%20shop");
    }

    #[test]
    fn should_not_decode_an_escape_twice() {
        assert_rejected("tunneler://devslot/vega/web%252Dshop");
    }

    #[test]
    fn should_find_no_link_when_no_argument_names_the_scheme() {
        let arguments = ["tunneler-ui", "--help"].map(OsString::from);

        assert_eq!(find_launch_link(arguments.into_iter()), None);
    }

    #[test]
    fn should_find_no_link_without_arguments() {
        assert_eq!(find_launch_link(std::iter::empty()), None);
    }

    #[test]
    fn should_find_the_link_of_the_first_argument_that_names_the_scheme() {
        let arguments = [
            "tunneler-ui",
            "tunneler://devslot/vega/web-shop-098a",
            "tunneler://devslot/other/web-shop-098a",
        ]
        .map(OsString::from);

        assert_eq!(
            find_launch_link(arguments.into_iter()),
            Some(link_to("vega", "web-shop-098a"))
        );
    }

    #[test]
    fn should_find_an_invalid_link_when_the_argument_names_the_scheme_but_breaks_the_contract() {
        let arguments = ["tunneler-ui", "tunneler://elsewhere/x/y"].map(OsString::from);

        assert_eq!(
            find_launch_link(arguments.into_iter()),
            Some(DevslotLinkDto::Invalid)
        );
    }
}
