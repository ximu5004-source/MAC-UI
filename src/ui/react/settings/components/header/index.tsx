import { process } from "@seelen-ui/lib/tauri";
import { ResourceText } from "libs/ui/react/components/ResourceText/index.tsx";
import { Button } from "antd";
import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { NavLink, useLocation, useSearchParams } from "react-router";

import { RouteExtraInfo } from "./ExtraInfo.tsx";
import cs from "./index.module.css";
import { themes as themeList, widgets as widgetList } from "../../state/resources.ts";
import { hasChanges, needRestart, restoreToLastSaved, saveSettings } from "../../state/mod.ts";

export const Header = () => {
  const location = useLocation();
  const [searchParams] = useSearchParams();
  const { t } = useTranslation();
  const [saving, setSaving] = useState(false);

  const save = async () => {
    if (hasChanges.value && !saving) {
      setSaving(true);
      try {
        const saved = await saveSettings();
        if (saved && needRestart.value) {
          await process.relaunch();
        }
      } finally {
        setSaving(false);
      }
    }
  };

  const saveBtnLabel = needRestart.value ? t("save_and_restart") : t("save");

  let label: React.ReactNode = <span>null!?</span>;
  let parts = location.pathname === "/" ? ["home"] : location.pathname.split("/").filter(Boolean);

  if (parts[0] === "widget") {
    const widgetId = searchParams.get("id");
    const widget = widgetList.value.find((w) => w.id === widgetId);
    label = widget ? <ResourceText text={widget.metadata.displayName} /> : <span>{widgetId}</span>;
  } else if (parts[0] === "theme") {
    const themeId = searchParams.get("id");
    const theme = themeList.value.find((t) => t.id === themeId);
    label = theme ? <ResourceText text={theme.metadata.displayName} /> : <span>{themeId}</span>;
  } else {
    if (parts[0] === "wallpaper") {
      parts = ["resources", "wallpaper", "config"];
    }

    label = parts.map((part, idx) => (
      <React.Fragment key={part}>
        {idx !== parts.length - 1
          ? (
            <NavLink to={`/${parts.slice(0, idx + 1).join("/")}`} className={cs.part}>
              {t(`header.labels.${part}`)}
            </NavLink>
          )
          : <span className={cs.part}>{t(`header.labels.${part}`)}</span>}
        {++idx < parts.length ? ">" : ""}
      </React.Fragment>
    ));
  }

  const ExtraInfo = RouteExtraInfo[location.pathname];

  return (
    <header className={cs.header} data-tauri-drag-region>
      <div className={cs.title}>
        {label}
        {ExtraInfo && <ExtraInfo />}
      </div>
      <div className={cs.actions}>
        <span className={cs.status} role="status">{hasChanges.value ? t("mac_ui.pending") : t("mac_ui.saved")}</span>
        <Button
          style={{ minWidth: 60 }}
          type="default"
          disabled={!hasChanges.value || saving}
          onClick={restoreToLastSaved}
        >
          {t("mac_ui.revert")}
        </Button>
        <Button
          style={{ minWidth: 60 }}
          type="primary"
          disabled={!hasChanges.value}
          loading={saving}
          onClick={save}
        >
          {saveBtnLabel}
        </Button>
      </div>
    </header>
  );
};
