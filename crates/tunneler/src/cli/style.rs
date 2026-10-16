//! How the output looks: color and unicode symbols, decided once from the
//! terminal and the environment.

use std::io::IsTerminal;

use clap::ValueEnum;

const NO_COLOR_VARIABLE: &str = "NO_COLOR";
const TERM_VARIABLE: &str = "TERM";
const DUMB_TERMINAL: &str = "dumb";
const LOCALE_VARIABLES: [&str; 3] = ["LC_ALL", "LC_CTYPE", "LANG"];
const UTF8_MARKERS: [&str; 2] = ["utf-8", "utf8"];

/// What `--color` asks for.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum ColorChoice {
    /// Color when stdout is a terminal that supports it.
    #[default]
    Auto,
    /// Always color.
    Always,
    /// Never color.
    Never,
}

/// The facts the style is decided from; gathered once, so the decision itself
/// is pure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StyleEnvironment<'a> {
    /// The value of `--color`.
    pub color_choice: ColorChoice,
    /// Whether stdout is a terminal.
    pub stdout_is_terminal: bool,
    /// The value of `NO_COLOR`, when set.
    pub no_color: Option<&'a str>,
    /// The value of `TERM`, when set.
    pub term: Option<&'a str>,
    /// The first of `LC_ALL`, `LC_CTYPE` and `LANG` that is set and not empty.
    pub locale: Option<&'a str>,
    /// Whether the program runs on Windows.
    pub is_windows: bool,
}

/// Whether to color and whether to use unicode symbols.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Style {
    /// Colors the state words; never the only carrier of information.
    pub color: bool,
    /// Uses `✔` and `→` rather than `[ok]` and `->`.
    pub unicode: bool,
}

/// A color a word can be painted in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// Things are fine.
    Good,
    /// Something needs attention.
    Warning,
    /// Something is wrong.
    Bad,
    /// Something is idle.
    Muted,
}

impl Tone {
    const fn escape_code(self) -> &'static str {
        match self {
            Self::Good => "32",
            Self::Warning => "33",
            Self::Bad => "31",
            Self::Muted => "2",
        }
    }
}

impl Style {
    /// Decides the style from the environment.
    ///
    /// Color needs a terminal, no `NO_COLOR` (unless empty), and a `TERM` that
    /// is not `dumb`; `--color always` and `never` override all of that.
    /// Unicode is used on Windows and when the locale names UTF-8.
    pub fn detect(environment: &StyleEnvironment<'_>) -> Self {
        Self {
            color: should_color(environment),
            unicode: environment.is_windows || is_utf8_locale(environment.locale),
        }
    }

    /// The style for this process: reads the environment and asks whether
    /// stdout is a terminal.
    pub fn for_process(color_choice: ColorChoice) -> Self {
        let no_color = std::env::var(NO_COLOR_VARIABLE).ok();
        let term = std::env::var(TERM_VARIABLE).ok();
        let locale = LOCALE_VARIABLES
            .iter()
            .filter_map(|name| std::env::var(name).ok())
            .find(|value| !value.is_empty());
        Self::detect(&StyleEnvironment {
            color_choice,
            stdout_is_terminal: std::io::stdout().is_terminal(),
            no_color: no_color.as_deref(),
            term: term.as_deref(),
            locale: locale.as_deref(),
            is_windows: cfg!(windows),
        })
    }

    /// Wraps `text` in the escape codes of `tone` when color is on.
    pub fn paint(self, tone: Tone, text: &str) -> String {
        if !self.color {
            return text.to_owned();
        }
        format!("\x1b[{}m{text}\x1b[0m", tone.escape_code())
    }

    /// The arrow between the listening side and the target.
    pub const fn arrow(self) -> &'static str {
        if self.unicode { "→" } else { "->" }
    }

    /// The ellipsis that ends a progress message.
    pub const fn ellipsis(self) -> &'static str {
        if self.unicode { "…" } else { "..." }
    }

    /// The separator between the parts of a status line.
    pub const fn separator(self) -> &'static str {
        if self.unicode { "·" } else { "-" }
    }
}

fn should_color(environment: &StyleEnvironment<'_>) -> bool {
    match environment.color_choice {
        ColorChoice::Always => true,
        ColorChoice::Never => false,
        ColorChoice::Auto => {
            environment.stdout_is_terminal
                && environment.no_color.is_none_or(str::is_empty)
                && environment.term != Some(DUMB_TERMINAL)
        }
    }
}

