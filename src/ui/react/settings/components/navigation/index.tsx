import { Icon } from "libs/ui/react/components/Icon";
import { getResourceText } from "libs/ui/react/utils";
import { Input } from "antd";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { NavLink, useLocation } from "react-router";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { RouteIcons, RoutePath } from "./routes";
import { resourcesWithUpdate, themes, widgets } from "../../state/resources";
import { settings } from "../../state/mod";
import cs from "./index.module.css";

export function Navigation() {
  const { t, i18n } = useTranslation();
  const location = useLocation();
  const [query, setQuery] = useState("");
  const window = getCurrentWebviewWindow();
  const routeItem = (route: RoutePath) => ({
    route,
    label: t(`header.labels.${route === "/" ? "home" : route.slice(1)}`),
    icon: RouteIcons[route],
  });
  const groups = [
    { label: t("mac_ui.personal"), items: [RoutePath.Home, RoutePath.General, RoutePath.Resource].map(routeItem) },
    {
      label: t("mac_ui.desktop"),
      items: widgets.value.filter((widget) => !widget.hidden).map((widget) => ({
        route: `/widget?${new URLSearchParams({ id: widget.id })}`,
        label: getResourceText(widget.metadata.displayName, i18n.language),
        icon: <Icon iconName={(widget.icon as any) || "TbLayoutGrid"} />,
      })),
    },
    {
      label: t("mac_ui.appearance"),
      items: themes.value.filter((theme) => theme.settings.length && settings.value.activeThemes.includes(theme.id))
        .map((theme) => ({
          route: `/theme?${new URLSearchParams({ id: theme.id })}`,
          label: getResourceText(theme.metadata.displayName, i18n.language),
          icon: <Icon iconName="TbPalette" />,
        })),
    },
    {
      label: t("mac_ui.advanced"),
      items: [RoutePath.SettingsByMonitor, RoutePath.SettingsByApplication, RoutePath.Shortcuts, RoutePath.DevTools]
        .map(routeItem),
    },
  ].map((group) => ({
    ...group,
    items: group.items.filter((item) => item.label.toLocaleLowerCase().includes(query.trim().toLocaleLowerCase())),
  }));
  return (
    <nav className={cs.navigation} aria-label={t("mac_ui.navigation")}>
      <div className={cs.windowBar} data-tauri-drag-region>
        <div className={cs.trafficLights}>
          <button className={cs.close} aria-label={t("close")} title={t("close")} onClick={() => window.close()}>
            <span>×</span>
          </button>
          <button
            className={cs.minimize}
            aria-label={t("mac_ui.minimize")}
            title={t("mac_ui.minimize")}
            onClick={() => window.minimize()}
          >
            <span>−</span>
          </button>
          <button
            className={cs.maximize}
            aria-label={t("mac_ui.maximize")}
            title={t("mac_ui.maximize")}
            onClick={() => window.toggleMaximize()}
          >
            <span>+</span>
          </button>
        </div>
      </div>
      <NavLink to="/" className={cs.identity}>
        <img src="./mac-ui.svg" alt="" width={44} height={44} />
        <span>
          <strong>MAC UI</strong>
          <small>{t("mac_ui.by_jona")}</small>
        </span>
      </NavLink>
      <div className={cs.search}>
        <Input
          allowClear
          prefix={<Icon iconName="TbSearch" />}
          aria-label={t("mac_ui.search")}
          placeholder={t("mac_ui.search")}
          value={query}
          onChange={(event) => setQuery((event.target as HTMLInputElement).value)}
        />
      </div>
      <div className={cs.body}>
        {groups.map((group, index) =>
          !!group.items.length && (
            <section className={cs.group} key={group.label}>
              <h2>{group.label}</h2>
              {group.items.map((item) => {
                const target = new URL(item.route, "https://settings.local");
                const active = (location.pathname === target.pathname ||
                  (item.route === RoutePath.Resource && location.pathname.startsWith("/resources/"))) &&
                  (!target.search || new URLSearchParams(location.search).get("id") === target.searchParams.get("id"));
                return (
                  <NavLink
                    key={item.route}
                    to={item.route}
                    aria-current={active ? "page" : undefined}
                    className={`${cs.item} ${active ? cs.active : ""}`}
                    title={item.label}
                  >
                    <span className={cs.iconWrapper} data-tone={index} aria-hidden="true">{item.icon}</span>
                    <span className={cs.label}>{item.label}</span>
                    {item.route === RoutePath.Resource && resourcesWithUpdate.value.length > 0 && (
                      <span className={cs.badge} aria-label={t("mac_ui.updates_available")} />
                    )}
                  </NavLink>
                );
              })}
            </section>
          )
        )}
        {groups.every((group) => !group.items.length) && (
          <p className={cs.empty} role="status">{t("mac_ui.no_results")}</p>
        )}
      </div>
      <NavLink
        to={RoutePath.Extras}
        className={`${cs.about} ${location.pathname === RoutePath.Extras ? cs.active : ""}`}
      >
        <Icon iconName="TbInfoCircle" />
        <span>{t("mac_ui.about")}</span>
      </NavLink>
    </nav>
  );
}
