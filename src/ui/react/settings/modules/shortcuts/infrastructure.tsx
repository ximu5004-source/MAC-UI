import { invoke, SeelenCommand, Widget } from "@seelen-ui/lib";
import { Icon } from "libs/ui/react/components/Icon/index.tsx";
import { Button, Input, message, Switch, Tooltip } from "antd";
import { useTranslation } from "react-i18next";
import { useEffect, useId, useRef, useState } from "preact/hooks";

import {
  getShortcutsConfig,
  resetShortcuts,
  setShortcutsEnabled,
  type ShortcutEntry,
  shortcutGroups,
  shortcutsError,
  updateShortcut,
} from "./application.ts";
import { settings } from "../../state/mod";

import { SettingsGroup, SettingsOption, SettingsSubGroup } from "../../components/SettingsBox/index.tsx";
import { Note } from "../../components/Note/index.tsx";
import { ResourceText } from "libs/ui/react/components/ResourceText/index.tsx";
import { isWidgetEnabled, patchWidgetConfig } from "../resources/Widget/application.ts";
import Compact from "antd/es/space/Compact";
import { createShortcutCaptureEvent, type ShortcutCapture, startShortcutCapture } from "./shortcutCapture.ts";

// The backend has one capture session per settings webview, not one per row.
let activeCapture: ShortcutCapture | null = null;

export function Shortcuts() {
  const { enabled } = getShortcutsConfig();
  const groups = shortcutGroups.value;

  const { t } = useTranslation();

  function mapEntry(entry: ShortcutEntry) {
    return <Shortcut key={entry.id} entry={entry} onChanged={(keys) => updateShortcut(entry, keys)} />;
  }

  return (
    <>
      {shortcutsError.value.size > 0 && (
        <Note type="error">
          <b>{t("shortcuts.duplicate_error")}:</b>
          {t("shortcuts.duplicate_error_hint")}
        </Note>
      )}

      <SettingsGroup>
        <SettingsOption
          label={t("shortcuts.enable")}
          tip={t("shortcuts.enable_tooltip")}
          action={<Switch value={enabled} onChange={setShortcutsEnabled} />}
        />
        <SettingsOption
          label={t("shortcuts.reset")}
          action={
            <Button onClick={resetShortcuts}>
              <Icon iconName="RiResetLeftLine" />
            </Button>
          }
        />
      </SettingsGroup>

      {/* Virtual Desktop */}
      <SettingsGroup>
        <SettingsSubGroup label={t("header.labels.virtual_desk")}>
          {groups.system.vdMain.map(mapEntry)}
        </SettingsSubGroup>
      </SettingsGroup>

      <SettingsGroup>
        <SettingsSubGroup label={t("header.labels.virtual_desk")}>
          {groups.system.vdSwitch.map(mapEntry)}
        </SettingsSubGroup>
      </SettingsGroup>

      <SettingsGroup>
        <SettingsSubGroup label={t("header.labels.virtual_desk")}>
          {groups.system.vdMove.map(mapEntry)}
        </SettingsSubGroup>
      </SettingsGroup>

      <SettingsGroup>
        <SettingsSubGroup label={t("header.labels.virtual_desk")}>
          {groups.system.vdSend.map(mapEntry)}
        </SettingsSubGroup>
      </SettingsGroup>

      {/* Widget groups */}
      {Array.from(groups.byWidget.values()).map(({ widget, entries }) => (
        <SettingsGroup key={widget.id}>
          <SettingsSubGroup label={<ResourceText text={widget.metadata.displayName} />}>
            {widget.id === "@seelen/apps-menu" && (
              <SettingsOption
                label={t("widget.enable")}
                action={
                  <Switch
                    aria-label={t("widget.enable")}
                    value={isWidgetEnabled(widget.id)}
                    onChange={(enabled) => patchWidgetConfig(widget.id, { enabled })}
                  />
                }
              />
            )}
            {entries.map(mapEntry)}
          </SettingsSubGroup>
        </SettingsGroup>
      ))}

      {/* Misc */}
      <SettingsGroup>{groups.system.misc.map(mapEntry)}</SettingsGroup>
    </>
  );
}

interface ShortcutProps {
  entry: ShortcutEntry;
  onChanged: (keys: string[]) => void;
}

function Shortcut({ entry, onChanged }: ShortcutProps) {
  const { id, label, keys, readonly: sugestedReadonly } = entry;
  const labelId = useId();
  const captureRef = useRef<ShortcutCapture | null>(null);
  const [startingCapture, setStartingCapture] = useState(false);

  useEffect(() => () => {
    captureRef.current?.cancel();
    if (activeCapture === captureRef.current) activeCapture = null;
  }, []);

  const { devTools, unlockShortcuts } = settings.value;
  const readonly = sugestedReadonly && !(devTools && unlockShortcuts);

  const isAttachedWidgetEnabled = entry.widgetId ? isWidgetEnabled(entry.widgetId) : true;

  const { t } = useTranslation();
  const hasError = shortcutsError.value.has(id);

  async function onEdit() {
    if (readonly || startingCapture) return;

    activeCapture?.cancel();
    setStartingCapture(true);
    const callbackEvent = createShortcutCaptureEvent();
    const capture = startShortcutCapture(
      {
        listen: (onFinished) =>
          Widget.getCurrent().webview.listen<null | string[]>(
            callbackEvent,
            ({ payload }) => onFinished(payload),
          ),
        request: () => invoke(SeelenCommand.RequestToUserInputShortcut, { callbackEvent }),
      },
      onChanged,
      () => message.warning(t("shortcuts.single_key_hint")),
    );
    captureRef.current = capture;
    activeCapture = capture;

    try {
      await capture.ready;
    } catch (error) {
      console.error("Shortcut capture request failed", error);
      message.error(t("shortcuts.capture_failed"));
    } finally {
      setStartingCapture(false);
    }
  }

  let inputTooltip: string | undefined = undefined;
  if (hasError) {
    inputTooltip = t("shortcuts.duplicate_error");
  } else if (!isAttachedWidgetEnabled) {
    inputTooltip = t("shortcuts.disabled_tooltip");
  }

  return (
    <SettingsOption
      label={
        <span id={labelId}>
          <ResourceText text={label} />
        </span>
      }
      action={
        <Compact>
          <Tooltip title={inputTooltip} placement="left">
            <Input
              aria-labelledby={labelId}
              value={keys.join(" + ")}
              status={hasError ? "error" : undefined}
              readOnly
              disabled={!isAttachedWidgetEnabled}
            />
          </Tooltip>

          <Tooltip title={readonly ? t("shortcuts.readonly_tooltip") : undefined}>
            <Button
              type="primary"
              aria-label={t("shortcuts.edit")}
              aria-describedby={labelId}
              disabled={readonly}
              loading={startingCapture}
              onClick={onEdit}
            >
              <Icon iconName="IoPencilOutline" aria-hidden="true" />
            </Button>
          </Tooltip>
        </Compact>
      }
    />
  );
}
