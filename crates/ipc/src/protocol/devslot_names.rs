//! The names `tunneler devslot <host> <instance>` and the `tunneler://devslot/`
//! link carry, and the bounds they must respect.
//!
//! Both sides of the contract (the command, the link, the daemon) refuse a
//! value outside these bounds. The first character is alphanumeric so that a
//! segment never reads as a command line option.

use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The longest host alias, in characters.
pub const MAX_DEVSLOT_HOST_LENGTH: usize = 64;

/// The longest instance name, in characters.
pub const MAX_DEVSLOT_INSTANCE_LENGTH: usize = 255;

/// Why a host alias or an instance name is out of bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum DevslotNameProblem {
    /// The name is empty.
    #[error("it is empty")]
    Empty,
    /// The name is longer than the bound.
    #[error("it is longer than {max} characters")]
    TooLong {
        /// The longest length allowed.
        max: usize,
    },
    /// The first character is not an ASCII letter or digit.
    #[error("it must start with a letter or a digit")]
    BadFirstCharacter,
    /// A character is not an ASCII letter, a digit, `.`, `_` or `-`.
    #[error("it may only contain letters, digits, `.`, `_` and `-`")]
    BadCharacter,
}

/// What the name is, to say it in the error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Subject {
    Host,
    Instance,
}

impl fmt::Display for Subject {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Host => "host",
            Self::Instance => "instance",
        })
    }
}

/// A host alias or an instance name that is out of bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("invalid devslot {subject} name: {problem}")]
pub struct DevslotNameError {
    subject: Subject,
    /// What is wrong with the name.
    pub problem: DevslotNameProblem,
}

fn find_problem(name: &str, max_length: usize) -> Option<DevslotNameProblem> {
    let Some(first) = name.chars().next() else {
        return Some(DevslotNameProblem::Empty);
    };
    if name.len() > max_length {
        return Some(DevslotNameProblem::TooLong { max: max_length });
    }
    if !first.is_ascii_alphanumeric() {
        return Some(DevslotNameProblem::BadFirstCharacter);
    }
    let is_clean = name
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-'));
    (!is_clean).then_some(DevslotNameProblem::BadCharacter)
}

/// What is wrong with a host alias, when something is: 1 to
/// [`MAX_DEVSLOT_HOST_LENGTH`] characters of `[A-Za-z0-9._-]`, the first one
/// alphanumeric.
pub fn find_devslot_host_name_problem(name: &str) -> Option<DevslotNameProblem> {
    find_problem(name, MAX_DEVSLOT_HOST_LENGTH)
}

/// What is wrong with an instance name, when something is: 1 to
/// [`MAX_DEVSLOT_INSTANCE_LENGTH`] characters of `[A-Za-z0-9._-]`, the first
/// one alphanumeric.
pub fn find_devslot_instance_name_problem(name: &str) -> Option<DevslotNameProblem> {
    find_problem(name, MAX_DEVSLOT_INSTANCE_LENGTH)
}

/// An SSH alias that respects the bounds of the devslot contract. It
/// deserializes from a string only when it is valid.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct DevslotHostName(String);

impl DevslotHostName {
    /// Checks `name` against the bounds.
    ///
    /// # Errors
    ///
    /// Fails when the name is out of bounds.
    pub fn parse(name: &str) -> Result<Self, DevslotNameError> {
        Self::try_from(name.to_owned())
    }

    /// The alias.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for DevslotHostName {
    type Error = DevslotNameError;

    fn try_from(name: String) -> Result<Self, Self::Error> {
        match find_devslot_host_name_problem(&name) {
            Some(problem) => Err(DevslotNameError {
                subject: Subject::Host,
                problem,
            }),
            None => Ok(Self(name)),
        }
    }
}

impl From<DevslotHostName> for String {
    fn from(name: DevslotHostName) -> Self {
        name.0
    }
}

