use macui_core::system_state::{
    AppNotification, NotificationsMode, Toast, ToastBindingChild, ToastText,
};
use parking_lot::Mutex as ParkingLotMutex;
use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{
        LazyLock,
        atomic::{AtomicBool, Ordering},
    },
};
use tauri::Manager;
use windows::{
    ApplicationModel::AppInfo,
    Foundation::{TypedEventHandler, Uri},
    UI::Notifications::{
        KnownNotificationBindings,
        Management::{UserNotificationListener, UserNotificationListenerAccessStatus},
        Notification, NotificationKinds, ToastNotification, ToastNotificationManager,
        ToastNotificationManagerForUser, ToastNotificationMode, UserNotification,
        UserNotificationChangedEventArgs, UserNotificationChangedKind,
    },
};
use windows_collections::IVectorView;
use winreg::{
    RegKey,
    enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE},
};

use crate::{
    app::get_app_handle,
    error::{Result, ResultLogExt},
    event_manager,
    modules::{
        apps::application::msix::get_hightest_quality_posible_for_uwp_image,
        start::application::StartMenuManager,
    },
    utils::{
        convert_file_to_src, icon_extractor::request_icon_extraction_from_umid,
        lock_free::SyncHashMap, spawn_named_thread,
    },
    windows_api::{
        Com, WindowsApi,
        event_window::IS_INTERACTIVE_SESSION,
        types::{AppUserModelId, DateTimeExt},
    },
};

static LOADED_NOTIFICATIONS: LazyLock<ParkingLotMutex<HashSet<u32>>> =
    LazyLock::new(|| ParkingLotMutex::new(HashSet::new()));
static RELEASED: AtomicBool = AtomicBool::new(true);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationEvent {
    Added(u32),
    Removed(u32),
    ModeChanged(NotificationsMode),
}

pub struct NotificationManager {
    notifications: SyncHashMap<u32, AppNotification>,
    manager: ToastNotificationManagerForUser,
    listener: UserNotificationListener,
    event_token: Option<i64>,
    mode_changed_token: Option<i64>,
}

unsafe impl Send for NotificationManager {}
unsafe impl Sync for NotificationManager {}

event_manager!(NotificationManager, NotificationEvent);

impl NotificationManager {
    fn new() -> windows::core::Result<Self> {
        Ok(Self {
            notifications: SyncHashMap::new(),
            manager: ToastNotificationManager::GetDefault()?,
            listener: UserNotificationListener::Current()?,
            event_token: None,
            mode_changed_token: None,
        })
    }

