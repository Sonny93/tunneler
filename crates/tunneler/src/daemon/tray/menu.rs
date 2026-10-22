//! The tray menu: what its items do, and how they are identified.
//!
//! The whole menu is rebuilt from the [`TrayView`] on every change, so the
//! actions are carried by stable string identifiers, never by item handles.

#[cfg(any(target_os = "linux", windows))]
use tray_icon::menu::{CheckMenuItem, IsMenuItem, Menu, MenuItem, PredefinedMenuItem};

#[cfg(any(target_os = "linux", windows))]
use super::view::TrayView;

const TAG_ID_PREFIX: &str = "tag:";
const OPEN_ID: &str = "open";
const QUIT_ID: &str = "quit";
const STANDBY_ID: &str = "standby";
#[cfg(any(target_os = "linux", windows))]
const OPEN_LABEL: &str = "Ouvrir Tunneler";
#[cfg(any(target_os = "linux", windows))]
const QUIT_LABEL: &str = "Quitter";
#[cfg(any(target_os = "linux", windows))]
const STANDBY_LABEL: &str = "Mettre en veille les tunnels réveillés";

/// What the user asked for through the tray.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TrayAction {
    /// Start the tunnels of a tag, or stop them when all of them run.
    ToggleTag(String),
    /// Send every awake tunnel back to standby, after a confirmation when
    /// one of them has open connections.
    StandbyAll,
    /// Show the UI.
    OpenUi,
    /// Stop the daemon, after a confirmation when tunnels are active.
    Quit,
}

impl TrayAction {
    /// The identifier of the menu item that triggers this action.
    #[cfg(any(test, target_os = "linux", windows))]
    fn menu_id(&self) -> String {
        match self {
            Self::ToggleTag(tag) => format!("{TAG_ID_PREFIX}{tag}"),
            Self::StandbyAll => STANDBY_ID.to_owned(),
            Self::OpenUi => OPEN_ID.to_owned(),
            Self::Quit => QUIT_ID.to_owned(),
        }
    }
}

/// The action of the menu item with this identifier, if it is one of ours.
pub(crate) fn action_for_menu_id(menu_id: &str) -> Option<TrayAction> {
    match menu_id {
        STANDBY_ID => Some(TrayAction::StandbyAll),
        OPEN_ID => Some(TrayAction::OpenUi),
        QUIT_ID => Some(TrayAction::Quit),
        other => other
            .strip_prefix(TAG_ID_PREFIX)
            .map(|tag| TrayAction::ToggleTag(tag.to_owned())),
    }
}

/// Builds the menu for a view: the summary, the tags, the standby item, then
/// open and quit.
#[cfg(any(target_os = "linux", windows))]
pub(super) fn build_menu(view: &TrayView) -> tray_icon::menu::Result<Menu> {
    let summary = MenuItem::new(&view.summary_line, false, None);
    let tag_items: Vec<CheckMenuItem> = view.tags.iter().map(tag_item).collect();
    let standby = MenuItem::with_id(
        TrayAction::StandbyAll.menu_id(),
        STANDBY_LABEL,
        view.has_awake_tunnels,
        None,
    );
    let open = MenuItem::with_id(TrayAction::OpenUi.menu_id(), OPEN_LABEL, true, None);
    let quit = MenuItem::with_id(TrayAction::Quit.menu_id(), QUIT_LABEL, true, None);
    let separator = PredefinedMenuItem::separator();
    let mut items: Vec<&dyn IsMenuItem> = vec![&summary, &separator];
    items.extend(tag_items.iter().map(|item| item as &dyn IsMenuItem));
    if !tag_items.is_empty() {
        items.push(&separator);
    }
    items.extend([&standby as &dyn IsMenuItem, &open, &quit]);
    Menu::with_items(&items)
}

#[cfg(any(target_os = "linux", windows))]
fn tag_item(entry: &super::view::TagEntry) -> CheckMenuItem {
    CheckMenuItem::with_id(
        TrayAction::ToggleTag(entry.tag.clone()).menu_id(),
        entry.label(),
        true,
        entry.is_checked,
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_open_the_ui_when_the_open_item_is_chosen() {
        assert_eq!(action_for_menu_id("open"), Some(TrayAction::OpenUi));
    }

    #[test]
    fn should_standby_all_when_the_standby_item_is_chosen() {
        assert_eq!(action_for_menu_id("standby"), Some(TrayAction::StandbyAll));
    }

    #[test]
    fn should_quit_when_the_quit_item_is_chosen() {
        assert_eq!(action_for_menu_id("quit"), Some(TrayAction::Quit));
    }

    #[test]
    fn should_toggle_the_tag_when_a_tag_item_is_chosen() {
        assert_eq!(
            action_for_menu_id("tag:dev"),
            Some(TrayAction::ToggleTag("dev".to_owned()))
        );
    }

    #[test]
    fn should_ignore_an_unknown_identifier() {
        assert_eq!(action_for_menu_id("something-else"), None);
    }

    #[test]
    fn should_round_trip_every_action_through_its_menu_id() {
        let actions = [
            TrayAction::OpenUi,
            TrayAction::Quit,
            TrayAction::StandbyAll,
            TrayAction::ToggleTag("dev".to_owned()),
            TrayAction::ToggleTag("prod:eu west".to_owned()),
            TrayAction::ToggleTag(String::new()),
            TrayAction::ToggleTag("open".to_owned()),
        ];

        for action in actions {
            assert_eq!(action_for_menu_id(&action.menu_id()), Some(action));
        }
    }
}