fn is_utf8_locale(locale: Option<&str>) -> bool {
    locale.is_some_and(|locale| {
        let lowercase = locale.to_lowercase();
        UTF8_MARKERS.iter().any(|marker| lowercase.contains(marker))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn terminal_environment() -> StyleEnvironment<'static> {
        StyleEnvironment {
            color_choice: ColorChoice::Auto,
            stdout_is_terminal: true,
            no_color: None,
            term: Some("xterm-256color"),
            locale: Some("en_US.UTF-8"),
            is_windows: false,
        }
    }

    fn color_of(environment: StyleEnvironment<'_>) -> bool {
        Style::detect(&environment).color
    }

    fn unicode_of(environment: StyleEnvironment<'_>) -> bool {
        Style::detect(&environment).unicode
    }

    #[test]
    fn should_color_when_stdout_is_a_capable_terminal() {
        assert!(color_of(terminal_environment()));
    }

    #[test]
    fn should_not_color_when_stdout_is_not_a_terminal() {
        assert!(!color_of(StyleEnvironment {
            stdout_is_terminal: false,
            ..terminal_environment()
        }));
    }

    #[test]
    fn should_not_color_when_no_color_is_set() {
        assert!(!color_of(StyleEnvironment {
            no_color: Some("1"),
            ..terminal_environment()
        }));
    }

    #[test]
    fn should_color_when_no_color_is_empty() {
        assert!(color_of(StyleEnvironment {
            no_color: Some(""),
            ..terminal_environment()
        }));
    }

    #[test]
    fn should_not_color_when_the_terminal_is_dumb() {
        assert!(!color_of(StyleEnvironment {
            term: Some("dumb"),
            ..terminal_environment()
        }));
    }

    #[test]
    fn should_color_when_term_is_unset() {
        assert!(color_of(StyleEnvironment {
            term: None,
            ..terminal_environment()
        }));
    }

    #[test]
    fn should_color_when_always_is_asked_even_without_a_terminal() {
        assert!(color_of(StyleEnvironment {
            color_choice: ColorChoice::Always,
            stdout_is_terminal: false,
            no_color: Some("1"),
            term: Some("dumb"),
            ..terminal_environment()
        }));
    }

    #[test]
    fn should_not_color_when_never_is_asked_even_on_a_capable_terminal() {
        assert!(!color_of(StyleEnvironment {
            color_choice: ColorChoice::Never,
            ..terminal_environment()
        }));
    }

    #[test]
    fn should_use_unicode_when_the_locale_names_utf8() {
        assert!(unicode_of(terminal_environment()));
    }

    #[test]
    fn should_use_unicode_when_the_locale_spells_utf8_without_a_dash() {
        assert!(unicode_of(StyleEnvironment {
            locale: Some("fr_FR.utf8"),
            ..terminal_environment()
        }));
    }

    #[test]
    fn should_use_ascii_when_the_locale_is_not_utf8() {
        assert!(!unicode_of(StyleEnvironment {
            locale: Some("C"),
            ..terminal_environment()
        }));
    }

    #[test]
    fn should_use_ascii_when_no_locale_is_set() {
        assert!(!unicode_of(StyleEnvironment {
            locale: None,
            ..terminal_environment()
        }));
    }

    #[test]
    fn should_use_unicode_on_windows_whatever_the_locale() {
        assert!(unicode_of(StyleEnvironment {
            locale: None,
            is_windows: true,
            ..terminal_environment()
        }));
    }

    #[test]
    fn should_leave_the_text_untouched_when_painting_without_color() {
        let style = Style {
            color: false,
            unicode: true,
        };

        assert_eq!(style.paint(Tone::Bad, "failed"), "failed");
    }

    #[test]
    fn should_wrap_the_text_in_escape_codes_when_painting_with_color() {
        let style = Style {
            color: true,
            unicode: true,
        };

        assert_eq!(style.paint(Tone::Good, "ok"), "\x1b[32mok\x1b[0m");
    }

    #[test]
    fn should_use_ascii_substitutes_when_unicode_is_off() {
        let style = Style {
            color: false,
            unicode: false,
        };

        assert_eq!(
            (style.arrow(), style.ellipsis(), style.separator()),
            ("->", "...", "-")
        );
    }
}
