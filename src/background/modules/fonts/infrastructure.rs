use std::sync::Once;

use macui_core::{handlers::MacUiEvent, system_state::MacUiFont};

use crate::{app::emit_to_webviews, error::Result};

use super::application::{FontManager, FontManagerEvent};

fn get_font_manager() -> &'static FontManager {
    static REGISTER: Once = Once::new();
    REGISTER.call_once(|| {
        FontManager::subscribe(|event| {
            if event == FontManagerEvent::FontsChanged
                && let Ok(fonts) = FontManager::instance().get_fonts()
            {
                emit_to_webviews(MacUiEvent::SystemFontsChanged, &fonts);
            }
        });
    });
    FontManager::instance()
}

#[tauri::command(async)]
pub fn get_fonts() -> Result<Vec<MacUiFont>> {
    get_font_manager().get_fonts()
}
