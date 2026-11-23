use macui_core::system_state::SysTrayIcon;
use macui_ipc::messages::IconEventData;

/// Windows identifies tray items by GUID or HWND + uID, never by tooltip,
/// executable or changing HICON pixels. Keep the original frontend stable ID
/// while resolving either identity advertised by later partial updates.
pub(super) fn find_icon<'a>(
    icons: &'a [SysTrayIcon],
    data: &IconEventData,
) -> Result<Option<&'a SysTrayIcon>, ()> {
    if let Some(guid) = data.guid {
        if let Some(icon) = icons.iter().find(|icon| icon.guid == Some(guid)) {
            return Ok(Some(icon));
        }
    }

    let (Some(handle), Some(uid)) = (data.window_handle, data.uid) else {
        return Ok(None);
    };
    let mut candidates = icons.iter().filter(|icon| {
        icon.window_handle == Some(handle)
            && icon.uid == Some(uid)
            // Different explicit GUIDs are distinct, even if HWND + uID match.
            && (data.guid.is_none() || icon.guid.is_none())
    });
    let first = candidates.next();
    if candidates.next().is_some() {
        // A GUID-less packet cannot identify one of several GUID icons. Do not
        // manufacture a third icon or deliver clicks to an arbitrary target.
        return Err(());
    }
    Ok(first)
}

#[cfg(test)]
mod tests {
    use super::*;
    use macui_core::system_state::SysTrayIconId;

    fn icon(handle: isize, uid: u32, guid: Option<uuid::Uuid>) -> SysTrayIcon {
        SysTrayIcon {
            stable_id: guid
                .map(SysTrayIconId::Guid)
                .unwrap_or(SysTrayIconId::HandleUid(handle, uid)),
            window_handle: Some(handle),
            uid: Some(uid),
            guid,
            executable: None,
            notification: None,
            notification_revision: 0,
            tooltip: "Task Manager".into(),
            icon_handle: None,
            icon_path: None,
            icon_image_hash: None,
            callback_message: Some(1024),
            version: Some(4),
            is_visible: true,
        }
    }

    fn data(handle: Option<isize>, uid: Option<u32>, guid: Option<uuid::Uuid>) -> IconEventData {
        IconEventData {
            window_handle: handle,
            uid,
            guid,
            tooltip: None,
            icon_handle: None,
            notification: None,
            callback_message: None,
            version: None,
            is_visible: None,
        }
    }

    #[test]
    fn guid_icon_matches_guidless_modify_and_delete() {
        let icons = [icon(100, 1, Some(uuid::Uuid::from_u128(1)))];
        let found = find_icon(&icons, &data(Some(100), Some(1), None))
            .unwrap()
            .unwrap();
        assert_eq!(found.stable_id, icons[0].stable_id);
    }

    #[test]
    fn handle_icon_retains_stable_id_when_it_later_advertises_guid() {
        let guid = uuid::Uuid::from_u128(1);
        let mut icons = [icon(100, 1, None)];
        assert_eq!(
            find_icon(&icons, &data(Some(100), Some(1), Some(guid)))
                .unwrap()
                .unwrap()
                .stable_id,
            SysTrayIconId::HandleUid(100, 1)
        );
        icons[0].guid = Some(guid);
        assert_eq!(
            find_icon(&icons, &data(None, None, Some(guid)))
                .unwrap()
                .unwrap()
                .stable_id,
            SysTrayIconId::HandleUid(100, 1)
        );
    }

    #[test]
    fn same_app_separate_ids_and_different_guids_remain_distinct() {
        let icons = [
            icon(100, 1, Some(uuid::Uuid::from_u128(1))),
            icon(100, 2, None),
        ];
        assert!(
            find_icon(&icons, &data(Some(100), Some(3), None))
                .unwrap()
                .is_none()
        );
        assert!(
            find_icon(
                &icons,
                &data(Some(100), Some(1), Some(uuid::Uuid::from_u128(2)))
            )
            .unwrap()
            .is_none()
        );
        assert_eq!(
            find_icon(&icons, &data(Some(100), Some(2), None))
                .unwrap()
                .unwrap()
                .uid,
            Some(2)
        );
    }

    #[test]
    fn ambiguous_guidless_event_cannot_create_a_duplicate() {
        let icons = [
            icon(100, 1, Some(uuid::Uuid::from_u128(1))),
            icon(100, 1, Some(uuid::Uuid::from_u128(2))),
        ];
        assert!(find_icon(&icons, &data(Some(100), Some(1), None)).is_err());
        assert_eq!(
            find_icon(
                &icons,
                &data(Some(100), Some(1), Some(uuid::Uuid::from_u128(2)))
            )
            .unwrap()
            .unwrap()
            .guid,
            Some(uuid::Uuid::from_u128(2))
        );
    }
}
