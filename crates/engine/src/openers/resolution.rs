use crate::runtime::EngineError;
use crate::store::{BROWSER_OPENER_ID, Opener, TunnelDefinition};

const BROWSER_NAME: &str = "Browser";

/// Where an available opener comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenerSource {
    /// A built-in preset whose program was found on this machine.
    Detected,
    /// An opener of the user, from `[[openers]]`.
    User,
}

/// An opener that can be used now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvailableOpener {
    /// The opener itself.
    pub opener: Opener,
    /// Where it comes from.
    pub source: OpenerSource,
}

/// The action a tunnel opens with when nothing is asked: its id and the name
/// to show (the browser has the id `browser`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefaultOpener {
    /// `browser` or the id of an opener.
    pub id: String,
    /// The name of the opener; its id when it is not available anymore.
    pub name: String,
}

/// The openers to offer: the ones of the user first, then the detected
/// presets they do not replace. An opener of the user with the id of a preset
/// replaces it.
pub fn available_openers(detected: &[Opener], user: &[Opener]) -> Vec<AvailableOpener> {
    let user_openers = user.iter().map(|opener| AvailableOpener {
        opener: opener.clone(),
        source: OpenerSource::User,
    });
    let presets = detected
        .iter()
        .filter(|preset| user.iter().all(|opener| opener.id != preset.id))
        .map(|preset| AvailableOpener {
            opener: preset.clone(),
            source: OpenerSource::Detected,
        });
    user_openers.chain(presets).collect()
}

/// What opens a tunnel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OpenAction<'a> {
    Browser,
    Program(&'a AvailableOpener),
}

/// The action asked for (`requested`, else the `open.with` of the tunnel,
/// else the suggestion for its target port, else the browser).
pub(crate) fn choose_action<'a>(
    definition: &TunnelDefinition,
    requested: Option<&str>,
    openers: &'a [AvailableOpener],
) -> Result<OpenAction<'a>, EngineError> {
    let wanted = requested.or(definition.open.with.as_deref());
    let Some(id) = wanted else {
        return Ok(suggest(definition, openers));
    };
    if id == BROWSER_OPENER_ID {
        return Ok(OpenAction::Browser);
    }
    openers
        .iter()
        .find(|available| available.opener.id == id)
        .map(OpenAction::Program)
        .ok_or_else(|| EngineError::UnknownOpener {
            id: id.to_owned(),
            available: available_ids(openers),
        })
}

fn available_ids(openers: &[AvailableOpener]) -> Vec<String> {
    std::iter::once(BROWSER_OPENER_ID.to_owned())
        .chain(openers.iter().map(|available| available.opener.id.clone()))
        .collect()
}

fn suggest<'a>(definition: &TunnelDefinition, openers: &'a [AvailableOpener]) -> OpenAction<'a> {
    let target_port = definition.effective_target_port();
    openers
        .iter()
        .find(|available| available.opener.ports.contains(&target_port))
        .map_or(OpenAction::Browser, OpenAction::Program)
}

