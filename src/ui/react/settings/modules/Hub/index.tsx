import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { useLocation } from "react-router";
import type { WidgetId } from "@macui/lib/types";
import { getResourceText } from "libs/ui/react/utils";
import { Icon } from "libs/ui/react/components/Icon";
import type { IconName } from "libs/ui/icons";
import { widgets } from "../../state/resources";
import { WidgetSettings } from "../resources/Widget/View";
import { DesktopGlassSettings } from "../resources/Widget/DesktopGlass";
import { AllWallpapersView } from "../resources/Wallpapers/AllView";
import { General } from "../general/infra";
import { Colors } from "../general/infra/Colors";
import { Shortcuts } from "../shortcuts/infrastructure";
import { SettingsByMonitor } from "../ByMonitor/infra";
import { AppsConfiguration } from "../appsConfigurations/infra/infra";
import { DeveloperTools } from "../developer/infra";
import { GlassMode } from "../../components/header/GlassMode";
import { widgetGroups } from "./catalog";
import cs from "./index.module.css";

function Section({ id, title, children }: { id: string; title: string; children: ReactNode }) {
  const icons: Record<string, IconName> = {
    glass: "TbSun",
    colors: "TbPalette",
    wallpapers: "TbWallpaper",
    shortcuts: "TbKeyboard",
    general: "TbSettings",
    monitors: "TbDeviceDesktop",
    applications: "TbApps",
    developer: "TbAdjustments",
  };
  return (
    <section data-section={id} className={cs.section} aria-labelledby={`section-${id}`}>
      <div className={cs.sectionHeading}>
        <span aria-hidden="true">
          <Icon iconName={icons[id] || "TbSettings"} size={19} />
        </span>
        <h2 id={`section-${id}`}>{title}</h2>
      </div>
      {children}
    </section>
  );
}

export function Hub() {
  const { pathname } = useLocation();
  const { t, i18n } = useTranslation();
  const category = pathname.slice(1);
  const list = Object.entries(widgetGroups)
    .filter(([, category]) => category === pathname)
    .flatMap(([id]) => widgets.value.filter((widget) => widget.id === id));
  const widgetSections = list.map((widget) => (
    <section
      key={widget.id}
      data-section={widget.id.split("/")[1]!}
      className={cs.section}
      aria-label={getResourceText(widget.metadata.displayName, i18n.language)}
    >
      <WidgetSettings widgetId={widget.id as WidgetId} />
    </section>
  ));
  return (
    <div className={cs.hub}>
      <div className={cs.intro}>
        <h1>{t(`settings_home.${category}`)}</h1>
        <p>{t(`settings_home.${category}_hint`)}</p>
      </div>
      <div className={`${cs.flow} ${category === "appearance" ? cs.appearanceFlow : ""}`} data-settings-flow>
        {category === "appearance"
          ? (
            <>
              <div className={cs.lane}>
                <Section id="glass" title={t("mac_local.color_scheme")}>
                  <GlassMode />
                </Section>
                <Section id="colors" title={t("mac_local.color_settings")}>
                  <Colors />
                </Section>
                <DesktopGlassSettings />
              </div>
              <div className={cs.lane}>{widgetSections}</div>
              <div className={cs.library}>
                <Section id="wallpapers" title={t("settings_detail.wallpaper_library")}>
                  <AllWallpapersView />
                </Section>
              </div>
            </>
          )
          : widgetSections}
        {category === "windows" && (
          <Section id="shortcuts" title={t("header.labels.shortcuts")}>
            <Shortcuts />
          </Section>
        )}
        {category === "system" && (
          <>
            <Section id="general" title={t("header.labels.general")}>
              <General />
            </Section>
            <Section id="monitors" title={t("header.labels.monitors")}>
              <SettingsByMonitor />
            </Section>
            <Section id="applications" title={t("header.labels.specific_apps")}>
              <AppsConfiguration />
            </Section>
            <Section id="developer" title={t("header.labels.developer")}>
              <DeveloperTools />
            </Section>
          </>
        )}
      </div>
    </div>
  );
}
