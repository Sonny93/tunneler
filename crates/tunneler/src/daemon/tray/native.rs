//! The tray icon itself, built on `tray-icon`; shared by the Linux and
//! Windows tray threads.
//!
//! The icon and the menu items are `!Send`: they must be created and used on
//! the thread that owns them.

use std::fmt::Display;

use tokio::sync::mpsc::UnboundedSender;
use tray_icon::menu::MenuEvent;
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

use super::icon::render_icon;
use super::menu::{TrayAction, action_for_menu_id, build_menu};
use super::view::TrayView;

const TRAY_TITLE: &str = "Tunneler";

/// The tray icon, showing a [`TrayView`].
pub(super) struct NativeTray {
    icon: TrayIcon,
}

impl NativeTray {
    /// Creates the icon showing `view`.
    pub(super) fn build(view: &TrayView) -> Result<Self, String> {
        let icon = TrayIconBuilder::new()
            .with_title(TRAY_TITLE)
            .with_tooltip(&view.tooltip)
            .with_icon(icon_for(view)?)
            .with_menu(Box::new(
                build_menu(view).map_err(|error| error.to_string())?,
            ))
            .with_menu_on_left_click(false)
            .build()
            .map_err(|error| error.to_string())?;
        Ok(Self { icon })
    }

    /// Redraws the icon, the tooltip and the menu for `view`.
    pub(super) fn show(&self, view: &TrayView) {
        match icon_for(view) {
            Ok(image) => log_failure(self.icon.set_icon(Some(image))),
            Err(message) => tracing::warn!(%message, "cannot draw the tray icon"),
        }
        log_failure(self.icon.set_tooltip(Some(&view.tooltip)));
        match build_menu(view) {
            Ok(menu) => self.icon.set_menu(Some(Box::new(menu))),
            Err(error) => tracing::warn!(%error, "cannot build the tray menu"),
        }
    }
}

fn log_failure<Failure: Display>(outcome: Result<(), Failure>) {
    if let Err(error) = outcome {
        tracing::warn!(%error, "cannot update the tray icon");
    }
}

fn icon_for(view: &TrayView) -> Result<Icon, String> {
    let image = render_icon(view.status);
    Icon::from_rgba(image.rgba, image.width, image.height).map_err(|error| error.to_string())
}

/// Forwards the clicks on the icon and on the menu items to `actions`.
///
/// The handlers are process-wide and can be set once; later calls are
/// ignored by `tray-icon`.
pub(super) fn forward_events(actions: UnboundedSender<TrayAction>) {
    let menu_actions = actions.clone();
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        if let Some(action) = action_for_menu_id(&event.id.0) {
            let _ = menu_actions.send(action);
        }
    }));
    TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
        if is_left_click_release(&event) {
            let _ = actions.send(TrayAction::OpenUi);
        }
    }));
}

fn is_left_click_release(event: &TrayIconEvent) -> bool {
    matches!(
        event,
        TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        }
    )
}
