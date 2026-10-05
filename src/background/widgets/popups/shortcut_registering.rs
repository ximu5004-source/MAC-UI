use std::sync::LazyLock;

use parking_lot::Mutex;
use seelen_core::state::{CssStyles, Dialog, DialogContent};
use slu_ipc::{ServiceIpc, messages::SvcAction};
use uuid::Uuid;

use tauri::{Emitter, EventId, Listener, Manager};

use crate::{
    app::get_app_handle,
    error::{Result, ResultLogExt},
    widgets::{trigger_dialog_backend, webview::WidgetWebviewLabel},
};

#[path = "shortcut_request.rs"]
mod request_metadata;
use request_metadata::RequestMetadata;

static REG_SHORTCUT_DATA: LazyLock<Mutex<RegShortcutData>> =
    LazyLock::new(|| Mutex::new(RegShortcutData::default()));

// Serialize service Start/Stop, including callbacks queued by old dialogs. Only
// this async gate may cross an await; never retain the DATA mutex while doing IPC.
static REG_SHORTCUT_LIFECYCLE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[derive(Default)]
struct RegShortcutData {
    metadata: RequestMetadata<Uuid>,
    shortcut: Option<Vec<String>>,
    response_view_label: Option<String>,
    response_event: Option<String>,
    listeners: Vec<EventId>,
}

impl RegShortcutData {
    /// Finish a detached snapshot so emitting, hiding and removing EVENT
    /// listeners cannot retain the DATA mutex or touch a replacement request.
    fn complete(self, shortcut: Option<Vec<String>>) {
        let app = get_app_handle();
        for listener in self.listeners {
            app.unlisten(listener);
        }
        if let Some(dialog_id) = self.metadata.dialog_id() {
            hide_dialog(dialog_id);
        }
        if let (Some(label), Some(event)) = (&self.response_view_label, &self.response_event) {
            app.emit_to(label, event, &shortcut).log_error();
        }
    }
}

fn hide_dialog(dialog_id: Uuid) {
    let label = WidgetWebviewLabel::new(&"@seelen/dialog".into(), None, Some(&dialog_id));
    if let Some(window) = get_app_handle().get_webview_window(&label.raw) {
        window.hide().log_error();
    }
}

fn take_registration(request_id: Option<Uuid>, dialog_id: Option<Uuid>) -> Option<RegShortcutData> {
    let mut reg = REG_SHORTCUT_DATA.lock();
    let active_request_id = reg.metadata.request_id()?;
    if let Some(request_id) = request_id {
        if !reg.metadata.matches_request(request_id) {
            return None;
        }
    }
    if let Some(dialog_id) = dialog_id {
        if !reg.metadata.matches_dialog(active_request_id, dialog_id) {
            return None;
        }
    }
    Some(std::mem::take(&mut *reg))
}

pub async fn request_shortcut_registration(label: String, event: String) -> Result<()> {
    let _lifecycle = REG_SHORTCUT_LIFECYCLE.lock().await;
    if let Some(previous) = take_registration(None, None) {
        previous.complete(None);
        ServiceIpc::send(SvcAction::StopShortcutRegistration).await?;
    }

    let request_id = Uuid::new_v4();
    {
        let mut reg = REG_SHORTCUT_DATA.lock();
        *reg = RegShortcutData {
            metadata: RequestMetadata::new(request_id),
            response_view_label: Some(label),
            response_event: Some(event),
            ..Default::default()
        };
    }

    // Start sends an initial empty shortcut before it returns. Publish the
    // identity AND response target first, otherwise that reply opens an orphan.
    if let Err(error) = ServiceIpc::send(SvcAction::StartShortcutRegistration { request_id }).await
    {
        if let Some(failed) = take_registration(Some(request_id), None) {
            failed.complete(None);
            ServiceIpc::send(SvcAction::StopShortcutRegistration)
                .await
                .log_error();
        }
        return Err(error.into());
    }
    Ok(())
}

fn queue_finish(request_id: Uuid, dialog_id: Uuid, accepted: bool) {
    tauri::async_runtime::spawn(async move {
        let _lifecycle = REG_SHORTCUT_LIFECYCLE.lock().await;
        // Recheck only after acquiring the gate. A queued old button must never
        // send an unscoped Stop after a new request has started.
        let Some(reg) = take_registration(Some(request_id), Some(dialog_id)) else {
            return;
        };
        let shortcut = if accepted {
            reg.shortcut.clone().filter(|keys| !keys.is_empty())
        } else {
            None
        };
        reg.complete(shortcut);
        ServiceIpc::send(SvcAction::StopShortcutRegistration)
            .await
            .log_error();
    });
}

