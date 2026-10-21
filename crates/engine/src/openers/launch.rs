use crate::opening::OpenAddress;
use crate::store::Opener;

const HOST_PLACEHOLDER: &str = "{host}";
const PORT_PLACEHOLDER: &str = "{port}";
const URL_PLACEHOLDER: &str = "{url}";

/// A program and its arguments, run without a shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchCommand {
    /// The program.
    pub program: String,
    /// The arguments, placeholders already replaced.
    pub args: Vec<String>,
}

impl LaunchCommand {
    /// The command line as shown to the user; arguments with whitespace are
    /// quoted for reading only.
    pub fn display(&self) -> String {
        std::iter::once(self.program.as_str())
            .chain(self.args.iter().map(String::as_str))
            .map(quote_for_display)
            .collect::<Vec<_>>()
            .join(" ")
    }
}

fn quote_for_display(text: &str) -> String {
    if text.contains(char::is_whitespace) {
        return format!("\"{text}\"");
    }
    text.to_owned()
}

/// How a tunnel is opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenPlan {
    /// In the default browser.
    Browser {
        /// The URL.
        url: String,
    },
    /// With a program.
    Program {
        /// The name of the opener, for messages.
        name: String,
        /// What to run.
        command: LaunchCommand,
    },
}

/// The plan of an opener once the address of the tunnel is known.
pub(crate) fn plan_open(opener: Option<&Opener>, address: &OpenAddress) -> OpenPlan {
    let Some(opener) = opener else {
        return OpenPlan::Browser {
            url: address.url.clone(),
        };
    };
    OpenPlan::Program {
        name: opener.name.clone(),
        command: LaunchCommand {
            program: opener.command.clone(),
            args: opener
                .args
                .iter()
                .map(|argument| substitute(argument, address))
                .collect(),
        },
    }
}

fn substitute(argument: &str, address: &OpenAddress) -> String {
    argument
        .replace(HOST_PLACEHOLDER, &address.host)
        .replace(PORT_PLACEHOLDER, &address.port.to_string())
        .replace(URL_PLACEHOLDER, &address.url)
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    fn address() -> OpenAddress {
        OpenAddress {
            host: "[::1]".to_owned(),
            port: 5999,
            url: "http://[::1]:5999/".to_owned(),
        }
    }

    fn opener(args: &[&str]) -> Opener {
        Opener {
            id: "viewer".to_owned(),
            name: "Viewer".to_owned(),
            command: "viewer".to_owned(),
            args: args.iter().map(|arg| (*arg).to_owned()).collect(),
            ports: Vec::new(),
        }
    }

    #[test]
    fn should_plan_the_browser_url_without_an_opener() {
        assert_eq!(
            plan_open(None, &address()),
            OpenPlan::Browser {
                url: "http://[::1]:5999/".to_owned()
            }
        );
    }

    #[test]
    fn should_replace_the_host_port_and_url_placeholders() {
        let plan = plan_open(
            Some(&opener(&["-c", "vnc://{host}:{port}", "--url={url}"])),
            &address(),
        );

        assert_eq!(
            plan,
            OpenPlan::Program {
                name: "Viewer".to_owned(),
                command: LaunchCommand {
                    program: "viewer".to_owned(),
                    args: vec![
                        "-c".to_owned(),
                        "vnc://[::1]:5999".to_owned(),
                        "--url=http://[::1]:5999/".to_owned()
                    ]
                }
            }
        );
    }

    #[test]
    fn should_keep_one_argument_per_entry_even_when_the_value_has_spaces() {
        let plan = plan_open(Some(&opener(&["{url} more"])), &address());

        let OpenPlan::Program { command, .. } = plan else {
            panic!("expected a program");
        };
        assert_eq!(command.args.len(), 1);
    }

    #[test]
    fn should_quote_arguments_with_spaces_when_displaying() {
        let command = LaunchCommand {
            program: "viewer".to_owned(),
            args: vec!["a b".to_owned(), "c".to_owned()],
        };

        assert_eq!(command.display(), "viewer \"a b\" c");
    }
}
