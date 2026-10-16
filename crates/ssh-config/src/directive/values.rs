use std::time::Duration;

use crate::environment::LocalEnvironment;
use crate::error::ValueError;
use crate::tokens::expand_environment;

const NONE_VALUE: &str = "none";
const MAX_CONVERTED_SECONDS: i64 = i32::MAX as i64;
const SECONDS_PER_MINUTE: i64 = 60;
const SECONDS_PER_HOUR: i64 = 60 * SECONDS_PER_MINUTE;
const SECONDS_PER_DAY: i64 = 24 * SECONDS_PER_HOUR;
const SECONDS_PER_WEEK: i64 = 7 * SECONDS_PER_DAY;
const ENVIRONMENT_REFERENCE_START: &str = "${";
const ENVIRONMENT_REFERENCE_PREFIX: char = '$';
const ASSIGNMENT_SEPARATOR: char = '=';
const REMOVAL_PREFIX: char = '-';

/// `StrictHostKeyChecking`. `no` and `off` are the same setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrictHostKeyChecking {
    /// Refuse unknown and changed host keys.
    Yes,
    /// Accept any host key (`no` or `off`).
    No,
    /// Ask before accepting a new host key.
    Ask,
    /// Accept new host keys, refuse changed ones.
    AcceptNew,
}

/// `AddressFamily`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressFamily {
    /// IPv4 and IPv6.
    Any,
    /// IPv4 only.
    Inet,
    /// IPv6 only.
    Inet6,
}

/// `PubkeyAuthentication`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PubkeyAuthentication {
    /// Public key authentication is enabled.
    Yes,
    /// Public key authentication is disabled.
    No,
    /// Enabled without host binding.
    Unbound,
    /// Enabled with host binding.
    HostBound,
}

/// The mode part of `AddKeysToAgent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddKeysMode {
    /// Never add keys.
    No,
    /// Add keys without asking.
    Yes,
    /// Ask before adding a key.
    Ask,
    /// Ask for confirmation at each use of the key.
    Confirm,
}

/// `AddKeysToAgent`: a mode and an optional key lifetime in the agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddKeysToAgent {
    /// What to do with keys.
    pub mode: AddKeysMode,
    /// How long keys stay in the agent; `None` for unlimited.
    pub lifetime: Option<Duration>,
}

/// `CanonicalizeHostname`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanonicalizeHostname {
    /// No canonicalization.
    No,
    /// Canonicalize unless a proxy is used.
    Yes,
    /// Canonicalize even with a proxy.
    Always,
}

/// A `ForwardAgent` argument: a boolean or the agent socket to forward.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ForwardAgentValue {
    Flag(bool),
    Socket(String),
}

/// One argument of `SendEnv`: a pattern to add or, with a leading `-`, to remove.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SendEnvEntry {
    Add(String),
    Remove(String),
}

/// A `NAME=value` pair from `SetEnv`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentVariable {
    /// The variable name.
    pub name: String,
    /// The value, with tokens and `${VAR}` references expanded once resolved.
    pub value: String,
}

pub(crate) fn single_argument(arguments: &[String]) -> Result<&str, ValueError> {
    let Some((first, rest)) = arguments.split_first() else {
        return Err(ValueError::MissingArgument);
    };
    if first.is_empty() {
        return Err(ValueError::MissingArgument);
    }
    if !rest.is_empty() {
        return Err(ValueError::ExtraArguments);
    }
    Ok(first)
}

pub(crate) fn parse_text(arguments: &[String]) -> Result<String, ValueError> {
    single_argument(arguments).map(str::to_owned)
}

fn parse_choice<T: Copy>(arguments: &[String], choices: &[(&str, T)]) -> Result<T, ValueError> {
    let argument = single_argument(arguments)?;
    choices
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(argument))
        .map(|(_, value)| *value)
        .ok_or_else(|| ValueError::UnsupportedValue {
            value: argument.to_owned(),
        })
}

const FLAG_CHOICES: &[(&str, bool)] = &[
    ("true", true),
    ("false", false),
    ("yes", true),
    ("no", false),
];

pub(crate) fn parse_flag(arguments: &[String]) -> Result<bool, ValueError> {
    parse_choice(arguments, FLAG_CHOICES)
}

pub(crate) fn parse_compression(arguments: &[String]) -> Result<bool, ValueError> {
    parse_choice(arguments, &[("yes", true), ("no", false)])
}

