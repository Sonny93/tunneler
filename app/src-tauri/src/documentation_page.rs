//! Which page of the documentation site the front end asks to open, and where it lives.
//! The front end only names a page and a language, never a URL.

use serde::Deserialize;

const DOCUMENTATION_BASE_URL: &str = "https://sonny93.github.io/tunneler/";
const FRENCH_PATH_PREFIX: &str = "fr/";

/// A page of the documentation the app links to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentationPage {
    Home,
    Applications,
}

impl DocumentationPage {
    const fn path(self) -> &'static str {
        match self {
            Self::Home => "",
            Self::Applications => "guide/applications",
        }
    }
}

/// A language the documentation is published in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentationLocale {
    En,
    Fr,
}

impl DocumentationLocale {
    const fn path_prefix(self) -> &'static str {
        match self {
            Self::En => "",
            Self::Fr => FRENCH_PATH_PREFIX,
        }
    }
}

/// The address of `page` in the documentation of `locale`.
pub fn documentation_url(page: DocumentationPage, locale: DocumentationLocale) -> String {
    format!(
        "{DOCUMENTATION_BASE_URL}{}{}",
        locale.path_prefix(),
        page.path()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_open_the_english_home_at_the_site_root() {
        assert_eq!(
            documentation_url(DocumentationPage::Home, DocumentationLocale::En),
            "https://sonny93.github.io/tunneler/"
        );
    }

    #[test]
    fn should_open_the_french_home_under_the_fr_prefix() {
        assert_eq!(
            documentation_url(DocumentationPage::Home, DocumentationLocale::Fr),
            "https://sonny93.github.io/tunneler/fr/"
        );
    }

    #[test]
    fn should_open_the_english_applications_guide() {
        assert_eq!(
            documentation_url(DocumentationPage::Applications, DocumentationLocale::En),
            "https://sonny93.github.io/tunneler/guide/applications"
        );
    }

    #[test]
    fn should_open_the_french_applications_guide() {
        assert_eq!(
            documentation_url(DocumentationPage::Applications, DocumentationLocale::Fr),
            "https://sonny93.github.io/tunneler/fr/guide/applications"
        );
    }

    #[test]
    fn should_read_the_page_and_locale_sent_by_the_front_end() {
        let page: DocumentationPage = serde_json::from_str("\"applications\"").unwrap();
        let locale: DocumentationLocale = serde_json::from_str("\"fr\"").unwrap();

        assert_eq!(page, DocumentationPage::Applications);
        assert_eq!(locale, DocumentationLocale::Fr);
    }

    #[test]
    fn should_refuse_a_page_that_is_not_listed() {
        assert!(serde_json::from_str::<DocumentationPage>("\"https://evil.test\"").is_err());
    }
}
