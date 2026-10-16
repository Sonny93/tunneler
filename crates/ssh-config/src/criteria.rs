use crate::error::CriteriaError;

const NEGATION_MARKER: char = '!';
const COMMENT_MARKER: char = '#';
const VALUE_SEPARATOR: char = '=';
const MAX_ATTRIBUTES_BEFORE_ALL: usize = 1;

/// What a `Match` criterion tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CriterionKind {
    /// `all`: always true.
    All,
    /// `canonical`: true in the final pass.
    Canonical,
    /// `final`: true in the final pass, and requests it.
    Final,
    /// `exec`: a command that must exit with status 0.
    Exec(String),
    /// `host`: comma-separated patterns for the target host name.
    Host(String),
    /// `originalhost`: comma-separated patterns for the alias as given.
    OriginalHost(String),
    /// `user`: comma-separated patterns for the remote user.
    User(String),
    /// `localuser`: comma-separated patterns for the local user.
    LocalUser(String),
    /// `tagged`: comma-separated patterns for the tag; empty matches no tag.
    Tagged(String),
    /// A criterion OpenSSH knows but this crate cannot evaluate; it never matches.
    Unsupported(String),
}

/// One criterion of a `Match` line, possibly negated with `!`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Criterion {
    /// Whether the criterion was written with a leading `!`.
    pub is_negated: bool,
    /// What the criterion tests.
    pub kind: CriterionKind,
}

const VALUE_CRITERIA_WITH_EQUALS_FORM: [&str; 9] = [
    "host",
    "originalhost",
    "user",
    "localuser",
    "localnetwork",
    "version",
    "tagged",
    "command",
    "exec",
];
const UNSUPPORTED_CRITERIA: [&str; 4] = ["localnetwork", "version", "sessiontype", "command"];
const CRITERIA_WITH_OPTIONAL_VALUE: [&str; 2] = ["tagged", "command"];

/// Parses the arguments of a `Match` line, applying OpenSSH `match_cfg_line`
/// validation.
pub(crate) fn parse_criteria(arguments: &[String]) -> Result<Vec<Criterion>, CriteriaError> {
    let mut criteria = Vec::new();
    let mut remaining = arguments.iter();
    let mut attribute_count = 0;
    while let Some(argument) = remaining.next() {
        if argument.starts_with(COMMENT_MARKER) {
            break;
        }
        let (is_negated, name) = split_negation(argument);
        if name.eq_ignore_ascii_case("all") {
            let followed_by_value = remaining
                .next()
                .map(String::as_str)
                .is_some_and(is_value_argument);
            if attribute_count > MAX_ATTRIBUTES_BEFORE_ALL || followed_by_value {
                return Err(CriteriaError::AllCombinedWithOthers {
                    criterion: argument.clone(),
                });
            }
            criteria.push(Criterion {
                is_negated,
                kind: CriterionKind::All,
            });
            return Ok(criteria);
        }
        attribute_count += 1;
        criteria.push(parse_criterion(is_negated, name, &mut remaining)?);
    }
    if attribute_count == 0 {
        return Err(CriteriaError::NoCriteria);
    }
    Ok(criteria)
}

fn is_value_argument(argument: &str) -> bool {
    !argument.is_empty() && !argument.starts_with(COMMENT_MARKER)
}

fn split_negation(argument: &str) -> (bool, &str) {
    match argument.strip_prefix(NEGATION_MARKER) {
        Some(name) => (true, name),
        None => (false, argument),
    }
}

fn parse_criterion<'arguments>(
    is_negated: bool,
    token: &str,
    remaining: &mut impl Iterator<Item = &'arguments String>,
) -> Result<Criterion, CriteriaError> {
    if token.eq_ignore_ascii_case("canonical") {
        return Ok(simple_criterion(is_negated, CriterionKind::Canonical));
    }
    if token.eq_ignore_ascii_case("final") {
        return Ok(simple_criterion(is_negated, CriterionKind::Final));
    }
    let (name, value) = split_name_and_value(token, remaining)?;
    let lowercase_name = name.to_ascii_lowercase();
    let is_value_optional = CRITERIA_WITH_OPTIONAL_VALUE.contains(&lowercase_name.as_str());
    if (value.is_empty() && !is_value_optional) || value.starts_with(COMMENT_MARKER) {
        return Err(CriteriaError::MissingCriteria {
            criterion: name.to_owned(),
        });
    }
    let kind = criterion_kind(&lowercase_name, value, name)?;
    Ok(simple_criterion(is_negated, kind))
}

