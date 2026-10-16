//! The credential answer as the front end sends it.

use ipc::protocol::{CredentialAnswer, Secret};
use serde::Deserialize;
use zeroize::Zeroizing;

/// A string, or one string per keyboard-interactive question.
///
/// Its strings are wiped when it is dropped, so converting it leaves no plain
/// copy behind (the JSON buffers Tauri parsed it from excepted).
#[derive(Deserialize)]
#[serde(untagged)]
pub enum CredentialAnswerInput {
    Single(Zeroizing<String>),
    Multiple(Vec<Zeroizing<String>>),
}

impl From<CredentialAnswerInput> for CredentialAnswer {
    fn from(input: CredentialAnswerInput) -> Self {
        match input {
            CredentialAnswerInput::Single(answer) => Self::Single(Secret::from(answer)),
            CredentialAnswerInput::Multiple(answers) => {
                Self::Multiple(answers.into_iter().map(Secret::from).collect())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn convert(json: &str) -> CredentialAnswer {
        serde_json::from_str::<CredentialAnswerInput>(json)
            .unwrap()
            .into()
    }

    #[test]
    fn should_convert_to_a_single_secret_when_the_input_is_a_string() {
        let answer = convert("\"hunter2\"");

        assert_eq!(
            answer,
            CredentialAnswer::Single(Secret::new("hunter2".to_owned()))
        );
    }

    #[test]
    fn should_convert_to_multiple_secrets_when_the_input_is_an_array() {
        let answer = convert("[\"one\", \"two\"]");

        assert_eq!(
            answer,
            CredentialAnswer::Multiple(vec![
                Secret::new("one".to_owned()),
                Secret::new("two".to_owned())
            ])
        );
    }

    #[test]
    fn should_reject_the_input_when_it_is_a_number() {
        assert!(serde_json::from_str::<CredentialAnswerInput>("42").is_err());
    }
}
