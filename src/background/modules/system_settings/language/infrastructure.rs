use std::sync::Once;

use macui_core::{
    handlers::MacUiEvent,
    system_state::{ImeState, SystemLanguage},
};

use crate::{app::emit_to_webviews, error::Result};

use super::application::{LanguageEvent, LanguageManager};

fn get_language_manager() -> &'static LanguageManager {
    static TAURI_EVENT_REGISTRATION: Once = Once::new();
    TAURI_EVENT_REGISTRATION.call_once(|| {
        LanguageManager::subscribe(|event| match event {
            LanguageEvent::LayoutChanged => {
                emit_to_webviews(
                    MacUiEvent::SystemLanguagesChanged,
                    &LanguageManager::instance().get_languages(),
                );
            }
            LanguageEvent::ImeChanged => {
                if let Ok(state) = LanguageManager::get_ime_state() {
                    emit_to_webviews(MacUiEvent::SystemImeStateChanged, &state);
                }
            }
        });
    });
    LanguageManager::instance()
}

#[tauri::command(async)]
pub fn get_system_languages() -> Vec<SystemLanguage> {
    get_language_manager().get_languages()
}

#[tauri::command(async)]
pub fn get_ime_state() -> Result<ImeState> {
    get_language_manager();
    LanguageManager::get_ime_state()
}

#[tauri::command(async)]
pub fn set_system_keyboard_layout(id: String, handle: String) -> Result<()> {
    get_language_manager();
    LanguageManager::set_keyboard_layout(&id, &handle)?;
    Ok(())
}
