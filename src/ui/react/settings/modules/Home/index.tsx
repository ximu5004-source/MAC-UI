import { NavLink } from "react-router";
import { useTranslation } from "react-i18next";
import { Icon } from "libs/ui/react/components/Icon";
import { ResourceText } from "libs/ui/react/components/ResourceText";
import { widgets } from "../../state/resources";
import { isWidgetEnabled } from "../resources/Widget/application";
import { EnvConfig } from "../shared/config/infra";
import cs from "./overview.module.css";

export function Home() {
  const { t } = useTranslation();
  const featured = ["@seelen/desktop-shell", "@seelen/weg", "@seelen/fancy-toolbar", "@seelen/wallpaper-manager"];
  return (
    <div className={cs.overview}>
      <section className={cs.identity}>
        <img src="./mac-ui.svg" alt="" width={80} height={80} />
        <h1>MAC UI</h1>
        <p>{t("mac_ui.overview_subtitle")}</p>
        <small>{t("mac_ui.by_jona")} · {EnvConfig.version}</small>
      </section>
      <h2>{t("mac_ui.your_desktop")}</h2>
      <div className={cs.modules}>
        {featured.flatMap((id) => {
          const widget = widgets.value.find((widget) => widget.id === id);
          return widget
            ? [
              <NavLink key={id} to={`/widget?${new URLSearchParams({ id })}`} className={cs.module}>
                <span className={cs.moduleIcon} aria-hidden="true">
                  <Icon iconName={(widget.icon as any) || "TbLayoutGrid"} />
                </span>
                <span className={cs.moduleText}>
                  <strong>
                    <ResourceText text={widget.metadata.displayName} />
                  </strong>
                  <small>{t(isWidgetEnabled(widget.id) ? "mac_ui.enabled" : "mac_ui.disabled")}</small>
                </span>
                <Icon iconName="TbChevronRight" />
              </NavLink>,
            ]
            : [];
        })}
      </div>
      <p className={cs.note}>
        <Icon iconName="TbWallpaper" />
        {t("mac_ui.wallpaper_hint")}
      </p>
      <h2>{t("mac_ui.personal")}</h2>
      <div className={cs.links}>
        <NavLink to="/general">
          <Icon iconName="TbSettings" />
          <span>{t("header.labels.general")}</span>
          <Icon iconName="TbChevronRight" />
        </NavLink>
        <NavLink to="/resources">
          <Icon iconName="TbPalette" />
          <span>{t("header.labels.resources")}</span>
          <Icon iconName="TbChevronRight" />
        </NavLink>
        <NavLink to="/extras">
          <Icon iconName="TbInfoCircle" />
          <span>{t("mac_ui.about")}</span>
          <Icon iconName="TbChevronRight" />
        </NavLink>
      </div>
    </div>
  );
}
