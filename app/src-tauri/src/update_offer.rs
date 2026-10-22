//! What the front end learns about a newer version, and the progress of its installation.

use serde::Serialize;

use crate::update_action::UpdateAction;

const RELEASE_PAGE_BASE_URL: &str = "https://github.com/Sonny93/tunneler/releases/tag/v";

/// A newer version found by the check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateOffer {
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    pub release_url: String,
    pub action: UpdateAction,
}

/// Where an installation stands, sent to the front end as it advances.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum UpdateProgress {
    #[serde(rename_all = "camelCase")]
    Downloading {
        downloaded_bytes: u64,
        total_bytes: Option<u64>,
    },
    Installing,
}

/// The release page of `version`.
pub fn release_page_url(version: &str) -> String {
    format!("{RELEASE_PAGE_BASE_URL}{version}")
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn should_point_to_the_tag_of_the_version() {
        assert_eq!(
            release_page_url("0.2.0"),
            "https://github.com/Sonny93/tunneler/releases/tag/v0.2.0"
        );
    }

    #[test]
    fn should_omit_the_notes_when_the_release_has_none() {
        let offer = UpdateOffer {
            version: "0.2.0".to_owned(),
            notes: None,
            release_url: release_page_url("0.2.0"),
            action: UpdateAction::ReleaseLinkOnly,
        };

        assert_eq!(
            serde_json::to_value(offer).unwrap(),
            json!({
                "version": "0.2.0",
                "releaseUrl": "https://github.com/Sonny93/tunneler/releases/tag/v0.2.0",
                "action": "releaseLinkOnly"
            })
        );
    }

    #[test]
    fn should_serialize_the_offer_in_camel_case_when_the_release_has_notes() {
        let offer = UpdateOffer {
            version: "0.2.0".to_owned(),
            notes: Some("Fixes".to_owned()),
            release_url: release_page_url("0.2.0"),
            action: UpdateAction::InstallAndRestart,
        };

        assert_eq!(
            serde_json::to_value(offer).unwrap(),
            json!({
                "version": "0.2.0",
                "notes": "Fixes",
                "releaseUrl": "https://github.com/Sonny93/tunneler/releases/tag/v0.2.0",
                "action": "installAndRestart"
            })
        );
    }

    #[test]
    fn should_serialize_the_download_progress_in_camel_case() {
        let progress = UpdateProgress::Downloading {
            downloaded_bytes: 512,
            total_bytes: Some(2048),
        };

        assert_eq!(
            serde_json::to_value(progress).unwrap(),
            json!({ "kind": "downloading", "downloadedBytes": 512, "totalBytes": 2048 })
        );
    }

    #[test]
    fn should_serialize_the_installing_step_by_its_kind() {
        assert_eq!(
            serde_json::to_value(UpdateProgress::Installing).unwrap(),
            json!({ "kind": "installing" })
        );
    }
}
