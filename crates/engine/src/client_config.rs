use std::borrow::Cow;
use std::time::Duration;

use russh::Preferred;
use russh::client::Config;
use ssh_config::ResolvedConfig;

use crate::host_key::order_host_key_algorithms;
use crate::store::Settings;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KeepaliveSettings {
    pub(crate) interval: Option<Duration>,
    pub(crate) maximum_unanswered: usize,
}

/// The keepalive used when `ssh_config` says nothing about it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KeepaliveDefaults {
    pub(crate) interval: Duration,
    pub(crate) maximum_unanswered: u32,
}

impl From<&Settings> for KeepaliveDefaults {
    fn from(settings: &Settings) -> Self {
        Self {
            interval: settings.keepalive_interval,
            maximum_unanswered: settings.keepalive_max,
        }
    }
}

/// The effective keepalive: what `ssh_config` set, else the defaults. A zero
/// interval, from either source, disables keepalive.
pub(crate) fn keepalive_settings(
    server_alive_interval: Option<Duration>,
    server_alive_count_max: Option<u32>,
    defaults: KeepaliveDefaults,
) -> KeepaliveSettings {
    let interval = server_alive_interval.unwrap_or(defaults.interval);
    let maximum_unanswered = server_alive_count_max.unwrap_or(defaults.maximum_unanswered);
    KeepaliveSettings {
        interval: (!interval.is_zero()).then_some(interval),
        maximum_unanswered: usize::try_from(maximum_unanswered).unwrap_or(usize::MAX),
    }
}

pub(crate) fn build_client_config(
    resolved: &ResolvedConfig,
    known_algorithm_names: &[String],
    keepalive_defaults: KeepaliveDefaults,
) -> Config {
    let keepalive = keepalive_settings(
        resolved.server_alive_interval,
        resolved.server_alive_count_max,
        keepalive_defaults,
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

    const DEFAULTS: KeepaliveDefaults = KeepaliveDefaults {
        interval: Duration::from_secs(15),
        maximum_unanswered: 3,
    };

    #[test]
    fn should_use_the_default_interval_when_ssh_config_sets_none() {
        assert_eq!(
            keepalive_settings(None, None, DEFAULTS).interval,
            Some(Duration::from_secs(15))
        );
    }

    #[test]
    fn should_use_the_default_count_when_ssh_config_sets_none() {
        assert_eq!(
            keepalive_settings(None, None, DEFAULTS).maximum_unanswered,
            3
        );
    }

    #[test]
    fn should_prefer_the_ssh_config_interval_when_it_is_set() {
        let interval = Some(Duration::from_secs(90));

        assert_eq!(
            keepalive_settings(interval, None, DEFAULTS).interval,
            interval
        );
    }

    #[test]
    fn should_prefer_the_ssh_config_count_when_it_is_set() {
        assert_eq!(
            keepalive_settings(None, Some(5), DEFAULTS).maximum_unanswered,
            5
        );
    }

    #[test]
    fn should_disable_keepalive_when_ssh_config_sets_a_zero_interval() {
        let explicit_zero = Some(Duration::ZERO);

        assert_eq!(
            keepalive_settings(explicit_zero, None, DEFAULTS).interval,
            None
        );
    }

    #[test]
    fn should_disable_keepalive_when_the_default_interval_is_zero() {
        let defaults = KeepaliveDefaults {
            interval: Duration::ZERO,
            ..DEFAULTS
        };

        assert_eq!(keepalive_settings(None, None, defaults).interval, None);
    }

    #[test]
    fn should_build_the_defaults_from_the_settings() {
        assert_eq!(KeepaliveDefaults::from(&Settings::default()), DEFAULTS);
    }
}
