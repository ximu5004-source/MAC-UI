//! Read-only UIA fallback for client-owned notification popups. Some current
//! clients expose UIA text/live regions without an equivalent MSAA Alert tree.
use std::time::{Duration, Instant};

use windows::{
    Win32::UI::Accessibility::{
        CUIAutomation8, IUIAutomation2, IUIAutomationCacheRequest, IUIAutomationElement,
        IUIAutomationTreeWalker, TreeScope_Element, UIA_AriaRolePropertyId,
        UIA_BoundingRectanglePropertyId, UIA_ControlTypePropertyId, UIA_DocumentControlTypeId,
        UIA_EditControlTypeId, UIA_IsOffscreenPropertyId, UIA_ListItemControlTypeId,
        UIA_LiveSettingPropertyId, UIA_MenuControlTypeId, UIA_NamePropertyId,
        UIA_TextControlTypeId,
    },
    core::BSTR,
};

use super::{MAX_DEPTH, MAX_NODES, MAX_TEXT_CHARS, PopupRule};
use crate::{
    error::Result,
    windows_api::{Com, window::Window},
};

pub(super) struct UiaReader {
    automation: IUIAutomation2,
    cache: IUIAutomationCacheRequest,
    walker: IUIAutomationTreeWalker,
}

impl UiaReader {
    /// Construct/use/drop on the collector's windowless MTA thread only.
    pub(super) fn new() -> Result<Self> {
        let automation: IUIAutomation2 = Com::create_instance(&CUIAutomation8)?;
        unsafe {
            // Reading a notification must never focus or activate its client.
            automation.SetAutoSetFocus(false)?;
            automation.SetConnectionTimeout(250)?;
            automation.SetTransactionTimeout(250)?;
            let cache = automation.CreateCacheRequest()?;
            cache.SetTreeScope(TreeScope_Element)?;
            for property in [
                UIA_NamePropertyId,
                UIA_ControlTypePropertyId,
                UIA_IsOffscreenPropertyId,
                UIA_BoundingRectanglePropertyId,
                UIA_AriaRolePropertyId,
                UIA_LiveSettingPropertyId,
            ] {
                cache.AddProperty(property)?;
            }
            let walker = automation.RawViewWalker()?;
            Ok(Self {
                automation,
                cache,
                walker,
            })
        }
    }

    pub(super) fn text(&self, window: Window, rule: PopupRule) -> Option<String> {
        let root = unsafe {
            self.automation
                .ElementFromHandleBuildCache(window.hwnd(), &self.cache)
        }
        .ok()?;
        let mut reader = TreeReader {
            owner: self,
            rule,
            started: Instant::now(),
            visited: 0,
            texts: Vec::new(),
        };
        reader.visit(&root, 0, false);
        (!reader.texts.is_empty()).then(|| reader.texts.join("\n"))
    }
}

fn notification_region(role: &str, live: i32) -> bool {
    role.split_ascii_whitespace()
        .any(|value| matches!(value, "alert" | "status"))
        || matches!(live, 1 | 2) // UIA LiveSetting Polite / Assertive
}

fn excluded_control(control_type: i32) -> bool {
    [
        UIA_EditControlTypeId.0,
        UIA_DocumentControlTypeId.0,
        UIA_MenuControlTypeId.0,
    ]
    .contains(&control_type)
}

struct TreeReader<'a> {
    owner: &'a UiaReader,
    rule: PopupRule,
    started: Instant,
    visited: usize,
    texts: Vec<String>,
}

impl TreeReader<'_> {
    fn full(&self) -> bool {
        self.visited >= MAX_NODES
            || self.started.elapsed() >= Duration::from_millis(350)
            || self.texts.len() >= 16
    }

    fn visit(&mut self, node: &IUIAutomationElement, depth: usize, in_notification: bool) {
        if self.full() || depth > MAX_DEPTH {
            return;
        }
        self.visited += 1;
        // All properties below are cached with the element in one remote read.
        let control_type = unsafe { node.CachedControlType() }
            .ok()
            .map(|value| value.0)
            .unwrap_or(0);
        if excluded_control(control_type)
            || unsafe { node.CachedIsOffscreen() }.is_ok_and(|value| value.as_bool())
        {
            return;
        }
        let role = unsafe { node.GetCachedPropertyValue(UIA_AriaRolePropertyId) }
            .ok()
            .and_then(|value| BSTR::try_from(&value).ok())
            .map(|value| value.to_string())
            .unwrap_or_default();
        let live = unsafe { node.GetCachedPropertyValue(UIA_LiveSettingPropertyId) }
            .ok()
            .and_then(|value| i32::try_from(&value).ok())
            .unwrap_or(0);
        let in_notification = in_notification || notification_region(&role, live);
        let wanted = match self.rule {
            PopupRule::WeChat => control_type == UIA_TextControlTypeId.0,
            PopupRule::DingTalk => control_type == UIA_ListItemControlTypeId.0,
            PopupRule::Alert => {
                in_notification
                    && [UIA_TextControlTypeId.0, UIA_ListItemControlTypeId.0]
                        .contains(&control_type)
            }
        };
        if wanted
            && unsafe { node.CachedBoundingRectangle() }
                .is_ok_and(|rect| rect.right > rect.left && rect.bottom > rect.top)
        {
            if let Ok(name) = unsafe { node.CachedName() } {
                let text: String = name
                    .to_string()
                    .trim()
                    .chars()
                    .take(MAX_TEXT_CHARS)
                    .collect();
                if !text.is_empty() && !self.texts.contains(&text) && self.texts.len() < 16 {
                    self.texts.push(text);
                }
            }
        }
        if self.full() {
            return;
        }
        let mut child = unsafe {
            self.owner
                .walker
                .GetFirstChildElementBuildCache(node, &self.owner.cache)
        }
        .ok();
        while let Some(current) = child {
            self.visit(&current, depth + 1, in_notification);
            if self.full() {
                break;
            }
            child = unsafe {
                self.owner
                    .walker
                    .GetNextSiblingElementBuildCache(&current, &self.owner.cache)
            }
            .ok();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic_uia_text_requires_an_explicit_notification_region() {
        assert!(notification_region("alert", 0));
        assert!(notification_region("status", 0));
        assert!(notification_region("region alert", 0));
        assert!(notification_region("", 1));
        assert!(notification_region("", 2));
        for role in ["window", "dialog", "menu", "list", "alertdialog", ""] {
            assert!(!notification_region(role, 0));
        }
        assert!(!notification_region("", 3));
    }

    #[test]
    fn editable_documents_and_menus_are_never_read_as_notification_text() {
        assert!(excluded_control(UIA_EditControlTypeId.0));
        assert!(excluded_control(UIA_DocumentControlTypeId.0));
        assert!(excluded_control(UIA_MenuControlTypeId.0));
        assert!(!excluded_control(UIA_TextControlTypeId.0));
        assert!(!excluded_control(UIA_ListItemControlTypeId.0));
    }
}
