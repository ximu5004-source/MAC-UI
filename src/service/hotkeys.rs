mod lifecycle;

use seelen_core::state::shortcuts::ResolvedShortcut;
use slu_ipc::{AppIpc, messages::AppMessage};
use uuid::Uuid;
use win_hotkeys::{
    Hotkey, HotkeyBuilder, HotkeyManager, TriggerTiming, VKey, error::WHKError,
    events::KeyboardInputEvent,
};

use crate::{
    app_management::kill_all_seelen_ui_processes, error::Result, exit, get_async_handler, log_error,
};

static SHORTCUT_CAPTURE: lifecycle::CaptureLifecycle = lifecycle::CaptureLifecycle::new();
static SHORTCUT_REGISTRATION: lifecycle::RegistrationSession<Uuid> =
    lifecycle::RegistrationSession::new();

pub fn apply_shortcuts(shortcuts: Vec<ResolvedShortcut>) -> Result<()> {
    SHORTCUT_CAPTURE.with_registry(!shortcuts.is_empty(), start_keyboard_capturing, || {
        replace_shortcut_registry(shortcuts)
    })
}

fn start_keyboard_capturing() -> Result<()> {
    if let Err(err) = HotkeyManager::start_keyboard_capturing() {
        match err {
            WHKError::AlreadyStarted => {}
            others => return Err(others.into()),
        }
    };
    Ok(())
}

fn replace_shortcut_registry(shortcuts: Vec<ResolvedShortcut>) -> Result<()> {
    let manager = HotkeyManager::current();
    manager.unregister_all()?;

    if shortcuts.is_empty() {
        return Ok(());
    }

    // Preserve an explicit user binding for the physical right Windows key.
    let explicit_right_win = has_explicit_right_win(&shortcuts);

    'registration: for s in shortcuts {
        if s.keys.is_empty() {
            continue 'registration;
        }

        let mut vkeys = Vec::new();
        for key in &s.keys {
            let vkey = match VKey::from_keyname(key) {
                Ok(vkey) => vkey,
                Err(e) => {
                    log::warn!("Failed to parse shortcut {:?} error: {e}", s.keys);
                    continue 'registration;
                }
            };
            vkeys.push(vkey);
        }

        let command = s.command.clone();
        let mut hotkey = Hotkey::from_keys(&vkeys).action(move || {
            log::trace!("Hotkey triggered: {command:?}");
            match command.as_slice() {
                [a, b] if a == "service" && b == "force-restart" => {
                    log_error!(kill_all_seelen_ui_processes());
                }
                [a, b] if a == "service" && b == "force-quit" => {
                    crate::EXITING.store(true, std::sync::atomic::Ordering::SeqCst);
                    log_error!(kill_all_seelen_ui_processes());
                    exit(0);
                }
                _ => {
                    let cmd = command.clone();
                    get_async_handler().spawn(async move {
                        log_error!(AppIpc::send(AppMessage::Cli(cmd)).await);
                    });
                }
            }
        });

        configure_trigger_timing(&mut hotkey, vkeys.len());

        if !explicit_right_win && s.keys.len() == 1 && s.keys[0].eq_ignore_ascii_case("Win") {
            // "Win" maps to LWin in the library, while trigger keys are exact.
            // Register the right key as a separate strict release shortcut.
            let mut right_win = Hotkey::from_keys(&[VKey::RWin]);
            right_win.callback = hotkey.callback.clone();
            configure_trigger_timing(&mut right_win, 1);
            log_error!(manager.register_hotkey(right_win));
        }
        log_error!(manager.register_hotkey(hotkey));
    }
    Ok(())
}

fn has_explicit_right_win(shortcuts: &[ResolvedShortcut]) -> bool {
    shortcuts
        .iter()
        .any(|s| s.keys.len() == 1 && VKey::from_keyname(&s.keys[0]).ok() == Some(VKey::RWin))
}

fn configure_trigger_timing(hotkey: &mut HotkeyBuilder, key_count: usize) {
    if key_count == 1 {
        hotkey.trigger_timing = TriggerTiming::OnKeyUp;
        hotkey.strict_sequence = true;
    }
}

#[cfg(test)]
mod launchpad_win_tests {
    use super::*;
    use win_hotkeys::state::KeyboardState;

