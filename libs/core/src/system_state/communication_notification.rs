use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::SysTrayIconId;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(export, repr(enum = name)))]
pub enum CommunicationApp {
    WeChat,
    WeCom,
    DingTalk,
    Feishu,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(export, repr(enum = name)))]
pub enum CommunicationNotificationSource {
    /// Title/body were actually supplied by the client through NIF_INFO.
    Balloon,
    /// The client only supplied an explicit unread tooltip, not message content.
    UnreadHint,
    /// Repeated native tray image changes without published message text.
    TrayActivity,
    /// Text exposed by the client's visible notification popup through MSAA.
    AccessiblePopup,
}

/// A local, in-memory notification. Its ID is never a Windows toast ID.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(export))]
pub struct CommunicationNotification {
    pub id: String,
    pub app: CommunicationApp,
    pub source: CommunicationNotificationSource,
    pub tray_id: Option<SysTrayIconId>,
    pub icon_path: Option<PathBuf>,
    /// Unix epoch milliseconds.
    pub date: i64,
    /// Empty for an unread hint. Presentation supplies a localized label.
    pub title: String,
    /// Empty for an unread hint: never a fabricated message body.
    pub body: String,
}
