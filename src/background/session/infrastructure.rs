use std::sync::Once;

use macui_core::{handlers::MacUiEvent, system_state::MacUiSession};

use crate::{
    app::emit_to_webviews,
    error::{Result, ResultLogExt},
    resources::RESOURCES,
    state::application::FULL_STATE,
};

use super::application::{SessionManager, SessionManagerEvent};

fn maybe_redact_session(session: Option<MacUiSession>) -> Option<MacUiSession> {
    session.map(|mut s| {
        if FULL_STATE.load().settings.streaming_mode {
            s.email = "***@local".to_string();
        }
        s
    })
}

fn get_session_manager() -> &'static parking_lot::Mutex<SessionManager> {
    static REGISTER: Once = Once::new();
    REGISTER.call_once(|| {
        SessionManager::subscribe(|event| {
            let SessionManagerEvent::Changed(session) = event;
            emit_to_webviews(
                MacUiEvent::MacUiSessionChanged,
                maybe_redact_session(session),
            );
            RESOURCES.emit_on_session_changed().log_error();
        });
    });
    SessionManager::instance()
}

pub fn reemit_session() {
    let session = get_session_manager().lock().session.clone();
    emit_to_webviews(
        MacUiEvent::MacUiSessionChanged,
        maybe_redact_session(session),
    );
}

/// Returns the current session (user data without tokens) or `null` if not
/// authenticated. Safe to call from any widget.
#[tauri::command(async)]
pub fn get_macui_session() -> Option<MacUiSession> {
    maybe_redact_session(get_session_manager().lock().session.clone())
}

/// Compatibility command for the removed online account service.
#[tauri::command(async)]
pub fn macui_login() -> Result<()> {
    get_session_manager(); // ensure event subscription is initialised
    SessionManager::login()
}

/// Clears only the in-memory compatibility session; saved credentials are untouched.
#[tauri::command(async)]
pub async fn macui_logout() -> Result<()> {
    get_session_manager(); // ensure event subscription is initialised
    SessionManager::logout().await
}