pub(crate) fn parse_strict_host_key_checking(
    arguments: &[String],
) -> Result<StrictHostKeyChecking, ValueError> {
    parse_choice(
        arguments,
        &[
            ("true", StrictHostKeyChecking::Yes),
            ("yes", StrictHostKeyChecking::Yes),
            ("false", StrictHostKeyChecking::No),
            ("no", StrictHostKeyChecking::No),
            ("off", StrictHostKeyChecking::No),
            ("ask", StrictHostKeyChecking::Ask),
            ("accept-new", StrictHostKeyChecking::AcceptNew),
        ],
    )
}

pub(crate) fn parse_address_family(arguments: &[String]) -> Result<AddressFamily, ValueError> {
    parse_choice(
        arguments,
        &[
            ("any", AddressFamily::Any),
            ("inet", AddressFamily::Inet),
            ("inet6", AddressFamily::Inet6),
        ],
    )
}

pub(crate) fn parse_pubkey_authentication(
    arguments: &[String],
) -> Result<PubkeyAuthentication, ValueError> {
    parse_choice(
        arguments,
        &[
            ("true", PubkeyAuthentication::Yes),
            ("yes", PubkeyAuthentication::Yes),
            ("false", PubkeyAuthentication::No),
            ("no", PubkeyAuthentication::No),
            ("unbound", PubkeyAuthentication::Unbound),
            ("host-bound", PubkeyAuthentication::HostBound),
        ],
    )
}

pub(crate) fn parse_canonicalize_hostname(
    arguments: &[String],
) -> Result<CanonicalizeHostname, ValueError> {
    parse_choice(
        arguments,
        &[
            ("true", CanonicalizeHostname::Yes),
            ("yes", CanonicalizeHostname::Yes),
            ("false", CanonicalizeHostname::No),
            ("no", CanonicalizeHostname::No),
            ("always", CanonicalizeHostname::Always),
        ],
    )
}

/// Converts a time such as `90`, `5m` or `1h30m` to seconds, like OpenSSH `convtime`.
pub(crate) fn convert_time(text: &str) -> Option<u32> {
    if text.is_empty() {
        return None;
    }
    let mut remaining = text;
    let mut total_seconds: i64 = 0;
    while !remaining.is_empty() {
        let (seconds, rest) = convert_time_segment(remaining)?;
        total_seconds += seconds;
        if total_seconds > MAX_CONVERTED_SECONDS {
            return None;
        }
        remaining = rest;
    }
    u32::try_from(total_seconds).ok()
}

fn convert_time_segment(segment: &str) -> Option<(i64, &str)> {
    let digits_end = segment
        .find(|character: char| !character.is_ascii_digit())
        .unwrap_or(segment.len());
    let amount: i64 = segment[..digits_end].parse().ok()?;
    if amount > MAX_CONVERTED_SECONDS {
        return None;
    }
    let after_digits = &segment[digits_end..];
    let Some(qualifier) = after_digits.chars().next() else {
        return Some((amount, after_digits));
    };
    let multiplier = time_multiplier(qualifier)?;
    let seconds = amount.checked_mul(multiplier)?;
    if seconds > MAX_CONVERTED_SECONDS {
        return None;
    }
    Some((seconds, &after_digits[qualifier.len_utf8()..]))
}

fn time_multiplier(qualifier: char) -> Option<i64> {
    match qualifier.to_ascii_lowercase() {
        's' => Some(1),
        'm' => Some(SECONDS_PER_MINUTE),
        'h' => Some(SECONDS_PER_HOUR),
        'd' => Some(SECONDS_PER_DAY),
        'w' => Some(SECONDS_PER_WEEK),
        _ => None,
    }
}

/// A time directive value; `None` stands for `none`, which leaves the option unset.
pub(crate) fn parse_seconds(arguments: &[String]) -> Result<Option<u32>, ValueError> {
    let argument = single_argument(arguments)?;
    if argument == NONE_VALUE {
        return Ok(None);
    }
    convert_time(argument)
        .map(Some)
        .ok_or_else(|| ValueError::InvalidTime {
            value: argument.to_owned(),
        })
}

pub(crate) fn parse_count(arguments: &[String]) -> Result<u32, ValueError> {
    let argument = single_argument(arguments)?;
    argument
        .parse::<u32>()
        .ok()
        .filter(|count| i64::from(*count) <= MAX_CONVERTED_SECONDS)
        .ok_or_else(|| ValueError::InvalidInteger {
            value: argument.to_owned(),
        })
}

