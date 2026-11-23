import { Modal } from "antd";
import { useLayoutEffect } from "react";

// State-level save operations also need the active settings theme and locale.
export const settingsDialogs = { error: Modal.error, confirm: Modal.confirm };

export function SettingsDialogHost() {
  const [modal, holder] = Modal.useModal();
  useLayoutEffect(() => {
    settingsDialogs.error = modal.error;
    settingsDialogs.confirm = modal.confirm;
    return () => {
      settingsDialogs.error = Modal.error;
      settingsDialogs.confirm = Modal.confirm;
    };
  }, [modal]);
  return holder;
}
