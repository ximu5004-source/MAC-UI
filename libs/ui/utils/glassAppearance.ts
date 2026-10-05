import type { WidgetId } from "@seelen-ui/lib/types";

export const GLASS_SETTINGS_WIDGET = "@seelen/desktop-shell" as WidgetId;

/** Shared by settings previews and live widgets. Percentages are transparency,
 * not opacity: 100 means no material. Never change the artwork's opacity. */
export interface GlassAppearance {
  dockTransparency: number;
  dockGloss: number;
  iconEdgeTransparency: number;
  iconEdgeGloss: number;
}

export const DEFAULT_GLASS_APPEARANCE: Readonly<GlassAppearance> = Object.freeze({
  dockTransparency: 90,
  dockGloss: 0,
  iconEdgeTransparency: 90,
  iconEdgeGloss: 0,
});

export function normalizeGlassAppearance(raw: unknown): GlassAppearance {
  const source = raw && typeof raw === "object" ? raw as Record<string, unknown> : {};
  const percent = (key: keyof GlassAppearance) => {
    const value = source[key];
    return typeof value === "number" && Number.isFinite(value)
      ? Math.round(Math.max(0, Math.min(100, value))) : DEFAULT_GLASS_APPEARANCE[key];
  };
  return {
    dockTransparency: percent("dockTransparency"), dockGloss: percent("dockGloss"),
    iconEdgeTransparency: percent("iconEdgeTransparency"), iconEdgeGloss: percent("iconEdgeGloss"),
  };
}

export function glassAppearanceVariables(raw: unknown): Record<string, string> {
  const value = normalizeGlassAppearance(raw);
  const dockAlpha = (100 - value.dockTransparency) / 100;
  const iconAlpha = (100 - value.iconEdgeTransparency) / 100;
  return {
    "--mac-dock-alpha": String(dockAlpha),
    "--mac-dock-gloss": String(value.dockGloss / 100),
    "--mac-icon-edge-alpha": String(iconAlpha),
    "--mac-icon-gloss": String(value.iconEdgeGloss / 100),
  };
}

export function glassAppearanceStyle(raw: unknown): string {
  return Object.entries(glassAppearanceVariables(raw)).map(([key, value]) => `${key}:${value}`).join(";");
}
