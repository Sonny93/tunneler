//! The command line run on the host to list its devslot instances.

/// The program run when the host overrides nothing.
pub(crate) const DEFAULT_PROGRAM: &str = "ds";

const HOME_PREFIX: &str = "~/";
const LISTING_ARGUMENTS: &str = "ls --json";

/// The command line that lists the instances: the program, quoted for a
/// POSIX shell, then `ls --json`. A leading `~/` stays outside the quotes so
/// that the remote shell expands it.
pub(crate) fn build_listing_command(program: Option<&str>) -> String {
    let program = program.unwrap_or(DEFAULT_PROGRAM);
    format!("{} {LISTING_ARGUMENTS}", quote_program(program))
}

fn quote_program(program: &str) -> String {
    match program.strip_prefix(HOME_PREFIX) {
        Some(relative) => format!("{HOME_PREFIX}{}", single_quote(relative)),
        None => single_quote(program),
    }
}

fn single_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_run_ds_when_the_host_has_no_path() {
        assert_eq!(build_listing_command(None), "'ds' ls --json");
    }

    #[test]
    fn should_quote_a_plain_path_when_the_host_has_one() {
        assert_eq!(
            build_listing_command(Some("/opt/devslot/ds")),
            "'/opt/devslot/ds' ls --json"
        );
    }

    #[test]
    fn should_quote_spaces_when_the_path_has_some() {
        assert_eq!(
            build_listing_command(Some("/opt/my tools/ds")),
            "'/opt/my tools/ds' ls --json"
        );
    }

    #[test]
    fn should_escape_a_single_quote_when_the_path_has_one() {
        assert_eq!(
            build_listing_command(Some("/opt/o'brien/ds")),
            "'/opt/o'\\''brien/ds' ls --json"
        );
    }

    #[test]
    fn should_leave_the_home_prefix_unquoted_when_the_path_starts_with_it() {
        assert_eq!(
            build_listing_command(Some("~/.cargo/bin/ds")),
            "~/'.cargo/bin/ds' ls --json"
        );
    }

    #[test]
    fn should_quote_a_tilde_that_is_not_a_home_prefix() {
        assert_eq!(
            build_listing_command(Some("~other/ds")),
            "'~other/ds' ls --json"
        );
    }

    #[test]
    fn should_not_let_shell_syntax_through_when_the_path_holds_some() {
        assert_eq!(
            build_listing_command(Some("ds; rm -rf $HOME")),
            "'ds; rm -rf $HOME' ls --json"
        );
    }
}
