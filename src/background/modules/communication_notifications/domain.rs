use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use macui_core::system_state::{
    CommunicationApp, CommunicationNotification, CommunicationNotificationSource, SysTrayIcon,
    SysTrayIconId,
};
use regex::Regex;

const MAX_NOTIFICATIONS: usize = 100;

struct Observation {
    revision: u32,
    unread: bool,
    unread_count: Option<u32>,
    image: Option<String>,
    changes: [Option<i64>; 2],
    announced: bool,
}

impl Observation {
    fn new(icon: &SysTrayIcon) -> Self {
        Self {
            revision: icon.notification_revision,
            unread: has_unread_hint(&icon.tooltip),
            unread_count: unread_count(&icon.tooltip),
            image: icon.icon_image_hash.clone(),
            changes: [None, None],
            announced: false,
        }
    }

    fn starts_flash(&mut self, image: &Option<String>, now: i64) -> bool {
        if &self.image == image {
            return false;
        }
        let had_image = self.image.is_some();
        self.image.clone_from(image);
        if !had_image || image.is_none() {
            return false;
        }
        let within_burst = |time: i64| (0..6000).contains(&now.saturating_sub(time));
        if !self.changes[1].is_some_and(within_burst) {
            self.announced = false;
        }
        let started = !self.announced && self.changes[0].is_some_and(within_burst);
        self.changes = [self.changes[1], Some(now)];
        self.announced |= started;
        started
    }
}

