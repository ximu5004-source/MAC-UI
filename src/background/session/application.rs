use crate::{error::Result, event_manager};
use macui_core::system_state::MacUiSession;
use parking_lot::Mutex;
use std::sync::LazyLock;

#[derive(Debug, Clone)]
pub enum SessionManagerEvent {
    Changed(Option<MacUiSession>),
}
event_manager!(SessionManager, SessionManagerEvent);

/// Compatibility for installed widgets. MAC UI has no remote account, token store,
/// refresh timer or online permission service. Windows user accounts are separate.
pub struct SessionManager {
    pub session: Option<MacUiSession>,
}

impl SessionManager {
    pub fn instance() -> &'static Mutex<Self> {
        static INSTANCE: LazyLock<Mutex<SessionManager>> =
            LazyLock::new(|| Mutex::new(SessionManager { session: None }));
        &INSTANCE
    }

    pub fn has_premium_access(&self) -> bool {
        false
    }

    pub fn login() -> Result<()> {
        Err("Online accounts are not available in MAC UI.".into())
    }

    pub async fn logout() -> Result<()> {
        Self::instance().lock().session = None;
        Self::send(SessionManagerEvent::Changed(None));
        Ok(())
    }
}
