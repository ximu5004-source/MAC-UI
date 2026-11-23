import { useEffect, useState } from "react";
import { Button, InputNumber, message, Select, Slider, Switch } from "antd";
import { event } from "@macui/lib/tauri";
import { useTranslation } from "react-i18next";
import {
  DESKTOP_CONTROLS_CHANGED,
  DESKTOP_CONTROLS_REQUEST,
  DESKTOP_CONTROLS_RESPONSE,
  type DesktopControls,
  type DesktopControlsResponse,
} from "libs/ui/utils/desktopControls";
import { SettingsGroup, SettingsOption } from "../../../components/SettingsBox";
import cs from "./desktop-glass.module.css";

function OrganizerGlass({ controls, change, busy }: {
  controls: DesktopControls;
  change: (patch: Partial<DesktopControls>) => Promise<void>;
  busy: boolean;
}) {
  const { t } = useTranslation();
  const [draft, setDraft] = useState({ glassTransparency: controls.glassTransparency, glassBlur: controls.glassBlur });
  useEffect(() => {
    setDraft({ glassTransparency: controls.glassTransparency, glassBlur: controls.glassBlur });
  }, [controls.glassTransparency, controls.glassBlur]);
  return (
    <SettingsGroup title={t("desktop_controls.glass")} description={t("desktop_controls.glass_hint")} icon="TbDroplet">
      {(["glassTransparency", "glassBlur"] as const).map((key) => (
        <div className={cs.control} key={key}>
          <div className={cs.controlLabel}>
            <label id={`organizer-${key}`}>{t(`desktop_controls.${key}`)}</label>
            <output>{draft[key]}%</output>
          </div>
          <Slider
            ariaLabelledByForHandle={`organizer-${key}`}
            min={0}
            max={100}
            value={draft[key]}
            disabled={busy}
            onChange={(value) => setDraft((current) => ({ ...current, [key]: value }))}
            onChangeComplete={(value) =>
              change({ [key]: value })}
          />
          <div className={cs.rangeEnds} aria-hidden="true">
            <span>{t(`desktop_controls.${key}_low`)}</span>
            <span>{t(`desktop_controls.${key}_high`)}</span>
          </div>
        </div>
      ))}
      <Button
        disabled={busy}
        onClick={() =>
          change({ glassTransparency: 88, glassBlur: 35 })}
      >
        {t("desktop_controls.glass_reset")}
      </Button>
    </SettingsGroup>
  );
}

async function requestControls(patch?: Partial<DesktopControls>): Promise<DesktopControls> {
  const id = crypto.randomUUID();
  let resolveReply!: (result: DesktopControls) => void;
  let rejectReply!: (error: Error) => void;
  const reply = new Promise<DesktopControls>((resolve, reject) => {
    resolveReply = resolve;
    rejectReply = reject;
  });
  const stop = await event.listen<DesktopControlsResponse>(DESKTOP_CONTROLS_RESPONSE, ({ payload }) => {
    if (payload.id !== id) return;
    if (payload.controls) resolveReply(payload.controls);
    else rejectReply(new Error(payload.error));
  });
  const timeout = setTimeout(() => rejectReply(new Error("desktop-unavailable")), 4000);
  try {
    await event.emit(DESKTOP_CONTROLS_REQUEST, { id, patch });
    return await reply;
  } finally {
    clearTimeout(timeout);
    stop();
  }
}

export function DesktopOrganizerSettings() {
  const { t } = useTranslation();
  const [controls, setControls] = useState<DesktopControls | null>(null);
  const [busy, setBusy] = useState(false);
  const [unavailable, setUnavailable] = useState(false);
  async function refresh() {
    setBusy(true);
    try {
      setControls(await requestControls());
      setUnavailable(false);
    } catch {
      setUnavailable(true);
    } finally {
      setBusy(false);
    }
  }
  useEffect(() => {
    let disposed = false;
    const subscription = event.listen<DesktopControls>(DESKTOP_CONTROLS_CHANGED, ({ payload }) => {
      if (!disposed) {
        setControls(payload);
        setUnavailable(false);
      }
    });
    void requestControls().then((value) => {
      if (!disposed) setControls(value);
    }).catch(() => {
      if (!disposed) setUnavailable(true);
    });
    return () => {
      disposed = true;
      void subscription.then((stop) => stop());
    };
  }, []);
  async function change(patch: Partial<DesktopControls>) {
    setBusy(true);
    try {
      setControls(await requestControls(patch));
    } catch (error) {
      message.error(t("desktop_controls.failed"));
      console.error(error);
    } finally {
      setBusy(false);
    }
  }
  if (!controls || unavailable) {
    return (
      <SettingsGroup
        title={t("desktop_controls.title")}
        description={t(unavailable ? "desktop_controls.unavailable" : "desktop_controls.loading")}
      >
        <Button onClick={refresh} loading={busy}>{t("desktop_controls.refresh")}</Button>
      </SettingsGroup>
    );
  }
  return (
    <>
      <OrganizerGlass controls={controls} change={change} busy={busy} />
      <SettingsGroup title={t("desktop_controls.icons")} description={t("desktop_controls.live")} icon="TbLayoutGrid">
        <SettingsOption
          label={t("desktop_controls.arrange")}
          description={t("desktop_controls.arrange_hint")}
          action={
            <Select
              aria-label={t("desktop_controls.arrange")}
              value={controls.arrangeMode}
              disabled={busy}
              options={[{ value: "category", label: t("desktop_controls.category") }, {
                value: "name",
                label: t("desktop_controls.name"),
              }]}
              onChange={(arrangeMode) => change({ arrangeMode })}
              style={{ width: "100%" }}
            />
          }
        />
        <SettingsOption
          label={t("desktop_controls.size")}
          action={
            <Select
              aria-label={t("desktop_controls.size")}
              value={controls.iconSize}
              disabled={busy}
              options={[...new Set([32, 48, 64, 80, 96, controls.iconSize])].sort((a, b) => a - b).map((value) => ({
                value,
                label: `${value} px`,
              }))}
              onChange={(iconSize) => change({ iconSize })}
              style={{ width: "100%" }}
            />
          }
        />
      </SettingsGroup>
      <SettingsGroup
        title={t("desktop_controls.spacing")}
        description={t("desktop_controls.live")}
        icon="TbSpacingHorizontal"
      >
        {(["horizontalSpacing", "verticalSpacing"] as const).map((key) => (
          <SettingsOption
            key={key}
            label={t(`desktop_controls.${key}`)}
            action={
              <InputNumber
                aria-label={t(`desktop_controls.${key}`)}
                min={0}
                max={64}
                precision={0}
                value={controls[key]}
                disabled={busy}
                addonAfter="px"
                onChange={(value) => {
                  if (value !== null) void change({ [key]: value });
                }}
              />
            }
          />
        ))}
      </SettingsGroup>
      <SettingsGroup
        title={t("desktop_controls.layout")}
        description={t("desktop_controls.layout_hint")}
        icon="TbAdjustments"
      >
        {(["snapBoxes", "snapToGrid", "showGrid", "layoutLocked"] as const).map((key) => (
          <SettingsOption
            key={key}
            label={t(`desktop_controls.${key}`)}
            description={t(`desktop_controls.${key}_hint`)}
            action={
              <Switch
                aria-label={t(`desktop_controls.${key}`)}
                checked={controls[key]}
                disabled={busy}
                onChange={(value) => change({ [key]: value })}
              />
            }
          />
        ))}
      </SettingsGroup>
    </>
  );
}