fn is_hint(source: CommunicationNotificationSource) -> bool {
    matches!(
        source,
        CommunicationNotificationSource::UnreadHint | CommunicationNotificationSource::TrayActivity
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ClientIdentity {
    pub process_id: u32,
    pub executable: PathBuf,
}

pub(super) struct PopupNotification {
    pub app: CommunicationApp,
    pub client: ClientIdentity,
    pub tray_id: Option<SysTrayIconId>,
    pub icon_path: Option<PathBuf>,
    pub body: String,
}

/// Pure state machine for client-published notification metadata and accessible
/// popup text. It never reads chat databases or process memory.
#[derive(Default)]
pub(super) struct CommunicationNotificationStore {
    observed: HashMap<SysTrayIconId, Observation>,
    notifications: Vec<CommunicationNotification>,
    popup_targets: HashMap<String, ClientIdentity>,
}

impl CommunicationNotificationStore {
    pub(super) fn baseline(&mut self, icons: &[SysTrayIcon]) {
        for icon in icons
            .iter()
            .filter(|icon| communication_app(icon).is_some())
        {
            self.observed
                .insert(icon.stable_id.clone(), Observation::new(icon));
        }
    }

    pub(super) fn notifications(&self) -> Vec<CommunicationNotification> {
        self.notifications.clone()
    }

    pub(super) fn get(&self, id: &str) -> Option<CommunicationNotification> {
        self.notifications
            .iter()
            .find(|item| item.id == id)
            .cloned()
    }

    pub(super) fn dismiss(&mut self, id: &str) -> bool {
        let before = self.notifications.len();
        self.notifications.retain(|item| item.id != id);
        self.popup_targets.remove(id);
        before != self.notifications.len()
    }

    pub(super) fn clear(&mut self) -> bool {
        let changed = !self.notifications.is_empty();
        self.notifications.clear();
        self.popup_targets.clear();
        changed
    }

    pub(super) fn dismiss_app(&mut self, app: CommunicationApp) -> bool {
        let before = self.notifications.len();
        self.notifications.retain(|item| item.app != app);
        self.retain_targets();
        before != self.notifications.len()
    }

    pub(super) fn update(&mut self, icons: &[SysTrayIcon], date: i64) -> bool {
        let mut changed = false;
        self.observed
            .retain(|id, _| icons.iter().any(|icon| &icon.stable_id == id));
        let before = self.notifications.len();
        self.notifications.retain(|item| {
            !is_hint(item.source)
                || icons
                    .iter()
                    .any(|icon| Some(&icon.stable_id) == item.tray_id.as_ref())
        });
        changed |= before != self.notifications.len();

        for icon in icons {
            let Some(app) = communication_app(icon) else {
                continue;
            };
            let unread = has_unread_hint(&icon.tooltip);
            // Native balloons retain their separate per-message revision.
            let observation = self
                .observed
                .entry(icon.stable_id.clone())
                .or_insert_with(|| Observation {
                    revision: 0,
                    ..Observation::new(icon)
                });
            let count = unread_count(&icon.tooltip);
            let increased = count
                .zip(observation.unread_count)
                .is_some_and(|(now, old)| now > old);
            let new_unread = unread && (!observation.unread || increased);
            observation.unread = unread;
            observation.unread_count = count;
            let flashing = observation.starts_flash(&icon.icon_image_hash, date);

            // Unread prompts describe current metadata, not permanent chat
            // history. Remove them once the client clears its own hint.
            if !unread {
                let before = self.notifications.len();
                self.notifications.retain(|item| {
                    item.tray_id.as_ref() != Some(&icon.stable_id)
                        || item.source != CommunicationNotificationSource::UnreadHint
                });
                changed |= before != self.notifications.len();
            }

            let source = if new_unread {
                CommunicationNotificationSource::UnreadHint
            } else if flashing
                && !unread
                && matches!(app, CommunicationApp::WeChat | CommunicationApp::WeCom)
            {
                CommunicationNotificationSource::TrayActivity
            } else {
                continue;
            };

            if self.notifications.iter().any(|item| {
                item.app == app && !is_hint(item.source) && date.abs_diff(item.date) < 10_000
            }) {
                continue;
            }
            self.notifications.retain(|item| {
                item.tray_id.as_ref() != Some(&icon.stable_id) || !is_hint(item.source)
            });

            self.notifications.insert(
                0,
                CommunicationNotification {
                    id: uuid::Uuid::new_v4().to_string(),
                    app,
                    source,
                    tray_id: Some(icon.stable_id.clone()),
                    icon_path: icon.icon_path.clone(),
                    date,
                    title: String::new(),
                    body: String::new(),
                },
            );
            self.notifications.truncate(MAX_NOTIFICATIONS);
            changed = true;
        }
        self.retain_targets();
        changed
    }

    pub(super) fn update_native(&mut self, icon: &SysTrayIcon, date: i64) -> bool {
        let Some(app) = communication_app(icon) else {
            return false;
        };
        let Some(content) = icon
            .notification
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
        else {
            return false;
        };
        let observation = self
            .observed
            .entry(icon.stable_id.clone())
            .or_insert_with(|| Observation {
                revision: 0,
                ..Observation::new(icon)
            });
        if icon.notification_revision <= observation.revision {
            return false;
        }
        observation.revision = icon.notification_revision;
        let (title, body) = split_balloon(content);
        if self.notifications.iter().any(|item| {
            item.app == app
                && item.source == CommunicationNotificationSource::AccessiblePopup
                && date.abs_diff(item.date) <= 10_000
                && normalized_text(&format!("{} {}", item.title, item.body))
                    == normalized_text(&format!("{title} {body}"))
        }) {
            return false;
        }
        self.notifications
            .retain(|item| item.tray_id.as_ref() != Some(&icon.stable_id) || !is_hint(item.source));
        self.notifications.insert(
            0,
            CommunicationNotification {
                id: uuid::Uuid::new_v4().to_string(),
                app,
                source: CommunicationNotificationSource::Balloon,
                tray_id: Some(icon.stable_id.clone()),
                icon_path: icon.icon_path.clone(),
                date,
                title,
                body,
            },
        );
        self.notifications.truncate(MAX_NOTIFICATIONS);
        self.retain_targets();
        true
    }

    pub(super) fn popup_target(&self, id: &str) -> Option<ClientIdentity> {
        self.popup_targets.get(id).cloned()
    }

    pub(super) fn update_popup(&mut self, popup: PopupNotification, date: i64) -> bool {
        let body = popup.body.trim();
        if body.is_empty() {
            return false;
        }
        // A popup and NIF_INFO may expose the very same notification. Do not
        // collapse repeated real messages within the same source.
        if self.notifications.iter().any(|item| {
            item.app == popup.app
                && item.source == CommunicationNotificationSource::Balloon
                && date.abs_diff(item.date) <= 10_000
                && normalized_text(&format!("{} {}", item.title, item.body))
                    == normalized_text(body)
        }) {
            return false;
        }
        self.notifications
            .retain(|item| item.app != popup.app || !is_hint(item.source));
        let id = uuid::Uuid::new_v4().to_string();
        self.popup_targets.insert(id.clone(), popup.client);
        self.notifications.insert(
            0,
            CommunicationNotification {
                id,
                app: popup.app,
                source: CommunicationNotificationSource::AccessiblePopup,
                tray_id: popup.tray_id,
                icon_path: popup.icon_path,
                date,
                title: String::new(),
                body: body.to_owned(),
            },
        );
        self.notifications.truncate(MAX_NOTIFICATIONS);
        self.retain_targets();
        true
    }

    fn retain_targets(&mut self) {
        self.popup_targets
            .retain(|id, _| self.notifications.iter().any(|item| &item.id == id));
    }
}

fn normalized_text(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(super) fn communication_executable(path: &Path) -> Option<CommunicationApp> {
    let value = path.to_string_lossy().to_lowercase();
    match value.rsplit(['/', '\\']).next().unwrap_or_default() {
        "wechat.exe" | "weixin.exe" | "wechatstore.exe" => Some(CommunicationApp::WeChat),
        "wxwork.exe" | "wecom.exe" => Some(CommunicationApp::WeCom),
        "dingtalk.exe" => Some(CommunicationApp::DingTalk),
        "feishu.exe" | "lark.exe" => Some(CommunicationApp::Feishu),
        _ => None,
    }
}

pub(super) fn communication_app(icon: &SysTrayIcon) -> Option<CommunicationApp> {
    // A known executable wins over a fluctuating/user-controlled tooltip.
    if let Some(executable) = &icon.executable {
        return communication_executable(executable);
    }

    let name = icon.tooltip.trim().to_lowercase();
    if ["企业微信", "wxwork", "wecom"]
        .iter()
        .any(|prefix| name.starts_with(prefix))
    {
        Some(CommunicationApp::WeCom)
    } else if ["微信", "wechat", "weixin"]
        .iter()
        .any(|prefix| name.starts_with(prefix))
    {
        Some(CommunicationApp::WeChat)
    } else if ["钉钉", "dingtalk"]
        .iter()
        .any(|prefix| name.starts_with(prefix))
    {
        Some(CommunicationApp::DingTalk)
    } else if ["飞书", "feishu", "lark"]
        .iter()
        .any(|prefix| name.starts_with(prefix))
    {
        Some(CommunicationApp::Feishu)
    } else {
        None
    }
}

fn has_unread_hint(tooltip: &str) -> bool {
    static ZERO: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?i)\b(?:0|no)\s*(?:unread|new messages?)|(?:未读|新消息|新信息)\s*[:：]?\s*0(?:\D|$)|(?:^|[^0-9])0\s*(?:条|个)?\s*(?:未读|新消息|新信息)|(?:没有|暂无|无)\s*(?:未读|新消息|新信息)",
        )
        .unwrap()
    });
    static UNREAD: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)未读|新消息|新信息|\bunread\b|\bnew messages?\b").unwrap()
    });
    !ZERO.is_match(tooltip) && UNREAD.is_match(tooltip)
}