/// What the tunnel opens with when nothing is asked, for display. A saved
/// choice that is not available anymore keeps its id as its name.
pub fn default_opener(definition: &TunnelDefinition, openers: &[AvailableOpener]) -> DefaultOpener {
    let saved_id = definition.open.with.as_deref();
    match choose_action(definition, None, openers) {
        Ok(OpenAction::Browser) => DefaultOpener {
            id: BROWSER_OPENER_ID.to_owned(),
            name: BROWSER_NAME.to_owned(),
        },
        Ok(OpenAction::Program(available)) => DefaultOpener {
            id: available.opener.id.clone(),
            name: available.opener.name.clone(),
        },
        Err(_) => DefaultOpener {
            id: saved_id.unwrap_or(BROWSER_OPENER_ID).to_owned(),
            name: saved_id.unwrap_or(BROWSER_NAME).to_owned(),
        },
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::store::{OpenOptions, TunnelId, TunnelKind};

    fn opener(id: &str, ports: &[u16]) -> Opener {
        Opener {
            id: id.to_owned(),
            name: id.to_uppercase(),
            command: id.to_owned(),
            args: Vec::new(),
            ports: ports.to_vec(),
        }
    }

    fn tunnel(target_port: u16, with: Option<&str>) -> TunnelDefinition {
        TunnelDefinition {
            id: TunnelId::generate(),
            host: "nova".to_owned(),
            kind: TunnelKind::Local,
            listen_port: 12345,
            listen_address: None,
            target_host: None,
            target_port: Some(target_port),
            tags: Vec::new(),
            open: OpenOptions {
                with: with.map(str::to_owned),
                ..OpenOptions::default()
            },
            standby: None,
        }
    }

    fn ids(openers: &[AvailableOpener]) -> Vec<&str> {
        openers
            .iter()
            .map(|available| available.opener.id.as_str())
            .collect()
    }

    #[test]
    fn should_list_the_openers_of_the_user_before_the_presets() {
        let openers = available_openers(&[opener("preset", &[])], &[opener("mine", &[])]);

        assert_eq!(ids(&openers), ["mine", "preset"]);
    }

    #[test]
    fn should_replace_a_preset_by_the_opener_of_the_user_with_the_same_id() {
        let openers = available_openers(&[opener("same", &[1])], &[opener("same", &[2])]);

        assert_eq!(
            openers,
            vec![AvailableOpener {
                opener: opener("same", &[2]),
                source: OpenerSource::User
            }]
        );
    }

    #[test]
    fn should_mark_detected_presets_as_detected() {
        let openers = available_openers(&[opener("preset", &[])], &[]);

        assert_eq!(openers[0].source, OpenerSource::Detected);
    }

    #[test]
    fn should_suggest_the_opener_whose_ports_match_the_target_port() {
        let openers = available_openers(&[opener("vnc", &[5900, 5901])], &[]);

        assert_eq!(
            choose_action(&tunnel(5901, None), None, &openers).unwrap(),
            OpenAction::Program(&openers[0])
        );
    }

    #[test]
    fn should_open_the_browser_when_no_opener_matches_the_target_port() {
        let openers = available_openers(&[opener("vnc", &[5900])], &[]);

        assert_eq!(
            choose_action(&tunnel(80, None), None, &openers).unwrap(),
            OpenAction::Browser
        );
    }

    #[test]
    fn should_suggest_an_opener_of_the_user_before_a_preset_on_the_same_port() {
        let openers = available_openers(&[opener("preset", &[5432])], &[opener("mine", &[5432])]);

        assert_eq!(
            choose_action(&tunnel(5432, None), None, &openers).unwrap(),
            OpenAction::Program(&openers[0])
        );
    }

    #[test]
    fn should_match_the_target_port_and_not_the_listen_port() {
        let openers = available_openers(&[opener("rdp", &[12345])], &[]);

        assert_eq!(
            choose_action(&tunnel(3389, None), None, &openers).unwrap(),
            OpenAction::Browser
        );
    }

    #[test]
    fn should_prefer_the_saved_choice_over_the_suggestion() {
        let openers = available_openers(&[opener("vnc", &[5900])], &[opener("mine", &[])]);

        assert_eq!(
            choose_action(&tunnel(5900, Some("mine")), None, &openers).unwrap(),
            OpenAction::Program(&openers[0])
        );
    }

    #[test]
    fn should_open_the_browser_when_the_saved_choice_is_the_browser() {
        let openers = available_openers(&[opener("vnc", &[5900])], &[]);

        assert_eq!(
            choose_action(&tunnel(5900, Some("browser")), None, &openers).unwrap(),
            OpenAction::Browser
        );
    }

    #[test]
    fn should_prefer_the_requested_choice_over_the_saved_one() {
        let openers = available_openers(&[opener("vnc", &[5900])], &[]);

        assert_eq!(
            choose_action(&tunnel(5900, Some("browser")), Some("vnc"), &openers).unwrap(),
            OpenAction::Program(&openers[0])
        );
    }

    #[test]
    fn should_list_the_available_ids_when_the_opener_is_unknown() {
        let openers = available_openers(&[opener("vnc", &[])], &[]);

        let error = choose_action(&tunnel(80, Some("ghost")), None, &openers).unwrap_err();

        assert!(matches!(
            error,
            EngineError::UnknownOpener { id, available } if id == "ghost" && available == ["browser", "vnc"]
        ));
    }

    #[test]
    fn should_name_the_browser_as_the_default_when_nothing_matches() {
        assert_eq!(default_opener(&tunnel(80, None), &[]).id, "browser");
    }

    #[test]
    fn should_name_the_suggested_opener_as_the_default() {
        let openers = available_openers(&[opener("vnc", &[5900])], &[]);

        assert_eq!(
            default_opener(&tunnel(5900, None), &openers),
            DefaultOpener {
                id: "vnc".to_owned(),
                name: "VNC".to_owned()
            }
        );
    }

    #[test]
    fn should_keep_the_saved_id_as_the_default_when_the_opener_is_gone() {
        assert_eq!(
            default_opener(&tunnel(80, Some("ghost")), &[]),
            DefaultOpener {
                id: "ghost".to_owned(),
                name: "ghost".to_owned()
            }
        );
    }
}