impl AsRef<str> for DevslotHostName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DevslotHostName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// The name of a devslot instance that respects the bounds of the devslot
/// contract. It deserializes from a string only when it is valid.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct DevslotInstanceName(String);

impl DevslotInstanceName {
    /// Checks `name` against the bounds.
    ///
    /// # Errors
    ///
    /// Fails when the name is out of bounds.
    pub fn parse(name: &str) -> Result<Self, DevslotNameError> {
        Self::try_from(name.to_owned())
    }

    /// The name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for DevslotInstanceName {
    type Error = DevslotNameError;

    fn try_from(name: String) -> Result<Self, Self::Error> {
        match find_devslot_instance_name_problem(&name) {
            Some(problem) => Err(DevslotNameError {
                subject: Subject::Instance,
                problem,
            }),
            None => Ok(Self(name)),
        }
    }
}

impl From<DevslotInstanceName> for String {
    fn from(name: DevslotInstanceName) -> Self {
        name.0
    }
}

impl AsRef<str> for DevslotInstanceName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DevslotInstanceName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn should_accept_a_host_of_64_characters() {
        assert!(DevslotHostName::parse(&"a".repeat(64)).is_ok());
    }

    #[test]
    fn should_refuse_a_host_of_65_characters() {
        assert_eq!(
            DevslotHostName::parse(&"a".repeat(65)).unwrap_err().problem,
            DevslotNameProblem::TooLong { max: 64 }
        );
    }

    #[test]
    fn should_accept_an_instance_of_255_characters() {
        assert!(DevslotInstanceName::parse(&"a".repeat(255)).is_ok());
    }

    #[test]
    fn should_refuse_an_instance_of_256_characters() {
        assert_eq!(
            DevslotInstanceName::parse(&"a".repeat(256))
                .unwrap_err()
                .problem,
            DevslotNameProblem::TooLong { max: 255 }
        );
    }

    #[test]
    fn should_refuse_an_empty_name() {
        assert_eq!(
            (
                DevslotHostName::parse("").unwrap_err().problem,
                DevslotInstanceName::parse("").unwrap_err().problem
            ),
            (DevslotNameProblem::Empty, DevslotNameProblem::Empty)
        );
    }

    #[test]
    fn should_refuse_a_name_that_starts_with_a_dash_a_dot_or_an_underscore() {
        let problems: Vec<_> = ["-vega", ".vega", "_vega"]
            .into_iter()
            .flat_map(|name| {
                [
                    find_devslot_host_name_problem(name),
                    find_devslot_instance_name_problem(name),
                ]
            })
            .collect();

        assert_eq!(
            problems,
            vec![Some(DevslotNameProblem::BadFirstCharacter); 6]
        );
    }

    #[test]
    fn should_refuse_a_name_with_a_character_outside_the_alphabet() {
        let problems: Vec<_> = ["ve/ga", "ve ga", "véga", "ve:ga"]
            .into_iter()
            .map(find_devslot_instance_name_problem)
            .collect();

        assert_eq!(problems, vec![Some(DevslotNameProblem::BadCharacter); 4]);
    }

    #[test]
    fn should_accept_dots_dashes_and_underscores_inside_a_name() {
        assert!(DevslotInstanceName::parse("web-shop_feat.auth-098a").is_ok());
    }

    #[test]
    fn should_serialize_a_name_as_a_plain_string() {
        let name = DevslotHostName::parse("vega").unwrap();

        assert_eq!(serde_json::to_string(&name).unwrap(), r#""vega""#);
    }

    #[test]
    fn should_not_deserialize_a_name_that_is_out_of_bounds() {
        assert!(serde_json::from_str::<DevslotInstanceName>(r#""-web-shop""#).is_err());
    }

    #[test]
    fn should_name_the_subject_when_a_name_is_refused() {
        assert_eq!(
            DevslotHostName::parse("-x").unwrap_err().to_string(),
            "invalid devslot host name: it must start with a letter or a digit"
        );
    }
}
