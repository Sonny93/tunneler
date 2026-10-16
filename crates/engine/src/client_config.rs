use std::borrow::Cow;
use std::time::Duration;

use russh::Preferred;
use russh::client::Config;
use ssh_config::ResolvedConfig;

use crate::host_key::order_host_key_algorithms;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KeepaliveSettings {
    pub(crate) interval: Option<Duration>,
    pub(crate) maximum_unanswered: usize,
}

pub(crate) fn keepalive_settings(
    server_alive_interval: Duration,
    server_alive_count_max: u32,
) -> KeepaliveSettings {
    let interval = (!server_alive_interval.is_zero()).then_some(server_alive_interval);
    KeepaliveSettings {
        interval,
        maximum_unanswered: usize::try_from(server_alive_count_max).unwrap_or(usize::MAX),
    }
}

pub(crate) fn build_client_config(
    resolved: &ResolvedConfig,
    known_algorithm_names: &[String],
) -> Config {
    let keepalive = keepalive_settings(
        resolved.server_alive_interval,
        resolved.server_alive_count_max,
    );
    let preferred = Preferred {
        key: Cow::Owned(order_host_key_algorithms(
            known_algorithm_names,
            &Preferred::DEFAULT.key,
        )),
        ..Preferred::DEFAULT
    };
    Config {
        keepalive_interval: keepalive.interval,
        keepalive_max: keepalive.maximum_unanswered,
        preferred,
        ..Config::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_disable_keepalive_when_interval_is_zero() {
        assert_eq!(keepalive_settings(Duration::ZERO, 3).interval, None);
    }

    #[test]
    fn should_use_the_interval_when_it_is_not_zero() {
        assert_eq!(
            keepalive_settings(Duration::from_secs(15), 3).interval,
            Some(Duration::from_secs(15))
        );
    }

    #[test]
    fn should_use_the_count_max_as_maximum_unanswered_keepalives() {
        assert_eq!(
            keepalive_settings(Duration::from_secs(15), 5).maximum_unanswered,
            5
        );
    }
}
