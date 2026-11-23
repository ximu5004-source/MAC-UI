import { process } from "@macui/lib/tauri";
import { Button } from "antd";
import { Icon } from "libs/ui/react/components/Icon";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { NavLink, useLocation } from "react-router";
import { SettingsSearch } from "../navigation";
import { QuitButton } from "./QuitButton";
import { hasChanges, needRestart, restoreToLastSaved, saveSettings } from "../../state/mod";
import cs from "./index.module.css";

export function Header() {
  const { pathname } = useLocation();
  const { t } = useTranslation();
  const [saving, setSaving] = useState(false);
  const nativeWindow = getCurrentWebviewWindow();
  async function save() {
    if (!hasChanges.value || saving) return;
    setSaving(true);
    try {
      if (await saveSettings() && needRestart.value) await process.relaunch();
    } finally {
      setSaving(false);
    }
  }
  return (
    <header className={cs.header} data-tauri-drag-region>
      <div className={cs.title}>
        <NavLink to="/" className={cs.brand} aria-label={t("header.labels.home")}>
          <img src="./mac-ui.svg" alt="" width={30} height={30} />
          <span>MAC UI</span>
        </NavLink>
        {pathname !== "/" && (
          <NavLink to="/" className={cs.back}>
            <Icon iconName="TbChevronLeft" />
            <span>{t("settings_home.back")}</span>
          </NavLink>
        )}
      </div>
      <div className={cs.actions}>
        <SettingsSearch />
        {hasChanges.value && (
          <>
            <span className={cs.status} role="status">{t("mac_ui.pending")}</span>
            <Button disabled={saving} onClick={restoreToLastSaved}>{t("mac_ui.revert")}</Button>
            <Button type="primary" loading={saving} aria-busy={saving} onClick={save}>
              {t(needRestart.value ? "save_and_restart" : "save")}
            </Button>
          </>
        )}
        <QuitButton />
        <div className={cs.windowControls}>
          <button
            aria-label={t("mac_ui.minimize")}
            title={t("mac_ui.minimize")}
            onClick={() => nativeWindow.minimize()}
          >
            <svg
              width="14"
              height="14"
              viewBox="0 0 16 16"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.5"
              strokeLinecap="round"
              aria-hidden="true"
            >
              <path d="M3 8h10" />
            </svg>
          </button>
          <button
            aria-label={t("mac_ui.maximize")}
            title={t("mac_ui.maximize")}
            onClick={() => nativeWindow.toggleMaximize()}
          >
            <svg
              width="14"
              height="14"
              viewBox="0 0 16 16"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.5"
              aria-hidden="true"
            >
              <rect x="3" y="3" width="10" height="10" rx="1.5" />
            </svg>
          </button>
          <button aria-label={t("close")} title={t("close")} onClick={() => nativeWindow.close()}>
            <svg
              width="14"
              height="14"
              viewBox="0 0 16 16"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.5"
              strokeLinecap="round"
              aria-hidden="true"
            >
              <path d="m4 4 8 8M12 4l-8 8" />
            </svg>
          </button>
        </div>
      </div>
    </header>
  );
}
