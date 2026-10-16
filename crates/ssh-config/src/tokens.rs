use sha1_smol::Sha1;

use crate::environment::LocalEnvironment;
use crate::error::TokenError;

const PERCENT: char = '%';
const DOLLAR: char = '$';
const OPENING_BRACE: char = '{';
const CLOSING_BRACE: char = '}';
const HOME_MARKER: char = '~';
const PATH_SEPARATOR: char = '/';

/// Tokens accepted by most path-like directives (`man ssh_config`, TOKENS).
pub(crate) const FULL_TOKENS: &[char] =
    &['C', 'L', 'd', 'h', 'i', 'j', 'k', 'l', 'n', 'p', 'r', 'u'];
/// Tokens accepted by `User`, which cannot refer to itself or to `%C`.
pub(crate) const USER_TOKENS: &[char] = &['L', 'd', 'h', 'i', 'j', 'k', 'l', 'n', 'p', 'u'];
/// Tokens accepted by `Hostname`.
pub(crate) const HOSTNAME_TOKENS: &[char] = &['h'];
/// Tokens accepted by `ProxyCommand` (OpenSSH `sshconnect.c`).
pub(crate) const PROXY_COMMAND_TOKENS: &[char] = &['h', 'k', 'n', 'p', 'r'];

/// Values substituted for `%` tokens.
#[derive(Debug, Clone, Default)]
pub(crate) struct TokenValues {
    pub local_host_name: String,
    pub short_host_name: String,
    pub home_directory: String,
    pub local_user: String,
    pub user_id: String,
    pub original_host: String,
    pub remote_host: String,
    pub port: String,
    pub key_alias: String,
    pub jump_host: String,
    pub remote_user: Option<String>,
    pub connection_hash: Option<String>,
}

impl TokenValues {
    pub(crate) fn from_environment(environment: &dyn LocalEnvironment) -> Self {
        Self {
            local_host_name: environment.host_name(),
            short_host_name: environment.short_host_name(),
            home_directory: environment.home_directory().to_string_lossy().into_owned(),
            local_user: environment.user_name(),
            user_id: environment.user_id().to_string(),
            ..Self::default()
        }
    }

    fn value_of(&self, token: char) -> Option<&str> {
        match token {
            'C' => self.connection_hash.as_deref(),
            'L' => Some(&self.short_host_name),
            'd' => Some(&self.home_directory),
            'h' => Some(&self.remote_host),
            'i' => Some(&self.user_id),
            'j' => Some(&self.jump_host),
            'k' => Some(&self.key_alias),
            'l' => Some(&self.local_host_name),
            'n' => Some(&self.original_host),
            'p' => Some(&self.port),
            'r' => self.remote_user.as_deref(),
            'u' => Some(&self.local_user),
            _ => None,
        }
    }

    /// Fills `%r` and `%C` once the remote user is known.
    pub(crate) fn with_remote_user(mut self, remote_user: String) -> Self {
        self.connection_hash = Some(connection_hash(
            &self.local_host_name,
            &self.remote_host,
            &self.port,
            &remote_user,
            &self.jump_host,
        ));
        self.remote_user = Some(remote_user);
        self
    }
}

/// Hex SHA-1 of `%l%h%p%r%j`, as computed by OpenSSH `ssh_connection_hash`.
pub(crate) fn connection_hash(
    local_host_name: &str,
    remote_host: &str,
    port: &str,
    remote_user: &str,
    jump_host: &str,
) -> String {
    let mut hasher = Sha1::new();
    for part in [local_host_name, remote_host, port, remote_user, jump_host] {
        hasher.update(part.as_bytes());
    }
    hasher.digest().to_string()
}

/// Which substitutions an expansion performs.
struct Expansion<'context> {
    tokens: Option<(&'context [char], &'context TokenValues)>,
    environment: Option<&'context dyn LocalEnvironment>,
}

