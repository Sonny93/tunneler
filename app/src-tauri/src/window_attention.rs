//! What a daemon event asks of the main window: bring it back, or flash its
//! taskbar entry without taking the focus.

use ipc::protocol::{EventMessage, HostKeyRequestEvent};
use tauri::{AppHandle, Manager, UserAttentionType};

const MAIN_WINDOW_LABEL: &str = "main";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowRequest {
    Nothing,
    BringToFront,
    RequestAttention,
}

fn window_request_for(event: &EventMessage) -> WindowRequest {
    match event {
        EventMessage::ShowRequested {} | EventMessage::QuitRequested {} => {
            WindowRequest::BringToFront
        }
        EventMessage::CredentialRequest(_)
        | EventMessage::HostKeyRequest(HostKeyRequestEvent::Prompt { .. }) => {
            WindowRequest::RequestAttention
        }
        _ => WindowRequest::Nothing,
    }
}

/// Applies what `event` asks of the main window; failures are logged only.
pub fn react_to_event(app: &AppHandle, event: &EventMessage) {
    let request = window_request_for(event);
    if request == WindowRequest::Nothing {
        return;
    }
    let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) else {
        tracing::warn!("the main window does not exist");
        return;
    };
    match request {
        WindowRequest::Nothing => {}
        WindowRequest::BringToFront => bring_to_front(&window),
        WindowRequest::RequestAttention => request_attention(&window),
    }
}

fn bring_to_front(window: &tauri::WebviewWindow) {
    if let Err(error) = window.unminimize() {
        tracing::warn!(%error, "cannot unminimize the main window");
    }
    if let Err(error) = window.show() {
        tracing::warn!(%error, "cannot show the main window");
    }
    if let Err(error) = window.set_focus() {
        tracing::warn!(%error, "cannot focus the main window");
    }
}

fn request_attention(window: &tauri::WebviewWindow) {
    match window.is_focused() {
        Ok(true) => {}
        Ok(false) => {
            if let Err(error) =
                window.request_user_attention(Some(UserAttentionType::Informational))
            {
                tracing::warn!(%error, "cannot request the user attention");
            }
        }
        Err(error) => tracing::warn!(%error, "cannot tell whether the main window is focused"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(json: &str) -> EventMessage {
        serde_json::from_str(json).expect("valid event json")
    }

    #[test]
    fn show_requested_brings_the_window_to_the_front() {
        let event = parse(r#"{"event":"show.requested","data":{}}"#);

        assert_eq!(window_request_for(&event), WindowRequest::BringToFront);
    }

    #[test]
    fn quit_requested_brings_the_window_to_the_front() {
        let event = parse(r#"{"event":"quit.requested","data":{}}"#);

        assert_eq!(window_request_for(&event), WindowRequest::BringToFront);
    }

    #[test]
    fn credential_request_requests_attention() {
        let event = parse(
            r#"{"event":"credential.request","data":{"promptId":"p","kind":"password","alias":"a","user":"u","host":"h"}}"#,
        );

        assert_eq!(window_request_for(&event), WindowRequest::RequestAttention);
    }

    #[test]
    fn host_key_prompt_requests_attention() {
        let event = parse(
            r#"{"event":"hostkey.request","data":{"kind":"prompt","promptId":"p","alias":"a","lookupName":"h","algorithm":"ssh-ed25519","fingerprintSha256":"SHA256:x"}}"#,
        );

        assert_eq!(window_request_for(&event), WindowRequest::RequestAttention);
    }

    #[test]
    fn host_key_alert_asks_nothing() {
        let event = parse(
            r#"{"event":"hostkey.request","data":{"kind":"alert","alias":"a","reason":{"kind":"unknownRejectedByPolicy"},"presentedFingerprint":"SHA256:x"}}"#,
        );

        assert_eq!(window_request_for(&event), WindowRequest::Nothing);
    }

    #[test]
    fn tray_changed_asks_nothing() {
        let event = parse(r#"{"event":"tray.changed","data":{"status":{"kind":"active"}}}"#);

        assert_eq!(window_request_for(&event), WindowRequest::Nothing);
    }

    #[test]
    fn config_reloaded_asks_nothing() {
        let event = parse(r#"{"event":"config.reloaded","data":{}}"#);

        assert_eq!(window_request_for(&event), WindowRequest::Nothing);
    }
}