/// A numeric port, as accepted by OpenSSH `a2port` (service names are not supported).
pub(crate) fn parse_port_number(text: &str) -> Option<u16> {
    text.parse::<u16>().ok()
}

pub(crate) fn parse_port(arguments: &[String]) -> Result<u16, ValueError> {
    let argument = single_argument(arguments)?;
    parse_port_number(argument)
        .filter(|port| *port != 0)
        .ok_or_else(|| ValueError::InvalidPort {
            value: argument.to_owned(),
        })
}

pub(crate) fn parse_add_keys_to_agent(arguments: &[String]) -> Result<AddKeysToAgent, ValueError> {
    let Some(first) = arguments.first() else {
        return Err(ValueError::MissingArgument);
    };
    let second = arguments.get(1);
    if arguments.len() > 2 {
        return Err(ValueError::ExtraArguments);
    }
    let mode = match first.to_ascii_lowercase().as_str() {
        "true" | "yes" => Some(AddKeysMode::Yes),
        "false" | "no" => Some(AddKeysMode::No),
        "ask" => Some(AddKeysMode::Ask),
        "confirm" => Some(AddKeysMode::Confirm),
        _ => None,
    };
    match (mode, second) {
        (Some(AddKeysMode::Confirm), Some(lifetime)) => Ok(agent_setting(
            AddKeysMode::Confirm,
            parse_lifetime(lifetime)?,
        )),
        (None, None) => Ok(agent_setting(AddKeysMode::Yes, parse_lifetime(first)?)),
        (Some(mode), None) => Ok(agent_setting(mode, 0)),
        _ => Err(ValueError::UnsupportedValue {
            value: arguments.join(" "),
        }),
    }
}

fn parse_lifetime(text: &str) -> Result<u32, ValueError> {
    convert_time(text).ok_or_else(|| ValueError::InvalidTime {
        value: text.to_owned(),
    })
}

fn agent_setting(mode: AddKeysMode, lifetime_seconds: u32) -> AddKeysToAgent {
    AddKeysToAgent {
        mode,
        lifetime: (lifetime_seconds > 0).then(|| Duration::from_secs(u64::from(lifetime_seconds))),
    }
}

pub(crate) fn is_valid_environment_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
}

/// Checks an agent socket argument the way `IdentityAgent`/`ForwardAgent` do:
/// `${VAR}` references must expand and a legacy `$NAME` must be a valid name.
pub(crate) fn validate_agent_socket(
    argument: &str,
    environment: &dyn LocalEnvironment,
) -> Result<(), ValueError> {
    expand_environment(argument, environment).map_err(|_| {
        ValueError::InvalidEnvironmentExpansion {
            value: argument.to_owned(),
        }
    })?;
    let Some(legacy_name) = argument.strip_prefix(ENVIRONMENT_REFERENCE_PREFIX) else {
        return Ok(());
    };
    if argument.starts_with(ENVIRONMENT_REFERENCE_START) || is_valid_environment_name(legacy_name) {
        return Ok(());
    }
    Err(ValueError::InvalidEnvironmentName {
        value: argument.to_owned(),
    })
}

pub(crate) fn parse_identity_agent(
    arguments: &[String],
    environment: &dyn LocalEnvironment,
) -> Result<String, ValueError> {
    let argument = single_argument(arguments)?;
    validate_agent_socket(argument, environment)?;
    Ok(argument.to_owned())
}

pub(crate) fn parse_forward_agent(
    arguments: &[String],
    environment: &dyn LocalEnvironment,
) -> Result<ForwardAgentValue, ValueError> {
    let argument = single_argument(arguments)?;
    if let Some(flag) = FLAG_CHOICES
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(argument))
        .map(|(_, flag)| *flag)
    {
        return Ok(ForwardAgentValue::Flag(flag));
    }
    validate_agent_socket(argument, environment)?;
    Ok(ForwardAgentValue::Socket(argument.to_owned()))
}

/// `UserKnownHostsFile`/`GlobalKnownHostsFile`: paths, with `none` only alone.
pub(crate) fn parse_path_list(arguments: &[String]) -> Result<Vec<String>, ValueError> {
    for (index, argument) in arguments.iter().enumerate() {
        if argument.is_empty() {
            return Err(ValueError::EmptyArgument);
        }
        let is_alone = index == 0 && arguments.len() == 1;
        if argument.eq_ignore_ascii_case(NONE_VALUE) && !is_alone {
            return Err(ValueError::NoneMustAppearAlone);
        }
    }
    Ok(arguments.to_vec())
}