fn simple_criterion(is_negated: bool, kind: CriterionKind) -> Criterion {
    Criterion { is_negated, kind }
}

fn split_name_and_value<'text, 'arguments: 'text>(
    token: &'text str,
    remaining: &mut impl Iterator<Item = &'arguments String>,
) -> Result<(&'text str, &'text str), CriteriaError> {
    if let Some((name, value)) = token.split_once(VALUE_SEPARATOR)
        && VALUE_CRITERIA_WITH_EQUALS_FORM.contains(&name.to_ascii_lowercase().as_str())
    {
        return Ok((name, value));
    }
    match remaining.next() {
        Some(value) => Ok((token, value.as_str())),
        None => Err(CriteriaError::MissingArgument {
            criterion: token.to_owned(),
        }),
    }
}

fn criterion_kind(
    lowercase_name: &str,
    value: &str,
    original_name: &str,
) -> Result<CriterionKind, CriteriaError> {
    let owned_value = value.to_owned();
    match lowercase_name {
        "host" => Ok(CriterionKind::Host(owned_value)),
        "originalhost" => Ok(CriterionKind::OriginalHost(owned_value)),
        "user" => Ok(CriterionKind::User(owned_value)),
        "localuser" => Ok(CriterionKind::LocalUser(owned_value)),
        "tagged" => Ok(CriterionKind::Tagged(owned_value)),
        "exec" => Ok(CriterionKind::Exec(owned_value)),
        name if UNSUPPORTED_CRITERIA.contains(&name) => {
            Ok(CriterionKind::Unsupported(name.to_owned()))
        }
        _ => Err(CriteriaError::UnknownCriterion {
            criterion: original_name.to_owned(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(line: &str) -> Result<Vec<Criterion>, CriteriaError> {
        parse_criteria(&crate::lexer::split_arguments(line).unwrap())
    }

    fn criterion(kind: CriterionKind) -> Criterion {
        Criterion {
            is_negated: false,
            kind,
        }
    }

    #[test]
    fn should_parse_all_when_it_stands_alone() {
        assert_eq!(parse("all"), Ok(vec![criterion(CriterionKind::All)]));
    }

    #[test]
    fn should_parse_negated_all_when_it_has_a_bang() {
        assert_eq!(
            parse("!all"),
            Ok(vec![Criterion {
                is_negated: true,
                kind: CriterionKind::All
            }])
        );
    }

    #[test]
    fn should_accept_all_after_canonical_when_both_are_given() {
        assert_eq!(parse("canonical all").unwrap().len(), 2);
    }

    #[test]
    fn should_accept_all_after_a_single_attribute_like_openssh() {
        assert_eq!(parse("host x all").unwrap().len(), 2);
    }

    #[test]
    fn should_fail_when_all_follows_two_attributes() {
        assert_eq!(
            parse("host x canonical all"),
            Err(CriteriaError::AllCombinedWithOthers {
                criterion: "all".to_owned()
            })
        );
    }

    #[test]
    fn should_fail_when_all_is_followed_by_an_attribute() {
        assert!(matches!(
            parse("all host x"),
            Err(CriteriaError::AllCombinedWithOthers { .. })
        ));
    }

    #[test]
    fn should_fail_when_all_is_repeated() {
        assert!(matches!(
            parse("all all"),
            Err(CriteriaError::AllCombinedWithOthers { .. })
        ));
    }

    #[test]
    fn should_ignore_trailing_comment_when_all_is_followed_by_one() {
        assert_eq!(parse("all # note"), Ok(vec![criterion(CriterionKind::All)]));
    }

    #[test]
    fn should_parse_final_and_canonical_when_they_have_no_argument() {
        assert_eq!(
            parse("final canonical"),
            Ok(vec![
                criterion(CriterionKind::Final),
                criterion(CriterionKind::Canonical)
            ])
        );
    }

    #[test]
    fn should_parse_host_when_argument_is_a_separate_token() {
        assert_eq!(
            parse("host a,b"),
            Ok(vec![criterion(CriterionKind::Host("a,b".to_owned()))])
        );
    }

    #[test]
    fn should_parse_host_when_written_with_equals() {
        assert_eq!(
            parse("host=a"),
            Ok(vec![criterion(CriterionKind::Host("a".to_owned()))])
        );
    }

    #[test]
    fn should_ignore_case_when_criterion_name_is_uppercase() {
        assert_eq!(
            parse("HOST a"),
            Ok(vec![criterion(CriterionKind::Host("a".to_owned()))])
        );
    }

    #[test]
    fn should_parse_negated_criterion_when_it_has_a_bang() {
        assert_eq!(
            parse("!user bob"),
            Ok(vec![Criterion {
                is_negated: true,
                kind: CriterionKind::User("bob".to_owned())
            }])
        );
    }

    #[test]
    fn should_parse_every_value_criterion_when_they_are_combined() {
        let criteria =
            parse("originalhost o localuser l user u tagged t exec \"test -f x\"").unwrap();
        assert_eq!(
            criteria,
            vec![
                criterion(CriterionKind::OriginalHost("o".to_owned())),
                criterion(CriterionKind::LocalUser("l".to_owned())),
                criterion(CriterionKind::User("u".to_owned())),
                criterion(CriterionKind::Tagged("t".to_owned())),
                criterion(CriterionKind::Exec("test -f x".to_owned())),
            ]
        );
    }

    #[test]
    fn should_accept_empty_value_when_criterion_is_tagged() {
        assert_eq!(
            parse("tagged \"\""),
            Ok(vec![criterion(CriterionKind::Tagged(String::new()))])
        );
    }

    #[test]
    fn should_flag_criterion_as_unsupported_when_openssh_knows_it() {
        assert_eq!(
            parse("version 10.*"),
            Ok(vec![criterion(CriterionKind::Unsupported(
                "version".to_owned()
            ))])
        );
    }

    #[test]
    fn should_flag_localnetwork_as_unsupported_when_written_with_equals() {
        assert_eq!(
            parse("localnetwork=10.0.0.0/8"),
            Ok(vec![criterion(CriterionKind::Unsupported(
                "localnetwork".to_owned()
            ))])
        );
    }

    #[test]
    fn should_flag_sessiontype_as_unsupported_when_it_has_a_separate_argument() {
        assert_eq!(
            parse("sessiontype shell"),
            Ok(vec![criterion(CriterionKind::Unsupported(
                "sessiontype".to_owned()
            ))])
        );
    }

    #[test]
    fn should_fail_when_there_is_no_criterion() {
        assert_eq!(parse("# nothing"), Err(CriteriaError::NoCriteria));
    }

    #[test]
    fn should_fail_when_criterion_has_no_argument() {
        assert_eq!(
            parse("host"),
            Err(CriteriaError::MissingArgument {
                criterion: "host".to_owned()
            })
        );
    }

    #[test]
    fn should_fail_when_equals_is_surrounded_by_spaces() {
        assert!(matches!(
            parse("host = x"),
            Err(CriteriaError::MissingArgument { .. })
        ));
    }

    #[test]
    fn should_fail_when_value_is_empty() {
        assert_eq!(
            parse("host \"\""),
            Err(CriteriaError::MissingCriteria {
                criterion: "host".to_owned()
            })
        );
    }

    #[test]
    fn should_fail_when_value_is_a_quoted_comment() {
        assert!(matches!(
            parse("host \"#x\""),
            Err(CriteriaError::MissingCriteria { .. })
        ));
    }

    #[test]
    fn should_fail_when_criterion_is_unknown() {
        assert_eq!(
            parse("foo bar"),
            Err(CriteriaError::UnknownCriterion {
                criterion: "foo".to_owned()
            })
        );
    }

    #[test]
    fn should_fail_when_unknown_criterion_has_no_argument() {
        assert!(matches!(
            parse("foo"),
            Err(CriteriaError::MissingArgument { .. })
        ));
    }
}