pub fn set_registering_shortcut(request_id: Uuid, shortcut: Option<Vec<String>>) -> Result<()> {
    let Some(mut shortcut) = shortcut else {
        if let Some(reg) = take_registration(Some(request_id), None) {
            // Service cancellation (for example Escape) is terminal too.
            reg.complete(None);
        }
        return Ok(());
    };

    // the library allows differences between keys, but we simplify things for users
    for key in &mut shortcut {
        if key == "LShift" || key == "RShift" {
            *key = "Shift".to_string();
        }
        if key == "LControl" || key == "RControl" {
            *key = "Ctrl".to_string();
        }
        if key == "LMenu" || key == "RMenu" {
            *key = "Alt".to_string();
        }
        if key == "LWin" || key == "RWin" {
            *key = "Win".to_string();
        }
    }

    let dialog_id = {
        let mut reg = REG_SHORTCUT_DATA.lock();
        if !reg.metadata.matches_request(request_id) {
            return Ok(());
        }
        reg.shortcut = Some(shortcut.clone());
        if let Some(dialog_id) = reg.metadata.dialog_id() {
            dialog_id
        } else {
            let dialog_id = Uuid::new_v4();
            reg.metadata.bind_dialog(request_id, dialog_id);
            // Unique events and explicit cleanup prevent the opposite button's
            // listener from surviving an accept/cancel into the next session.
            let accepted = get_app_handle().listen(accepted_event(dialog_id), move |_| {
                queue_finish(request_id, dialog_id, true);
            });
            let cancelled = get_app_handle().listen(cancelled_event(dialog_id), move |_| {
                queue_finish(request_id, dialog_id, false);
            });
            reg.listeners = vec![accepted, cancelled];
            dialog_id
        }
    };

    // Dialog creation touches widget state; do it without retaining DATA.
    if let Err(error) = trigger_dialog_backend(get_dialog(dialog_id, &shortcut)) {
        queue_finish(request_id, dialog_id, false);
        return Err(error);
    }
    // A button/service cancellation can win while triggering without DATA.
    // Hide only the stale dialog; never send a late Stop to the new capture.
    let still_active = REG_SHORTCUT_DATA
        .lock()
        .metadata
        .matches_dialog(request_id, dialog_id);
    if !still_active {
        hide_dialog(dialog_id);
    }
    Ok(())
}

fn accepted_event(dialog_id: Uuid) -> String {
    format!("user_shortcut_accepted:{dialog_id}")
}

fn cancelled_event(dialog_id: Uuid) -> String {
    format!("shortcut_register_cancelled:{dialog_id}")
}

fn get_dialog(id: Uuid, shortcut: &[String]) -> Dialog {
    let items = if shortcut.is_empty() {
        vec![DialogContent::Text {
            value: t!("shortcut.register.placeholder").to_string(),
            styles: Some(
                CssStyles::new()
                    .add("color", "var(--color-gray-400)")
                    .add("fontSize", "1rem")
                    .add("fontStyle", "italic"),
            ),
        }]
    } else {
        shortcut
            .iter()
            .map(|s| DialogContent::Text {
                value: s.to_string(),
                styles: Some(
                    CssStyles::new()
                        .add("backgroundColor", "var(--slu-std-bg-light-color)")
                        .add("padding", "4px 10px")
                        .add("borderRadius", "4px"),
                ),
            })
            .collect()
    };

    let keys_box = DialogContent::Group {
        items,
        styles: Some(
            CssStyles::new()
                .add("width", "100%")
                .add("height", "100%")
                .add("display", "flex")
                .add("gap", "5px")
                .add("justifyContent", "center")
                .add("alignItems", "center")
                .add("flexWrap", "wrap")
                .add("fontWeight", "bold"),
        ),
    };

    // Cancellation is available even before the user has pressed any keys; the
    // header close button uses this same per-dialog cancellation event.
    let mut actions = vec![DialogContent::Button {
        skin: Some("default".to_string()),
        inner: vec![DialogContent::Text {
            value: t!("cancel").to_string(),
            styles: None,
        }],
        on_click: cancelled_event(id),
        styles: None,
    }];
    if !shortcut.is_empty() {
        actions.push(DialogContent::Button {
            skin: Some("solid".to_string()),
            inner: vec![DialogContent::Text {
                value: t!("done").to_string(),
                styles: None,
            }],
            on_click: accepted_event(id),
            styles: None,
        });
    }

    Dialog {
        identifier: id,
        width: 360.0,
        height: 180.0,
        title: vec![DialogContent::Text {
            value: t!("shortcut.register.title").to_string(),
            styles: None,
        }],
        content: vec![keys_box],
        footer: actions,
    }
}