pub(crate) fn parse_send_env(arguments: &[String]) -> Result<Vec<SendEnvEntry>, ValueError> {
    if arguments.is_empty() {
        return Err(ValueError::MissingArgument);
    }
    arguments
        .iter()
        .map(|argument| parse_send_env_entry(argument))
        .collect()
}

fn parse_send_env_entry(argument: &str) -> Result<SendEnvEntry, ValueError> {
    if argument.is_empty() || argument.contains(ASSIGNMENT_SEPARATOR) {
        return Err(ValueError::InvalidEnvironmentName {
            value: argument.to_owned(),
        });
    }
    Ok(match argument.strip_prefix(REMOVAL_PREFIX) {
        Some(pattern) => SendEnvEntry::Remove(pattern.to_owned()),
        None => SendEnvEntry::Add(argument.to_owned()),
    })
}

pub(crate) fn parse_set_env(arguments: &[String]) -> Result<Vec<EnvironmentVariable>, ValueError> {
    let mut variables: Vec<EnvironmentVariable> = Vec::new();
    for argument in arguments {
        let Some((name, value)) = argument.split_once(ASSIGNMENT_SEPARATOR) else {
            return Err(ValueError::InvalidEnvironmentAssignment {
                value: argument.clone(),
            });
        };
        if variables.iter().any(|existing| existing.name == name) {
            continue;
        }
        variables.push(EnvironmentVariable {
            name: name.to_owned(),
            value: value.to_owned(),
        });
    }
    if variables.is_empty() {
        return Err(ValueError::MissingArgument);
    }
    Ok(variables)
}