fn unread_count(tooltip: &str) -> Option<u32> {
    static COUNT: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)([0-9]+)\s*(?:条|个)?\s*(?:未读|新消息|新信息|unread\b|new messages?\b)|(?:未读(?:消息)?|新消息|新信息|unread(?: messages?)?)\s*[:：]\s*([0-9]+)").unwrap()
    });
    let captures = COUNT.captures(tooltip)?;
    captures
        .get(1)
        .or_else(|| captures.get(2))?
        .as_str()
        .parse()
        .ok()
}

fn split_balloon(content: &str) -> (String, String) {
    let content = content.replace("\r\n", "\n");
    match content.split_once('\n') {
        Some((title, body)) => (title.trim().to_owned(), body.trim().to_owned()),
        None => (String::new(), content.trim().to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn icon(uid: u32, executable: &str) -> SysTrayIcon {
        SysTrayIcon {
            executable: Some(executable.into()),
            notification: None,
            notification_revision: 0,
            stable_id: SysTrayIconId::HandleUid(100, uid),
            uid: Some(uid),
            window_handle: Some(100),
            guid: None,
            tooltip: String::new(),
            icon_handle: None,
            icon_path: None,
            icon_image_hash: None,
            callback_message: Some(1200),
            version: Some(4),
            is_visible: true,
        }
    }

    #[test]
    fn all_four_clients_are_adopted_with_exact_executable_identity() {
        assert_eq!(
            communication_app(&icon(1, r"C:\Apps\WEIXIN.EXE")),
            Some(CommunicationApp::WeChat)
        );
        assert_eq!(
            communication_app(&icon(1, r"C:\Apps\WXWork.exe")),
            Some(CommunicationApp::WeCom)
        );
        let mut other = icon(1, "Browser.exe");
        other.tooltip = "微信 新消息".into();
        assert_eq!(communication_app(&other), None);
        other.executable = None;
        other.tooltip = "企业微信 新消息".into();
        assert_eq!(communication_app(&other), Some(CommunicationApp::WeCom));
        assert_eq!(
            communication_app(&icon(2, r"C:\Apps\DingTalk.exe")),
            Some(CommunicationApp::DingTalk)
        );
        assert_eq!(
            communication_app(&icon(3, r"C:\Apps\Feishu.exe")),
            Some(CommunicationApp::Feishu)
        );
        assert_eq!(
            communication_app(&icon(4, r"C:\Apps\Lark.exe")),
            Some(CommunicationApp::Feishu)
        );
        for executable in [
            "WeChatBrowser.exe",
            "DingTalkHelper.exe",
            "FeishuUpdater.exe",
            "ScalarKeyFrameAnimation.exe",
        ] {
            assert_eq!(communication_app(&icon(5, executable)), None);
        }
    }

    #[test]
    fn baseline_and_replayed_revisions_do_not_create_notifications() {
        let mut store = CommunicationNotificationStore::default();
        let mut item = icon(1, "WeChat.exe");
        item.notification = Some("Alice\nActual client notification".into());
        item.notification_revision = 1;
        store.baseline(&[item.clone()]);
        assert!(!store.update(&[item.clone()], 100));
        assert!(store.notifications().is_empty());
        item.notification_revision += 1;
        assert!(store.update_native(&item, 200));
        let notification = store.notifications()[0].clone();
        assert_eq!(notification.title, "Alice");
        assert_eq!(notification.body, "Actual client notification");
        assert_eq!(
            notification.source,
            CommunicationNotificationSource::Balloon
        );
        assert!(store.dismiss(&notification.id));
        assert!(!store.update_native(&item, 300));
        assert!(!store.update(&[item], 300));
        assert!(store.notifications().is_empty());
    }

    #[test]
    fn a_wechat_flash_burst_creates_one_activatable_card_even_without_a_toast() {
        let mut store = CommunicationNotificationStore::default();
        let mut item = icon(1, "Weixin.exe");
        item.icon_image_hash = Some("idle".into());
        store.baseline(&[item.clone()]);
        for (i, hash) in ["blank", "idle", "blank", "idle", "blank"]
            .iter()
            .enumerate()
        {
            item.icon_image_hash = Some(hash.to_string());
            assert_eq!(store.update(&[item.clone()], 1000 + i as i64 * 500), i == 2);
        }
        let card = store.notifications().pop().unwrap();
        assert_eq!(card.source, CommunicationNotificationSource::TrayActivity);
        assert_eq!(card.tray_id, Some(item.stable_id.clone()));
        assert!(card.body.is_empty());
        store.dismiss(&card.id);
        item.icon_image_hash = Some("idle".into());
        assert!(!store.update(&[item.clone()], 4000));
        for (i, hash) in ["blank", "idle", "blank"].iter().enumerate() {
            item.icon_image_hash = Some(hash.to_string());
            assert_eq!(
                store.update(&[item.clone()], 12000 + i as i64 * 300),
                i == 2
            );
        }
    }

    #[test]
    fn wecom_flash_is_replaced_by_its_real_balloon() {
        let mut store = CommunicationNotificationStore::default();
        let mut icon = icon(3, "WXWork.exe");
        icon.icon_image_hash = Some("first".into());
        store.baseline(&[icon.clone()]);
        for (time, pixels) in [(100, "off"), (500, "on"), (900, "off")] {
            icon.icon_image_hash = Some(pixels.into());
            store.update(&[icon.clone()], time);
        }
        let first = store.notifications();
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].app, CommunicationApp::WeCom);
        assert_eq!(
            first[0].source,
            CommunicationNotificationSource::TrayActivity
        );
        icon.notification_revision += 1;
        icon.notification = Some("Project update".into());
        assert!(store.update_native(&icon, 1300));
        let cards = store.notifications();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].source, CommunicationNotificationSource::Balloon);
    }

    #[test]
    fn unread_count_increase_reports_a_new_message_even_while_already_unread() {
        for executable in ["Weixin.exe", "WXWork.exe"] {
            let mut store = CommunicationNotificationStore::default();
            let mut item = icon(1, executable);
            item.tooltip = "1 条未读消息".into();
            store.baseline(std::slice::from_ref(&item));
            item.tooltip = "未读消息：2".into();
            assert!(store.update(std::slice::from_ref(&item), 100));
            assert!(!store.update(std::slice::from_ref(&item), 200));
            item.tooltip = "1 unread message".into();
            assert!(!store.update(std::slice::from_ref(&item), 300));
            item.tooltip = "2 unread messages".into();
            assert!(store.update(std::slice::from_ref(&item), 400));
            assert_eq!(store.notifications().len(), 1);
            assert!(store.notifications()[0].body.is_empty());
        }
    }

    #[test]
    fn explicit_unread_transition_has_no_fabricated_body_and_can_resolve() {
        let mut store = CommunicationNotificationStore::default();
        let mut item = icon(1, "Weixin.exe");
        item.tooltip = "微信".into();
        store.baseline(&[item.clone()]);
        item.icon_image_hash = Some("flashing-is-not-a-message".into());
        assert!(!store.update(&[item.clone()], 100));
        item.tooltip = "微信 有新消息".into();
        assert!(store.update(&[item.clone()], 200));
        let notification = &store.notifications()[0];
        assert_eq!(
            notification.source,
            CommunicationNotificationSource::UnreadHint
        );
        assert!(notification.title.is_empty() && notification.body.is_empty());
        assert!(!store.update(&[item.clone()], 300));
        item.tooltip = "微信 未读：0".into();
        assert!(store.update(&[item], 400));
        assert!(store.notifications().is_empty());
    }

    #[test]
    fn a_real_balloon_replaces_only_the_generic_hint() {
        let mut store = CommunicationNotificationStore::default();
        let mut item = icon(1, "WXWork.exe");
        store.baseline(&[item.clone()]);
        item.tooltip = "企业微信 有新消息".into();
        store.update(&[item.clone()], 100);
        item.notification = Some("测试标题\r\n客户端发布的正文".into());
        item.notification_revision = 1;
        store.update_native(&item, 200);
        assert_eq!(store.notifications().len(), 1);
        assert_eq!(store.notifications()[0].body, "客户端发布的正文");
    }

    #[test]
    fn clear_does_not_repeat_the_same_hint_and_history_is_bounded() {
        let mut store = CommunicationNotificationStore::default();
        let mut item = icon(1, "WeChat.exe");
        store.baseline(&[item.clone()]);
        item.tooltip = "微信 未读".into();
        store.update(&[item.clone()], 0);
        assert!(store.clear());
        assert!(!store.update(&[item.clone()], 1));
        item.notification = Some("Actual notification body".into());
        for revision in 1..=110 {
            item.notification_revision = revision;
            store.update_native(&item, revision as i64);
        }
        assert_eq!(store.notifications().len(), MAX_NOTIFICATIONS);
        assert_eq!(store.notifications()[0].date, 110);
        assert_eq!(store.notifications()[0].body, "Actual notification body");
    }

    #[test]
    fn a_new_icon_with_only_an_existing_unread_hint_is_not_new_content() {
        let mut store = CommunicationNotificationStore::default();
        let mut item = icon(1, "WeChat.exe");
        item.tooltip = "微信 未读：2".into();
        assert!(!store.update(&[item], 0));
    }

    #[test]
    fn removed_client_clears_generic_prompt_without_deleting_real_history() {
        let mut store = CommunicationNotificationStore::default();
        let mut wechat = icon(1, "WeChat.exe");
        let mut wecom = icon(2, "WXWork.exe");
        store.baseline(&[wechat.clone(), wecom.clone()]);
        wechat.tooltip = "微信 有新消息".into();
        store.update(&[wechat.clone(), wecom.clone()], 100);
        wecom.notification = Some("A real notification".into());
        wecom.notification_revision = 1;
        store.update_native(&wecom, 200);
        assert!(store.update(&[], 300));
        assert_eq!(store.notifications().len(), 1);
        assert_eq!(
            store.notifications()[0].source,
            CommunicationNotificationSource::Balloon
        );
    }

    #[test]
    fn explicit_zero_unread_is_not_a_notification() {
        for tooltip in [
            "微信 未读：0",
            "微信 0 条未读消息",
            "WeChat 0 unread",
            "微信 暂无新消息",
            "WeCom no unread messages",
        ] {
            assert!(!has_unread_hint(tooltip), "{tooltip}");
        }
        assert!(has_unread_hint("微信 10 条未读消息"));
    }

    #[test]
    fn coalesced_tray_snapshots_do_not_lose_native_balloon_bursts() {
        let mut store = CommunicationNotificationStore::default();
        let mut item = icon(1, "WeChat.exe");
        store.baseline(&[item.clone()]);
        let mut latest = item.clone();
        latest.notification_revision = 3;
        latest.notification = Some("Last actual notification".into());
        for revision in 1..=3 {
            item.notification_revision = revision;
            item.notification = Some(format!("Actual notification {revision}"));
            assert!(store.update_native(&item, revision as i64));
            // The Changed callback may already see the end of the burst.
            assert!(!store.update(&[latest.clone()], revision as i64));
        }
        assert_eq!(store.notifications().len(), 3);
        assert_eq!(store.notifications()[1].body, "Actual notification 2");
    }

    fn popup(app: CommunicationApp, body: &str) -> PopupNotification {
        PopupNotification {
            app,
            client: ClientIdentity {
                process_id: 123,
                executable: "C:\\Apps\\WeChat.exe".into(),
            },
            tray_id: None,
            icon_path: None,
            body: body.to_owned(),
        }
    }

    #[test]
    fn popup_and_balloon_duplicates_are_suppressed_in_both_arrival_orders() {
        for popup_first in [true, false] {
            let mut store = CommunicationNotificationStore::default();
            let mut item = icon(1, "WeChat.exe");
            item.notification = Some("Alice\nA real message".into());
            item.notification_revision = 1;
            if popup_first {
                assert!(store.update_popup(
                    popup(CommunicationApp::WeChat, "Alice\nA real message"),
                    100
                ));
                assert!(!store.update_native(&item, 200));
            } else {
                assert!(store.update_native(&item, 100));
                assert!(!store.update_popup(
                    popup(CommunicationApp::WeChat, "Alice\nA real message"),
                    200
                ));
            }
            assert_eq!(store.notifications().len(), 1);
            assert!(!store.update_native(&item, 300));
        }
    }

    #[test]
    fn popup_without_tray_retains_activation_identity_and_dismiss_cleans_it() {
        let mut store = CommunicationNotificationStore::default();
        assert!(!store.update_popup(popup(CommunicationApp::WeChat, "  "), 0));
        assert!(store.update_popup(popup(CommunicationApp::WeChat, "Public popup text"), 100));
        let notification = store.notifications()[0].clone();
        assert_eq!(
            notification.source,
            CommunicationNotificationSource::AccessiblePopup
        );
        assert!(notification.tray_id.is_none());
        assert_eq!(
            store.popup_target(&notification.id).unwrap().process_id,
            123
        );
        assert!(store.dismiss(&notification.id));
        assert!(store.popup_target(&notification.id).is_none());
    }

    #[test]
    fn popup_content_is_not_merged_across_apps_or_distinct_message_text() {
        let mut store = CommunicationNotificationStore::default();
        assert!(store.update_popup(popup(CommunicationApp::WeChat, "Actual notification"), 100));
        let mut item = icon(1, "WXWork.exe");
        item.notification = Some("Actual notification".into());
        item.notification_revision = 1;
        assert!(store.update_native(&item, 200));
        item.executable = Some("WeChat.exe".into());
        item.notification = Some("A different notification".into());
        item.notification_revision += 1;
        assert!(store.update_native(&item, 300));
        assert_eq!(store.notifications().len(), 3);
    }

    #[test]
    fn real_popup_prevents_a_second_generic_unread_hint() {
        let mut store = CommunicationNotificationStore::default();
        let mut item = icon(1, "WeChat.exe");
        store.baseline(&[item.clone()]);
        store.update_popup(popup(CommunicationApp::WeChat, "Published content"), 100);
        item.tooltip = "微信 有新消息".into();
        assert!(!store.update(&[item], 200));
        assert_eq!(store.notifications().len(), 1);
    }
}