    fn single_win(win: VKey) -> Hotkey {
        let mut hotkey = Hotkey::from_keys(&[VKey::from_keyname("Win").unwrap()]).action(|| {});
        configure_trigger_timing(&mut hotkey, 1);
        hotkey.trigger(win).build()
    }

    #[test]
    fn single_win_triggers_on_release_for_either_windows_key() {
        for win in [VKey::LWin, VKey::RWin] {
            let hotkey = single_win(win);
            let mut state = KeyboardState::new();
            state.keydown(win);
            state.keyup(win);
            assert!(hotkey.is_trigger_state(&win, &state));
            assert_eq!(hotkey.trigger_timing, TriggerTiming::OnKeyUp);
            assert!(hotkey.strict_sequence);
        }
    }

    #[test]
    fn windows_combinations_do_not_trigger_launchpad_on_release() {
        for win in [VKey::LWin, VKey::RWin] {
            let hotkey = single_win(win);
            for other in [VKey::E, VKey::D, VKey::L, VKey::Tab, VKey::V] {
                let mut state = KeyboardState::new();
                state.keydown(win);
                state.keydown(other);
                state.keyup(other);
                state.keyup(win);
                assert!(!hotkey.is_trigger_state(&win, &state));
            }
        }
    }

    #[test]
    fn explicit_right_windows_binding_keeps_priority_over_generic_win_alias() {
        let mut shortcuts = vec![ResolvedShortcut {
            command: vec![],
            keys: vec!["Win".into()],
        }];
        assert!(!has_explicit_right_win(&shortcuts));
        shortcuts.push(ResolvedShortcut {
            command: vec![],
            keys: vec!["RWin".into()],
        });
        assert!(has_explicit_right_win(&shortcuts));
    }
}

pub fn stop_app_shortcuts() {
    // Only service shutdown calls this. The upstream hook cannot be restarted
    // after stop; temporarily disabling shortcuts must only clear the registry.
    HotkeyManager::stop_keyboard_capturing();
}

pub async fn start_shortcut_registration(request_id: Uuid) -> Result<()> {
    // The shortcut editor also needs capture when no shortcuts were enabled
    // during startup. Keep it under the same process-wide lifecycle gate.
    SHORTCUT_CAPTURE.with_registry(true, start_keyboard_capturing, || Ok(()))?;
    let hkm = HotkeyManager::current();

    let handle = tokio::runtime::Handle::current();
    let on_free_keyboard = move || {
        // free_keyboard dispatches this callback asynchronously. An older
        // callback must not remove the listener of a newer recording session.
        SHORTCUT_REGISTRATION.finish(request_id, || {
            HotkeyManager::current().remove_global_keyboard_listener();
        });
        handle.spawn(async move {
            let _ = send_registering_to_app(request_id, None).await;
        });
    };

    let handle = tokio::runtime::Handle::current();
    let on_keyboard_event = move |event| {
        handle.spawn(async move {
            match event {
                KeyboardInputEvent::KeyDown { key, state } => {
                    if key == VKey::Escape {
                        return;
                    }
                    let keys = state.pressing.iter().map(|vkey| vkey.to_string()).collect();
                    let _ = send_registering_to_app(request_id, Some(keys)).await;
                }
                KeyboardInputEvent::KeyUp { .. } => {}
            }
        });
    };

    send_registering_to_app(request_id, Some(vec![])).await?;
    SHORTCUT_REGISTRATION.begin(request_id, || {
        hkm.steal_keyboard(on_free_keyboard);
        hkm.set_global_keyboard_listener(on_keyboard_event);
    });
    Ok(())
}

pub async fn stop_shortcut_registration() -> Result<()> {
    SHORTCUT_REGISTRATION.cancel(|| {
        let hkm = HotkeyManager::current();
        hkm.remove_global_keyboard_listener();
        hkm.free_keyboard();
    });
    Ok(())
}

async fn send_registering_to_app(request_id: Uuid, hotkey: Option<Vec<String>>) -> Result<()> {
    AppIpc::send(AppMessage::Cli(vec![
        "popup".to_owned(),
        "internal-set-shortcut".to_owned(),
        serde_json::to_string(&hotkey)?,
        request_id.to_string(),
    ]))
    .await?;
    Ok(())
}