pub(crate) fn parse_include_patterns(arguments: &[String]) -> Result<Vec<String>, ValueError> {
    if arguments.iter().any(String::is_empty) {
        return Err(ValueError::EmptyArgument);
    }
    Ok(arguments.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::fake::FakeEnvironment;

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn should_parse_true_when_flag_is_yes() {
        assert_eq!(parse_flag(&strings(&["yes"])), Ok(true));
    }

    #[test]
    fn should_parse_true_when_flag_is_true_in_any_case() {
        assert_eq!(parse_flag(&strings(&["TrUe"])), Ok(true));
    }

    #[test]
    fn should_parse_false_when_flag_is_no() {
        assert_eq!(parse_flag(&strings(&["no"])), Ok(false));
    }

    #[test]
    fn should_parse_false_when_flag_is_false() {
        assert_eq!(parse_flag(&strings(&["false"])), Ok(false));
    }

    #[test]
    fn should_fail_when_flag_is_not_a_boolean() {
        assert_eq!(
            parse_flag(&strings(&["maybe"])),
            Err(ValueError::UnsupportedValue {
                value: "maybe".to_owned()
            })
        );
    }

    #[test]
    fn should_fail_when_flag_has_no_argument() {
        assert_eq!(parse_flag(&[]), Err(ValueError::MissingArgument));
    }

    #[test]
    fn should_fail_when_flag_has_an_extra_argument() {
        assert_eq!(
            parse_flag(&strings(&["yes", "no"])),
            Err(ValueError::ExtraArguments)
        );
    }

    #[test]
    fn should_reject_true_when_parsing_compression() {
        assert!(parse_compression(&strings(&["true"])).is_err());
    }

    #[test]
    fn should_accept_yes_when_parsing_compression() {
        assert_eq!(parse_compression(&strings(&["yes"])), Ok(true));
    }

    #[test]
    fn should_map_off_to_no_when_parsing_strict_host_key_checking() {
        assert_eq!(
            parse_strict_host_key_checking(&strings(&["off"])),
            Ok(StrictHostKeyChecking::No)
        );
    }

    #[test]
    fn should_parse_accept_new_when_parsing_strict_host_key_checking() {
        assert_eq!(
            parse_strict_host_key_checking(&strings(&["accept-new"])),
            Ok(StrictHostKeyChecking::AcceptNew)
        );
    }

    #[test]
    fn should_parse_ask_when_parsing_strict_host_key_checking() {
        assert_eq!(
            parse_strict_host_key_checking(&strings(&["ASK"])),
            Ok(StrictHostKeyChecking::Ask)
        );
    }

    #[test]
    fn should_fail_when_strict_host_key_checking_is_unknown() {
        assert!(parse_strict_host_key_checking(&strings(&["sometimes"])).is_err());
    }

    #[test]
    fn should_parse_inet6_when_parsing_address_family() {
        assert_eq!(
            parse_address_family(&strings(&["inet6"])),
            Ok(AddressFamily::Inet6)
        );
    }

    #[test]
    fn should_fail_when_address_family_is_unknown() {
        assert!(parse_address_family(&strings(&["ipx"])).is_err());
    }

    #[test]
    fn should_parse_host_bound_when_parsing_pubkey_authentication() {
        assert_eq!(
            parse_pubkey_authentication(&strings(&["host-bound"])),
            Ok(PubkeyAuthentication::HostBound)
        );
    }

    #[test]
    fn should_parse_always_when_parsing_canonicalize_hostname() {
        assert_eq!(
            parse_canonicalize_hostname(&strings(&["always"])),
            Ok(CanonicalizeHostname::Always)
        );
    }

    #[test]
    fn should_convert_plain_number_when_it_has_no_unit() {
        assert_eq!(convert_time("90"), Some(90));
    }

    #[test]
    fn should_convert_minutes_when_unit_is_m() {
        assert_eq!(convert_time("5m"), Some(300));
    }

    #[test]
    fn should_convert_compound_time_when_units_are_chained() {
        assert_eq!(convert_time("1h30m"), Some(5400));
    }

    #[test]
    fn should_convert_weeks_when_unit_is_uppercase_w() {
        assert_eq!(convert_time("1W"), Some(604_800));
    }

    #[test]
    fn should_reject_time_when_unit_is_unknown() {
        assert_eq!(convert_time("5x"), None);
    }

    #[test]
    fn should_reject_time_when_it_has_no_digits() {
        assert_eq!(convert_time("m"), None);
    }

    #[test]
    fn should_reject_time_when_it_is_empty() {
        assert_eq!(convert_time(""), None);
    }

    #[test]
    fn should_reject_time_when_it_overflows() {
        assert_eq!(convert_time("99999999999"), None);
    }

    #[test]
    fn should_reject_time_when_total_overflows() {
        assert_eq!(convert_time("2147483647s1s"), None);
    }

    #[test]
    fn should_leave_seconds_unset_when_value_is_none() {
        assert_eq!(parse_seconds(&strings(&["none"])), Ok(None));
    }

    #[test]
    fn should_parse_seconds_when_value_is_a_time() {
        assert_eq!(parse_seconds(&strings(&["2m"])), Ok(Some(120)));
    }

    #[test]
    fn should_fail_when_seconds_value_is_invalid() {
        assert_eq!(
            parse_seconds(&strings(&["soon"])),
            Err(ValueError::InvalidTime {
                value: "soon".to_owned()
            })
        );
    }

    #[test]
    fn should_parse_count_when_value_is_a_non_negative_integer() {
        assert_eq!(parse_count(&strings(&["3"])), Ok(3));
    }

    #[test]
    fn should_fail_when_count_is_negative() {
        assert!(parse_count(&strings(&["-1"])).is_err());
    }

    #[test]
    fn should_fail_when_count_is_not_a_number() {
        assert!(parse_count(&strings(&["many"])).is_err());
    }

    #[test]
    fn should_parse_port_when_value_is_in_range() {
        assert_eq!(parse_port(&strings(&["2222"])), Ok(2222));
    }

    #[test]
    fn should_fail_when_port_is_zero() {
        assert!(parse_port(&strings(&["0"])).is_err());
    }

    #[test]
    fn should_fail_when_port_is_above_65535() {
        assert!(parse_port(&strings(&["65536"])).is_err());
    }

    #[test]
    fn should_fail_when_port_is_a_service_name() {
        assert!(parse_port(&strings(&["ssh"])).is_err());
    }

    #[test]
    fn should_parse_yes_mode_when_add_keys_to_agent_is_yes() {
        assert_eq!(
            parse_add_keys_to_agent(&strings(&["yes"])),
            Ok(AddKeysToAgent {
                mode: AddKeysMode::Yes,
                lifetime: None
            })
        );
    }

    #[test]
    fn should_parse_yes_with_lifetime_when_add_keys_to_agent_is_a_time() {
        assert_eq!(
            parse_add_keys_to_agent(&strings(&["5m"])),
            Ok(AddKeysToAgent {
                mode: AddKeysMode::Yes,
                lifetime: Some(Duration::from_secs(300))
            })
        );
    }

    #[test]
    fn should_parse_confirm_with_lifetime_when_both_are_given() {
        assert_eq!(
            parse_add_keys_to_agent(&strings(&["confirm", "1h"])),
            Ok(AddKeysToAgent {
                mode: AddKeysMode::Confirm,
                lifetime: Some(Duration::from_secs(3600))
            })
        );
    }

    #[test]
    fn should_fail_when_add_keys_to_agent_has_a_lifetime_after_yes() {
        assert!(parse_add_keys_to_agent(&strings(&["yes", "5m"])).is_err());
    }

    #[test]
    fn should_fail_when_add_keys_to_agent_is_garbage() {
        assert!(parse_add_keys_to_agent(&strings(&["sometimes"])).is_err());
    }

    #[test]
    fn should_parse_flag_when_forward_agent_is_a_boolean() {
        let environment = FakeEnvironment::default();
        assert_eq!(
            parse_forward_agent(&strings(&["yes"]), &environment),
            Ok(ForwardAgentValue::Flag(true))
        );
    }

    #[test]
    fn should_parse_socket_when_forward_agent_is_a_path() {
        let environment = FakeEnvironment::default();
        assert_eq!(
            parse_forward_agent(&strings(&["/run/agent.sock"]), &environment),
            Ok(ForwardAgentValue::Socket("/run/agent.sock".to_owned()))
        );
    }

    #[test]
    fn should_accept_legacy_variable_when_agent_socket_starts_with_dollar() {
        let environment = FakeEnvironment::default();
        assert_eq!(
            parse_identity_agent(&strings(&["$MY_AGENT"]), &environment),
            Ok("$MY_AGENT".to_owned())
        );
    }

    #[test]
    fn should_fail_when_legacy_variable_name_is_invalid() {
        let environment = FakeEnvironment::default();
        assert!(parse_identity_agent(&strings(&["$MY-AGENT"]), &environment).is_err());
    }

    #[test]
    fn should_fail_when_agent_socket_references_an_undefined_variable() {
        let environment = FakeEnvironment::default();
        assert!(parse_identity_agent(&strings(&["${NOPE}/sock"]), &environment).is_err());
    }

    #[test]
    fn should_accept_path_list_when_none_appears_alone() {
        assert_eq!(parse_path_list(&strings(&["none"])), Ok(strings(&["none"])));
    }

    #[test]
    fn should_fail_when_none_is_mixed_with_other_paths() {
        assert_eq!(
            parse_path_list(&strings(&["a", "none"])),
            Err(ValueError::NoneMustAppearAlone)
        );
    }

    #[test]
    fn should_fail_when_path_list_has_an_empty_entry() {
        assert_eq!(
            parse_path_list(&strings(&[""])),
            Err(ValueError::EmptyArgument)
        );
    }

    #[test]
    fn should_parse_additions_and_removals_when_parsing_send_env() {
        assert_eq!(
            parse_send_env(&strings(&["LANG", "-LC_*"])),
            Ok(vec![
                SendEnvEntry::Add("LANG".to_owned()),
                SendEnvEntry::Remove("LC_*".to_owned())
            ])
        );
    }

    #[test]
    fn should_fail_when_send_env_name_contains_equals() {
        assert!(parse_send_env(&strings(&["A=b"])).is_err());
    }

    #[test]
    fn should_fail_when_send_env_has_no_argument() {
        assert_eq!(parse_send_env(&[]), Err(ValueError::MissingArgument));
    }

    #[test]
    fn should_split_assignments_when_parsing_set_env() {
        assert_eq!(
            parse_set_env(&strings(&["A=1", "B=x=y"])),
            Ok(vec![
                EnvironmentVariable {
                    name: "A".to_owned(),
                    value: "1".to_owned()
                },
                EnvironmentVariable {
                    name: "B".to_owned(),
                    value: "x=y".to_owned()
                }
            ])
        );
    }

    #[test]
    fn should_keep_first_value_when_set_env_repeats_a_name() {
        let variables = parse_set_env(&strings(&["A=1", "A=2"])).unwrap();
        assert_eq!(variables.len(), 1);
        assert_eq!(variables[0].value, "1");
    }

    #[test]
    fn should_fail_when_set_env_has_no_equals() {
        assert!(parse_set_env(&strings(&["A"])).is_err());
    }

    #[test]
    fn should_fail_when_include_pattern_is_empty() {
        assert_eq!(
            parse_include_patterns(&strings(&[""])),
            Err(ValueError::EmptyArgument)
        );
    }
}