/// Expands `%` tokens only (OpenSSH `percent_expand`).
pub(crate) fn expand_tokens(
    template: &str,
    allowed_tokens: &[char],
    values: &TokenValues,
) -> Result<String, TokenError> {
    expand(
        template,
        &Expansion {
            tokens: Some((allowed_tokens, values)),
            environment: None,
        },
    )
}

/// Expands `%` tokens and `${VAR}` references (OpenSSH `percent_dollar_expand`).
pub(crate) fn expand_tokens_and_environment(
    template: &str,
    allowed_tokens: &[char],
    values: &TokenValues,
    environment: &dyn LocalEnvironment,
) -> Result<String, TokenError> {
    expand(
        template,
        &Expansion {
            tokens: Some((allowed_tokens, values)),
            environment: Some(environment),
        },
    )
}

/// Expands `${VAR}` references only (OpenSSH `dollar_expand`).
pub(crate) fn expand_environment(
    template: &str,
    environment: &dyn LocalEnvironment,
) -> Result<String, TokenError> {
    expand(
        template,
        &Expansion {
            tokens: None,
            environment: Some(environment),
        },
    )
}

fn expand(template: &str, expansion: &Expansion<'_>) -> Result<String, TokenError> {
    let mut expanded = String::with_capacity(template.len());
    let mut characters = template.chars().peekable();
    while let Some(character) = characters.next() {
        match (character, expansion) {
            (
                DOLLAR,
                Expansion {
                    environment: Some(environment),
                    ..
                },
            ) if characters.peek() == Some(&OPENING_BRACE) => {
                characters.next();
                let name = read_variable_name(&mut characters)?;
                let value = environment
                    .variable(&name)
                    .ok_or(TokenError::UndefinedEnvironmentVariable { name })?;
                expanded.push_str(&value);
            }
            (
                PERCENT,
                Expansion {
                    tokens: Some((allowed_tokens, values)),
                    ..
                },
            ) => {
                let token = characters.next().ok_or(TokenError::DanglingPercent)?;
                expanded.push_str(&substitute_token(token, allowed_tokens, values)?);
            }
            _ => expanded.push(character),
        }
    }
    Ok(expanded)
}

fn read_variable_name(
    characters: &mut std::iter::Peekable<std::str::Chars<'_>>,
) -> Result<String, TokenError> {
    let mut name = String::new();
    loop {
        match characters.next() {
            Some(CLOSING_BRACE) => break,
            Some(character) => name.push(character),
            None => return Err(TokenError::UnterminatedEnvironmentReference { reference: name }),
        }
    }
    if name.is_empty() {
        return Err(TokenError::EmptyEnvironmentVariableName);
    }
    Ok(name)
}

fn substitute_token(
    token: char,
    allowed_tokens: &[char],
    values: &TokenValues,
) -> Result<String, TokenError> {
    if token == PERCENT {
        return Ok(PERCENT.to_string());
    }
    allowed_tokens
        .contains(&token)
        .then(|| values.value_of(token))
        .flatten()
        .map(str::to_owned)
        .ok_or(TokenError::UnknownToken { token })
}

