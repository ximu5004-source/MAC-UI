import type { i18n as I18n, TFunction } from "i18next";
import { getResourceText } from "libs/ui/react/utils";
import { widgets } from "../../state/resources";

export const sections = [
  { route: "/desktop", key: "desktop", icon: "TbLayoutGrid" },
  { route: "/dock", key: "dock", icon: "TbApps" },
  { route: "/toolbar", key: "toolbar", icon: "TbBell" },
  { route: "/windows", key: "windows", icon: "TbAppWindow" },
  { route: "/appearance", key: "appearance", icon: "TbPalette" },
  { route: "/system", key: "system", icon: "TbSettings" },
] as const;

export const widgetGroups: Record<string, string> = {
  "@macui/desktop-shell": "/desktop",
  "@macui/weg": "/dock",
  "@macui/fancy-toolbar": "/toolbar",
  "@macui/apps-menu": "/dock",
  "@macui/window-manager": "/windows",
  "@macui/task-switcher": "/windows",
  "@macui/flyouts": "/toolbar",
  "@macui/wallpaper-manager": "/appearance",
};
export const widgetRoute = (id: string) =>
  widgetGroups[id]
    ? `${widgetGroups[id]}?${new URLSearchParams({ section: id.split("/")[1]! })}`
    : `/widget?${new URLSearchParams({ id })}`;

export const pageRoutes: Record<string, string> = {
  "/general": "/system?section=general",
  "/devices": "/system?section=monitors",
  "/advanced": "/system?section=developer",
  "/shortcuts": "/windows?section=shortcuts",
  "/monitors": "/system?section=monitors",
  "/specific_apps": "/system?section=applications",
  "/developer": "/system?section=developer",
  "/wallpapers": "/appearance?section=wallpapers",
};
export interface Destination {
  route: string;
  label: string;
  icon: string;
  group: string;
  hint?: string;
}

export function destinations(t: TFunction, i18n: I18n): Destination[] {
  const items: Destination[] = sections.map((s) => ({ ...s, label: t(`settings_home.${s.key}`), group: s.route }));
  const pages = [
    ["/general", "general", "system"],
    ["/shortcuts", "shortcuts", "windows"],
    ["/monitors", "monitors", "system"],
    ["/specific_apps", "specific_apps", "system"],
    ["/developer", "developer", "system"],
    ["/extras", "extras", "system"],
    ["/wallpapers", "wallpaper", "appearance"],
  ];
  for (const [route, key, group] of pages) {
    items.push({
      route: pageRoutes[route!] || route!,
      label: t(`header.labels.${key}`),
      icon: "TbChevronRight",
      group: `/${group}`,
    });
  }
  for (const widget of widgets.value.filter((w) => !w.hidden)) {
    items.push({
      route: widgetRoute(widget.id),
      label: getResourceText(widget.metadata.displayName, i18n.language),
      icon: widget.icon || "TbLayoutGrid",
      group: widgetGroups[widget.id] || "/system",
    });
  }
  return items;
}

export function searchSettings(query: string, t: TFunction, i18n: I18n): Destination[] {
  const q = query.trim().toLocaleLowerCase();
  if (!q) return [];
  const pages = destinations(t, i18n);
  const results = pages.filter((p) => p.label.toLocaleLowerCase().includes(q));
  const prefixes: [string, string][] = [
    ["general", pageRoutes["/general"]!],
    ["performance", pageRoutes["/general"]!],
    ["weg", widgetRoute("@macui/weg")],
    ["toolbar", widgetRoute("@macui/fancy-toolbar")],
    ["wm", widgetRoute("@macui/window-manager")],
    ["wall", widgetRoute("@macui/wallpaper-manager")],
    ["shortcuts", pageRoutes["/shortcuts"]!],
    ["desktop_glass", "/appearance"],
    ["desktop_controls", "/desktop"],
    ["apps_configurations", pageRoutes["/specific_apps"]!],
  ];
  const add = (value: unknown, route: string) => {
    if (typeof value === "string" && value.length < 90 && value.toLocaleLowerCase().includes(q)) {
      const page = pages.find((p) => p.route === route);
      if (page && !results.some((p) => p.route.startsWith(route) && p.label === value)) {
        results.push({
          ...page,
          route: route + (route.includes("?") ? "&" : "?") + new URLSearchParams({ find: value }),
          label: value,
          hint: page.label,
        });
      }
    } else if (value && typeof value === "object") {
      Object.entries(value).filter(([key]) => !/(description|hint|tooltip|_tip$|^theme$|^icon_pack$)/i.test(key))
        .forEach(([, v]) => add(v, route));
    }
  };
  prefixes.forEach(([key, route]) => add(t(key, { returnObjects: true }), route));
  const walk = (defs: any[], route: string) =>
    defs.forEach((def) => {
      if (def.group) {
        add(getResourceText(def.group.label, i18n.language), route);
        walk(def.group.items, route);
      } else if (def.label) add(getResourceText(def.label, i18n.language), route);
    });
  widgets.value.filter((w) => !w.hidden).forEach((w) => walk(w.settings, widgetRoute(w.id)));
  return results.slice(0, 24);
}
