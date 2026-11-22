//! The browser host name of a devslot web port.

use crate::store::find_host_name_problem;

const URL_SCHEMES: [&str; 2] = ["http://", "https://"];
const AUTHORITY_END: [char; 3] = ['/', '?', '#'];

/// The host of `url` when it is a valid open host name
/// (`<label>.localhost`), else `None`: `http://my-app.localhost:20050` gives
/// `my-app.localhost`, `http://localhost:20051` gives nothing.
pub(crate) fn host_name_of_url(url: &str) -> Option<String> {
    let rest = URL_SCHEMES
        .iter()
        .find_map(|scheme| url.strip_prefix(scheme))?;
    let authority = rest.split(AUTHORITY_END).next()?;
    if authority.contains('@') {
        return None;
    }
    let host = authority
        .rsplit_once(':')
        .map_or(authority, |(host, _port)| host);
    find_host_name_problem(host)
        .is_none()
        .then(|| host.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_give_the_host_name_when_the_url_names_a_localhost_subdomain() {
        assert_eq!(
            host_name_of_url("http://web-shop-feat-auth-3f2a.localhost:20050"),
            Some("web-shop-feat-auth-3f2a.localhost".to_owned())
        );
    }

    #[test]
    fn should_give_the_host_name_when_the_url_has_a_path_and_no_port() {
        assert_eq!(
            host_name_of_url("https://app.localhost/admin?x=1"),
            Some("app.localhost".to_owned())
        );
    }

    #[test]
    fn should_give_nothing_when_the_host_is_localhost_alone() {
        assert_eq!(host_name_of_url("http://localhost:20051"), None);
    }

    #[test]
    fn should_give_nothing_when_the_host_is_not_under_localhost() {
        assert_eq!(host_name_of_url("http://evil.example:80"), None);
    }

    #[test]
    fn should_give_nothing_when_the_host_is_an_ip_address() {
        assert_eq!(host_name_of_url("http://127.0.0.1:20050"), None);
    }

    #[test]
    fn should_give_nothing_when_the_host_has_an_uppercase_letter() {
        assert_eq!(host_name_of_url("http://App.localhost:80"), None);
    }

    #[test]
    fn should_give_nothing_when_the_url_has_user_information() {
        assert_eq!(host_name_of_url("http://user@app.localhost:80"), None);
    }

    #[test]
    fn should_give_nothing_when_the_scheme_is_not_http() {
        assert_eq!(host_name_of_url("ftp://app.localhost:21"), None);
    }

    #[test]
    fn should_give_nothing_when_a_label_is_too_long() {
        let url = format!("http://{}.localhost:80", "a".repeat(64));

        assert_eq!(host_name_of_url(&url), None);
    }
}