/// Expands a leading `~` or `~/` to the home directory, like OpenSSH
/// `tilde_expand_filename`. `~user` forms are left untouched.
pub(crate) fn expand_tilde(path: &str, home_directory: &str) -> String {
    let Some(after_tilde) = path.strip_prefix(HOME_MARKER) else {
        return path.to_owned();
    };
    if !after_tilde.is_empty() && !after_tilde.starts_with(PATH_SEPARATOR) {
        return path.to_owned();
    }
    let tail = after_tilde.trim_start_matches(PATH_SEPARATOR);
    if home_directory.ends_with(PATH_SEPARATOR) {
        format!("{home_directory}{tail}")
    } else {
        format!("{home_directory}{PATH_SEPARATOR}{tail}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::fake::FakeEnvironment;

    fn sample_values() -> TokenValues {
        TokenValues {
            local_host_name: "laptop.example.org".to_owned(),
            short_host_name: "laptop".to_owned(),
            home_directory: "/home/alice".to_owned(),
            local_user: "alice".to_owned(),
            user_id: "1000".to_owned(),
            original_host: "alias".to_owned(),
            remote_host: "real.example.com".to_owned(),
            port: "2222".to_owned(),
            key_alias: "keyalias".to_owned(),
            jump_host: "jump".to_owned(),
            remote_user: None,
            connection_hash: None,
        }
        .with_remote_user("bob".to_owned())
    }

    fn expand_full(template: &str) -> Result<String, TokenError> {
        expand_tokens(template, FULL_TOKENS, &sample_values())
    }

    #[test]
    fn should_expand_remote_host_when_template_has_h() {
        assert_eq!(expand_full("%h").unwrap(), "real.example.com");
    }

    #[test]
    fn should_expand_original_host_when_template_has_n() {
        assert_eq!(expand_full("%n").unwrap(), "alias");
    }

    #[test]
    fn should_expand_port_when_template_has_p() {
        assert_eq!(expand_full("%p").unwrap(), "2222");
    }

    #[test]
    fn should_expand_remote_user_when_template_has_r() {
        assert_eq!(expand_full("%r").unwrap(), "bob");
    }

    #[test]
    fn should_expand_local_user_when_template_has_u() {
        assert_eq!(expand_full("%u").unwrap(), "alice");
    }

    #[test]
    fn should_expand_home_directory_when_template_has_d() {
        assert_eq!(expand_full("%d").unwrap(), "/home/alice");
    }

    #[test]
    fn should_expand_user_id_when_template_has_i() {
        assert_eq!(expand_full("%i").unwrap(), "1000");
    }

    #[test]
    fn should_expand_full_local_host_when_template_has_l() {
        assert_eq!(expand_full("%l").unwrap(), "laptop.example.org");
    }

    #[test]
    fn should_expand_short_local_host_when_template_has_upper_l() {
        assert_eq!(expand_full("%L").unwrap(), "laptop");
    }

    #[test]
    fn should_expand_jump_host_when_template_has_j() {
        assert_eq!(expand_full("%j").unwrap(), "jump");
    }

    #[test]
    fn should_expand_key_alias_when_template_has_k() {
        assert_eq!(expand_full("%k").unwrap(), "keyalias");
    }

    #[test]
    fn should_expand_percent_when_it_is_doubled() {
        assert_eq!(expand_full("100%%").unwrap(), "100%");
    }

    #[test]
    fn should_hash_the_connection_parameters_when_template_has_upper_c() {
        let expected = connection_hash(
            "laptop.example.org",
            "real.example.com",
            "2222",
            "bob",
            "jump",
        );
        assert_eq!(expand_full("%C").unwrap(), expected);
    }

    #[test]
    fn should_compute_sha1_of_concatenated_parameters_when_hashing_connection() {
        let hash = connection_hash("a", "b", "22", "c", "");
        assert_eq!(hash, "a61adf5088241eb4750ac3aea0e456b9a488f14a");
    }

    #[test]
    fn should_fail_when_token_is_unknown() {
        assert_eq!(
            expand_full("%z"),
            Err(TokenError::UnknownToken { token: 'z' })
        );
    }

    #[test]
    fn should_fail_when_token_is_not_allowed_for_the_directive() {
        assert_eq!(
            expand_tokens("%r", HOSTNAME_TOKENS, &sample_values()),
            Err(TokenError::UnknownToken { token: 'r' })
        );
    }

    #[test]
    fn should_fail_when_remote_user_is_not_known_yet() {
        let mut values = sample_values();
        values.remote_user = None;
        assert_eq!(
            expand_tokens("%r", FULL_TOKENS, &values),
            Err(TokenError::UnknownToken { token: 'r' })
        );
    }

    #[test]
    fn should_fail_when_template_ends_with_a_percent() {
        assert_eq!(expand_full("abc%"), Err(TokenError::DanglingPercent));
    }

    #[test]
    fn should_expand_environment_variable_when_it_is_defined() {
        let mut environment = FakeEnvironment::default();
        environment
            .variables
            .insert("KEYDIR".to_owned(), "/keys".to_owned());
        assert_eq!(
            expand_environment("${KEYDIR}/id", &environment).unwrap(),
            "/keys/id"
        );
    }

    #[test]
    fn should_leave_percent_untouched_when_only_environment_is_expanded() {
        let environment = FakeEnvironment::default();
        assert_eq!(expand_environment("%h", &environment).unwrap(), "%h");
    }

    #[test]
    fn should_leave_dollar_untouched_when_it_has_no_brace() {
        let environment = FakeEnvironment::default();
        assert_eq!(expand_environment("$HOME", &environment).unwrap(), "$HOME");
    }

    #[test]
    fn should_fail_when_environment_variable_is_undefined() {
        let environment = FakeEnvironment::default();
        assert_eq!(
            expand_environment("${MISSING}", &environment),
            Err(TokenError::UndefinedEnvironmentVariable {
                name: "MISSING".to_owned()
            })
        );
    }

    #[test]
    fn should_fail_when_environment_reference_is_not_closed() {
        let environment = FakeEnvironment::default();
        assert_eq!(
            expand_environment("${HOME", &environment),
            Err(TokenError::UnterminatedEnvironmentReference {
                reference: "HOME".to_owned()
            })
        );
    }

    #[test]
    fn should_fail_when_environment_variable_name_is_empty() {
        let environment = FakeEnvironment::default();
        assert_eq!(
            expand_environment("${}", &environment),
            Err(TokenError::EmptyEnvironmentVariableName)
        );
    }

    #[test]
    fn should_not_reexpand_tokens_when_environment_value_contains_a_percent() {
        let mut environment = FakeEnvironment::default();
        environment
            .variables
            .insert("RAW".to_owned(), "%h".to_owned());
        let expanded =
            expand_tokens_and_environment("${RAW}", FULL_TOKENS, &sample_values(), &environment);
        assert_eq!(expanded.unwrap(), "%h");
    }

    #[test]
    fn should_expand_tokens_and_environment_when_both_are_present() {
        let mut environment = FakeEnvironment::default();
        environment
            .variables
            .insert("ROOT".to_owned(), "/keys".to_owned());
        let expanded = expand_tokens_and_environment(
            "${ROOT}/k_%r",
            FULL_TOKENS,
            &sample_values(),
            &environment,
        );
        assert_eq!(expanded.unwrap(), "/keys/k_bob");
    }

    #[test]
    fn should_expand_home_when_path_is_a_lone_tilde() {
        assert_eq!(expand_tilde("~", "/home/alice"), "/home/alice/");
    }

    #[test]
    fn should_expand_home_when_path_starts_with_tilde_slash() {
        assert_eq!(
            expand_tilde("~/.ssh/id", "/home/alice"),
            "/home/alice/.ssh/id"
        );
    }

    #[test]
    fn should_collapse_slashes_when_tilde_is_followed_by_several() {
        assert_eq!(expand_tilde("~//x", "/home/alice"), "/home/alice/x");
    }

    #[test]
    fn should_not_duplicate_slash_when_home_ends_with_one() {
        assert_eq!(expand_tilde("~/x", "/home/alice/"), "/home/alice/x");
    }

    #[test]
    fn should_leave_path_untouched_when_tilde_names_another_user() {
        assert_eq!(expand_tilde("~bob/x", "/home/alice"), "~bob/x");
    }

    #[test]
    fn should_leave_path_untouched_when_it_does_not_start_with_tilde() {
        assert_eq!(expand_tilde("/a/~/b", "/home/alice"), "/a/~/b");
    }
}
