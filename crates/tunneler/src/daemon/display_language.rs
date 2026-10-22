//! The language the daemon uses for what it shows itself: the tray and the
//! desktop notifications.

use engine::Language;

const FRENCH_LANGUAGE_SUBTAG: &str = "fr";
const LOCALE_SUBTAG_SEPARATORS: [char; 4] = ['-', '_', '.', '@'];

/// A language the daemon can display.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum DisplayLanguage {
    /// The language of the code and the default.
    #[default]
    English,
    /// French.
    French,
}

impl DisplayLanguage {
    /// The language to display for `setting`; `Language::System` follows
    /// `system_locale`, for example `fr-FR`, `fr_FR.UTF-8` or `en-US`, and
    /// falls back to English when it is unknown or not supported.
    pub(crate) fn resolve(setting: Language, system_locale: Option<&str>) -> Self {
        match setting {
            Language::English => Self::English,
            Language::French => Self::French,
            Language::System => system_locale.map_or(Self::English, Self::from_locale),
        }
    }

    /// Like [`Self::resolve`], reading the locale of the operating system.
    pub(crate) fn current(setting: Language) -> Self {
        match setting {
            Language::System => Self::resolve(setting, sys_locale::get_locale().as_deref()),
            Language::English | Language::French => Self::resolve(setting, None),
        }
    }

    fn from_locale(locale: &str) -> Self {
        let language_subtag = locale
            .split(LOCALE_SUBTAG_SEPARATORS)
            .next()
            .unwrap_or_default();
        if language_subtag.eq_ignore_ascii_case(FRENCH_LANGUAGE_SUBTAG) {
            Self::French
        } else {
            Self::English
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_follow_a_french_locale_when_the_setting_is_system() {
        for locale in ["fr", "fr-FR", "fr_CA", "fr_FR.UTF-8", "FR-fr", "fr@euro"] {
            assert_eq!(
                DisplayLanguage::resolve(Language::System, Some(locale)),
                DisplayLanguage::French,
                "{locale}"
            );
        }
    }

    #[test]
    fn should_use_english_when_the_setting_is_system_and_the_locale_is_not_french() {
        for locale in ["en-US", "de-DE", "C", "POSIX", "", "fra-x", "frb"] {
            assert_eq!(
                DisplayLanguage::resolve(Language::System, Some(locale)),
                DisplayLanguage::English,
                "{locale}"
            );
        }
    }

    #[test]
    fn should_use_english_when_the_setting_is_system_and_the_locale_is_unknown() {
        assert_eq!(
            DisplayLanguage::resolve(Language::System, None),
            DisplayLanguage::English
        );
    }

    #[test]
    fn should_ignore_the_locale_when_the_language_is_chosen() {
        assert_eq!(
            DisplayLanguage::resolve(Language::English, Some("fr-FR")),
            DisplayLanguage::English
        );
        assert_eq!(
            DisplayLanguage::resolve(Language::French, Some("en-US")),
            DisplayLanguage::French
        );
    }
}