    pub fn instance() -> Result<&'static Self> {
        static MANAGER: LazyLock<windows::core::Result<NotificationManager>> =
            LazyLock::new(|| {
                let mut manager = NotificationManager::new()?;
                manager.init().log_error();
                Ok(manager)
            });
        Ok(MANAGER.as_ref().map_err(Clone::clone)?)
    }

    pub fn has_access(&self) -> Result<bool> {
        Ok(self.listener.GetAccessStatus()? == UserNotificationListenerAccessStatus::Allowed)
    }

    pub fn notifications(&self) -> Vec<AppNotification> {
        let mut notifications = self.notifications.values();
        notifications.sort_by_key(|b| std::cmp::Reverse(b.date));
        notifications
    }

    pub fn remove_notification(&self, id: u32) -> Result<()> {
        self.listener.RemoveNotification(id)?;
        Self::send(NotificationEvent::Removed(id));
        Ok(())
    }

    pub fn clear_notifications(&self) -> Result<()> {
        let ids: Vec<u32> = self.notifications.keys();
        let mut result = Ok(());
        for id in ids {
            if let Err(error) = self.remove_notification(id) {
                result = Err(error);
            }
        }
        result
    }

    fn init(&mut self) -> Result<()> {
        // The polling and WinRT paths both publish into this dispatcher. Install
        // it before optional event registrations, which unpackaged clients may
        // reject independently of notification-read access.
        let eid = Self::subscribe(|e| Self::process_event(e).log_error());
        Self::set_event_handler_priority(&eid, 1);

        // Toast access and system quiet mode have independent lifetimes.
        // A user can grant Toast access in Windows Settings after MAC UI starts.
        let access = self
            .listener
            .RequestAccessAsync()
            .and_then(|request| request.join());
        if matches!(access, Ok(UserNotificationListenerAccessStatus::Allowed)) {
            let snapshot = self
                .listener
                .GetNotificationsAsync(NotificationKinds::Toast)
                .and_then(|request| request.join());
            match snapshot {
                Ok(items) => {
                    for item in items {
                        self.load_notification(item).log_error();
                    }
                }
                Err(error) => log::warn!("Initial Toast snapshot unavailable: {error}"),
            }
        }

        // TODO: this only works on MSIX/APPX/UWP builds so idk how to make it work on win32 apps
        match self
            .listener
            .NotificationChanged(&TypedEventHandler::new(Self::on_notifications_change))
        {
            Ok(token) => self.event_token = Some(token),
            Err(error) => {
                log::warn!("Failed to register winrt notification change handler: {error}");
            }
        }

        match self
            .manager
            .NotificationModeChanged(&TypedEventHandler::new(Self::on_mode_change))
        {
            Ok(token) => self.mode_changed_token = Some(token),
            Err(error) => log::warn!(
                "Notification quiet-mode events unavailable; polling remains active: {error}"
            ),
        }

        // MyDockFinder's observed listener path reconciles Toast snapshots.
        // Keep reconciliation even when WinRT accepts an event registration:
        // unpackaged applications can miss notifications from desktop clients.
        RELEASED.store(false, Ordering::Release);
        spawn_named_thread("Notification Manager", || {
            Com::run_with_mta_context(|| {
                let mut previous_mode = None;
                while !RELEASED.load(Ordering::Acquire) {
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    if !IS_INTERACTIVE_SESSION.load(Ordering::Acquire) {
                        continue;
                    }
                    let manager = Self::instance()?;
                    if matches!(manager.has_access(), Ok(true)) {
                        Self::manual_notifications_loop().log_error();
                    }
                    if let Ok(mode) = manager.get_notifications_mode() {
                        if previous_mode.replace(mode) != Some(mode) {
                            Self::send(NotificationEvent::ModeChanged(mode));
                        }
                    }
                }
                Ok(())
            })
            .log_error();
        });
        Ok(())
    }

    fn on_mode_change(
        sender: windows_core::Ref<ToastNotificationManagerForUser>,
        _args: windows_core::Ref<windows_core::IInspectable>,
    ) -> windows_core::Result<()> {
        let Some(sender) = sender.as_ref() else {
            return Ok(());
        };

        let mode = sender.NotificationMode()?;
        let mode = match mode {
            ToastNotificationMode::Unrestricted => NotificationsMode::All,
            ToastNotificationMode::AlarmsOnly => NotificationsMode::AlarmsOnly,
            ToastNotificationMode::PriorityOnly => NotificationsMode::PriorityOnly,
            _ => NotificationsMode::All,
        };
        Self::send(NotificationEvent::ModeChanged(mode));
        Ok(())
    }

    fn on_notifications_change(
        listener: windows_core::Ref<UserNotificationListener>,
        args: windows_core::Ref<UserNotificationChangedEventArgs>,
    ) -> windows_core::Result<()> {
        let (Some(_listener), Some(args)) = (listener.as_ref(), args.as_ref()) else {
            return Ok(());
        };

        let change_kind = args.ChangeKind()?;
        let changed_id = args.UserNotificationId()?;

        match change_kind {
            UserNotificationChangedKind::Added => {
                Self::send(NotificationEvent::Added(changed_id));
            }
            UserNotificationChangedKind::Removed => {
                Self::send(NotificationEvent::Removed(changed_id));
            }
            _ => {}
        }
        Ok(())
    }

    fn manual_notifications_loop() -> Result<()> {
        let listener = { UserNotificationListener::Current()? };
        let mut old_toasts = LOADED_NOTIFICATIONS.lock().clone();

        for u_notification in listener
            .GetNotificationsAsync(NotificationKinds::Toast)?
            .join()?
        {
            let id: u32 = u_notification.Id()?;
            if !old_toasts.contains(&id) {
                Self::send(NotificationEvent::Added(id))
            }
            old_toasts.remove(&id);
        }

        for id in old_toasts {
            Self::send(NotificationEvent::Removed(id))
        }
        Ok(())
    }

    fn process_event(event: NotificationEvent) -> Result<()> {
        let manager = NotificationManager::instance()?;
        match event {
            NotificationEvent::Added(id) => {
                let u_notification = UserNotificationListener::Current()?.GetNotification(id)?;
                manager.load_notification(u_notification)?;
            }
            NotificationEvent::Removed(id) => {
                manager.notifications.remove(&id);
                LOADED_NOTIFICATIONS.lock().remove(&id);
            }
            NotificationEvent::ModeChanged(_) => {}
        }
        Ok(())
    }

    fn get_notification_content(
        notification: &Notification,
        toast_notifications: Option<IVectorView<ToastNotification>>,
        umid: &str,
    ) -> Result<Toast> {
        let visuals = notification.Visual()?;
        let binding = visuals.GetBinding(&KnownNotificationBindings::ToastGeneric()?)?;
        let text_sequence = binding.GetTextElements()?;

        let mut notification_text = String::new();
        for text in &text_sequence {
            let text = text.Text()?.to_string_lossy().trim().replace("\r\n", "\n");
            notification_text.push_str(&text);
        }

        for toast_notification in toast_notifications.into_iter().flatten() {
            // this can be null when the notification count is bigger than the max allowed by default 20
            if let Ok(content) = toast_notification.Content() {
                let Ok(toast_xml) = content.GetXml() else {
                    continue;
                };
                let Ok(mut toast) = quick_xml::de::from_str::<Toast>(&toast_xml.to_string()) else {
                    // One client's unsupported XML must not drop the actual
                    // plain-text notification exposed by the listener.
                    continue;
                };
                let toast_text = get_text_from_toast_for_comparison(&toast);

                // log::debug!("comparing: \n - {notification_text:?} \n - {toast_text:?} \n {toast:?}");

                if notification_text == toast_text {
                    Self::clean_toast(&mut toast, umid)?;
                    return Ok(toast);
                }
            }
        }

        log::debug!("Toast content not found, generating one from plain text");
        let mut text_entries = Vec::new();
        for text in text_sequence {
            let text = text
                .Text()?
                .to_string_lossy()
                .replace("\r\n", "\n")
                .trim()
                .to_owned();
            text_entries.push(text);
        }
        let mut toast = toast_from_plain_text(text_entries);
        Self::clean_toast(&mut toast, umid)?;
        Ok(toast)
    }

    fn clean_toast(toast: &mut Toast, umid: &str) -> Result<()> {
        let package_path = AppInfo::GetFromAppUserModelId(&umid.into())
            .and_then(|info| info.Package())
            .and_then(|package| package.InstalledPath())
            .map(|path| PathBuf::from(path.to_os_string()));

        for entry in &mut toast.visual.binding.children {
            let ToastBindingChild::Image(image) = entry else {
                continue;
            };

            if image.src.is_empty() {
                continue;
            }

            log::trace!("Resolving image src:e \"{}\"", image.src);
            let uri = Uri::CreateUri(&image.src.clone().into())?;
            let scheme = uri.SchemeName()?.to_string_lossy();
            let uri_path = PathBuf::from(
                Uri::UnescapeComponent(&uri.Path()?)?
                    .to_string_lossy()
                    .trim_start_matches('/'),
            );

            // https://learn.microsoft.com/en-us/windows/uwp/app-resources/uri-schemes
            // https://learn.microsoft.com/en-us/uwp/schemas/tiles/toastschema/element-image
            match scheme.as_str() {
                "http" | "https" => {}
                "ms-appx" | "ms-appx-web" => {
                    let path = package_path.clone()?.join(uri_path);
                    if let Some((path, _)) = get_hightest_quality_posible_for_uwp_image(&path) {
                        log::debug!("  Resolved path: {}", path.display());
                        image.src = convert_file_to_src(&path);
                    } else {
                        log::warn!("  Unable to resolve path {}", path.display());
                    }
                }
                "ms-appdata" => {
                    let parent = if uri_path.starts_with("local") {
                        "LocalState"
                    } else if uri_path.starts_with("roaming") {
                        "LocalCache"
                    } else {
                        continue;
                    };

                    let uri_path = PathBuf::from(
                        Uri::UnescapeComponent(&uri.Path()?)?
                            .to_string_lossy()
                            .to_lowercase()
                            .trim_start_matches('/')
                            .trim_start_matches("local/")
                            .trim_start_matches("roaming/"),
                    );

                    let package_family_name = AppInfo::GetFromAppUserModelId(&umid.into())?
                        .PackageFamilyName()?
                        .to_string_lossy();

                    let path = get_app_handle()
                        .path()
                        .local_data_dir()?
                        .join("Packages")
                        .join(package_family_name)
                        .join(parent)
                        .join(uri_path);

                    log::debug!("  Resolved path: {}", path.display());
                    image.src = convert_file_to_src(&path);
                }
                "file" => {
                    image.src = if uri_path.exists() {
                        convert_file_to_src(&uri_path)
                    } else {
                        // telegram desktop from ms store uses file intead of ms-appdata
                        // so this causes the image to be missing, windows doesn't show image as well
                        // so we decide to follow same behavior.
                        "".to_string()
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    // this function an in general all the notification system can still be improved on usability and performance
    fn load_notification(&self, u_notification: UserNotification) -> Result<()> {
        let notification_id = u_notification.Id()?;
        let notification = u_notification.Notification()?;

        let app_info = match u_notification.AppInfo() {
            Ok(info) => info,
            Err(_) => {
                // will fail if the notification was added by an uninstalled app
                // log::error!("Unable to get app info: {}", error);
                LOADED_NOTIFICATIONS.lock().insert(notification_id);
                return Ok(());
            }
        };

        let display_info = app_info.DisplayInfo()?;
        let app_umid = app_info.AppUserModelId()?;

        // Some traditional desktop clients publish valid listener text but
        // do not expose full toast history under their AUMID. Keep the text
        // path working instead of silently losing their notification.
        let toast_notifications = self
            .manager
            .History()
            .and_then(|history| history.GetHistoryWithId(&app_umid))
            .ok();

        log::trace!(
            "Loading notification, ID: {}, AppID: {}",
            u_notification.Id()?,
            app_umid
        );

        let notification_content = Self::get_notification_content(
            &notification,
            toast_notifications,
            &app_umid.to_string(),
        )?;

        // pre-extraction to avoid flickering on the ui
        request_icon_extraction_from_umid(&app_umid.to_string().into());

        self.notifications.upsert(
            notification_id,
            AppNotification {
                id: notification_id,
                app_umid: app_umid.to_string(),
                app_name: display_info.DisplayName()?.to_string(),
                app_description: display_info.Description()?.to_string(),
                date: u_notification.CreationTime()?.to_unix_ms(),
                content: notification_content,
            },
        );
        // A transient history/text failure must be retried by reconciliation,
        // rather than permanently marking an unreadable notification as loaded.
        LOADED_NOTIFICATIONS.lock().insert(notification_id);
        Ok(())
    }

    pub fn get_notifications_mode(&self) -> Result<NotificationsMode> {
        let mode = self.manager.NotificationMode()?;
        let mode = match mode {
            ToastNotificationMode::Unrestricted => NotificationsMode::All,
            ToastNotificationMode::AlarmsOnly => NotificationsMode::AlarmsOnly,
            ToastNotificationMode::PriorityOnly => NotificationsMode::PriorityOnly,
            _ => NotificationsMode::All,
        };
        Ok(mode)
    }

    pub fn set_mode(_mode: NotificationsMode) -> Result<()> {
        // just there's no way to change this with official apis
        std::process::Command::new("explorer.exe")
            .arg("ms-settings:notifications")
            .spawn()?;
        Ok(())
    }
}

impl Drop for NotificationManager {
    fn drop(&mut self) {
        if let Some(token) = self.event_token.take() {
            self.listener.RemoveNotificationChanged(token).log_error();
        }
        if let Some(token) = self.mode_changed_token.take() {
            self.manager
                .RemoveNotificationModeChanged(token)
                .log_error();
        }
        RELEASED.store(true, Ordering::Release);
    }
}

fn toast_from_plain_text(text_entries: Vec<String>) -> Toast {
    let mut toast = Toast::default();
    toast.visual.binding.children = text_entries
        .into_iter()
        .map(|content| ToastBindingChild::Text(ToastText { id: None, content }))
        .collect();
    toast
}

fn get_text_from_toast_for_comparison(toast: &Toast) -> String {
    let mut text = String::new();
    // only the first two top-level text entries are exposed by UserNotification::Visual,
    // so only those are relevant for comparison
    for entry in toast
        .visual
        .binding
        .children
        .iter()
        .filter_map(|entry| match entry {
            ToastBindingChild::Text(entry) => Some(entry),
            _ => None,
        })
        .take(2)
    {
        text.push_str(entry.content.replace("\r\n", "\n").trim());
    }
    text
}

#[cfg(test)]
mod notification_fallback_tests {
    use super::*;

    #[test]
    fn unavailable_rich_history_keeps_actual_plain_text_in_order() {
        let toast = toast_from_plain_text(vec!["实际通知标题".into(), "客户端发布的正文".into()]);
        let text: Vec<&str> = toast
            .visual
            .binding
            .children
            .iter()
            .filter_map(|entry| match entry {
                ToastBindingChild::Text(item) => Some(item.content.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(text, vec!["实际通知标题", "客户端发布的正文"]);
        assert!(toast.actions.is_none());
        assert!(toast.launch.is_none());
    }
}

pub fn get_toast_activator_clsid(app_umid: &AppUserModelId) -> Result<String> {
    match app_umid {
        AppUserModelId::PropertyStore(umid) => {
            // Win32 apps using a dedicated toast AUMID (Firefox's `FirefoxToast-*`,
            // FanControl, DSX...) have no start menu shortcut carrying that AUMID, so
            // the shortcut lookup below can't find them. Their CLSID lives in the
            // registry instead.
            if let Some(clsid) = get_registered_custom_activator(umid) {
                return Ok(clsid);
            }

            let guard = StartMenuManager::instance();
            if let Some(item) = guard.get_by_file_umid(umid) {
                let clsid = WindowsApi::get_file_toast_activator(&item.path)?;
                return Ok(clsid);
            }
        }
        AppUserModelId::Appx(umid) => {
            return get_appx_toast_activator_clsid(umid);
        }
    };
    Err(format!("Unable to get toast activator clsid for: {app_umid:?}").into())
}

/// Reads `CustomActivator` from `SOFTWARE\Classes\AppUserModelId\<aumid>`, where
/// Win32 apps register the CLSID of their `INotificationActivationCallback` when they
/// use a dedicated toast AUMID. HKCU takes precedence over HKLM, matching how the
/// shell resolves `Classes`.
fn get_registered_custom_activator(umid: &str) -> Option<String> {
    let roots = [
        (HKEY_CURRENT_USER, r"SOFTWARE\Classes\AppUserModelId"),
        (HKEY_LOCAL_MACHINE, r"SOFTWARE\Classes\AppUserModelId"),
    ];

    for (root, subkey) in roots {
        let Ok(key) = RegKey::predef(root).open_subkey(subkey) else {
            continue;
        };
        let Ok(key) = key.open_subkey(umid) else {
            continue;
        };
        let Ok(clsid) = key.get_value::<String, _>("CustomActivator") else {
            continue;
        };
        let clsid = clsid.trim().trim_start_matches('{').trim_end_matches('}');
        if !clsid.is_empty() {
            return Some(clsid.to_owned());
        }
    }
    None
}

/// Reads the package's `AppxManifest.xml` (via the shared `PackageManifest`
/// deserializer) and returns the `ToastActivatorCLSID` declared by the
/// `windows.toastNotificationActivation` extension for the given AppId.
/// This is what enables MSIX/APPX apps (WhatsApp, Telegram, Teams, etc.)
/// to handle toast action activation.
fn get_appx_toast_activator_clsid(app_umid: &str) -> Result<String> {
    let (_family, app_id) = app_umid
        .split_once('!')
        .ok_or("Invalid Appx AUMID (missing '!')")?;

    let app_info = AppInfo::GetFromAppUserModelId(&app_umid.into())?;
    let package = app_info.Package()?;
    let manifest =
        crate::modules::apps::application::msix_manifest::PackageManifest::try_read_for(&package)?;

    let app = manifest
        .get_app(app_id)
        .ok_or_else(|| format!("AppId {app_id} not found in manifest"))?;

    let extensions = app
        .extensions
        .as_ref()
        .ok_or("Application has no <Extensions> block")?;

    for extension in &extensions.extension {
        if extension.category != "windows.toastNotificationActivation" {
            continue;
        }
        if let Some(activation) = &extension.toast_notification_activation {
            let clsid = activation
                .toast_activator_clsid
                .trim()
                .trim_start_matches('{')
                .trim_end_matches('}')
                .to_string();
            return Ok(clsid);
        }
    }

    Err(format!("ToastActivatorCLSID not found in manifest for {app_umid}").into())
}
