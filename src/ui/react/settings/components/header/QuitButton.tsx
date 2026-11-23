import { process } from "@macui/lib/tauri";
import { Button } from "antd";
import { Icon } from "libs/ui/react/components/Icon";
import { useTranslation } from "react-i18next";
import { hasChanges, restoreToLastSaved } from "../../state/mod";
import { settingsDialogs } from "../SettingsDialogs";
import cs from "./index.module.css";

export function QuitButton() {
  const { t } = useTranslation();
  function quit() {
    if (!hasChanges.value) {
      void process.exit(0);
      return;
    }
    settingsDialogs.confirm({
      title: t("settings_exit.unsaved_title"),
      content: t("settings_exit.unsaved_hint"),
      centered: true,
      okText: t("settings_exit.discard_exit"),
      cancelText: t("mac_ui.keep_editing"),
      okButtonProps: { danger: true },
      onOk: async () => {
        restoreToLastSaved();
        await process.exit(0);
      },
    });
  }
  return (
    <Button
      type="text"
      className={cs.quit}
      title={t("settings_exit.hint")}
      onClick={quit}
      icon={<Icon iconName="TbLogout" />}
    >
      {t("settings_exit.label")}
    </Button>
  );
}
