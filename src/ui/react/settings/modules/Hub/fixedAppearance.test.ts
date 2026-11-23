import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import test from "node:test";

const source = (path: string) => readFileSync(new URL(path, import.meta.url), "utf8");

test("settings no longer mount theme, icon pack or resource selection", () => {
  const routing = source("../../router.tsx");
  assert.doesNotMatch(routing, /ResourcesView|ThemeView|IconPacksView|PluginsView|WidgetsView/);
  assert.match(routing, /RoutePath\.Wallpapers\} Component=\{AllWallpapersView\}/);
  assert.match(routing, /path="theme" element=\{<Navigate to="\/appearance" replace/);
  assert.match(routing, /path="resources\/\*" element=\{<Navigate to="\/appearance" replace/);
  assert.equal(existsSync(new URL("../resources/Theme/View.tsx", import.meta.url)), false);
  assert.equal(existsSync(new URL("../resources/IconPacks.tsx", import.meta.url)), false);
});

test("hubs and search expose glass and wallpapers, not removed settings", () => {
  const hub = source("./index.tsx");
  const catalog = source("./catalog.ts");
  const resources = source("../../state/resources.ts");
  assert.match(hub, /section === "appearance" && <DesktopGlassSettings \/>/);
  assert.match(hub, /"\/wallpapers"/);
  assert.doesNotMatch(hub + catalog, /["`]\/resources|["`]\/theme\?|themes\.value|iconPacks\.value/);
  assert.doesNotMatch(resources, /StateGetThemes|StateThemesChanged|StateGetIconPacks|StateIconPacksChanged/);
  assert.match(catalog, /\^theme\$\|\^icon_pack\$/);
  assert.match(catalog, /widgets\.value\.filter\(\(w\) => !w\.hidden\)/);
});

test("both glass finishes keep the light palette and persist through the existing preference", () => {
  const mode = source("../../state/colorScheme.ts");
  const app = source("../../app.tsx");
  const controls = source("../../components/header/index.tsx");
  assert.match(mode, /SETTINGS_WIDGET = "@macui\/settings"/);
  assert.match(mode, /colorScheme === "dark" \? "dark" : "light"/);
  assert.doesNotMatch(mode + app, /prefers-color-scheme|useDarkMode|SystemSetDarkMode/);
  assert.match(mode, /dataset\.glassContrast = mode === "dark" \? "enhanced" : "original"/);
  assert.doesNotMatch(app, /darkAlgorithm/);
  const shared = source("../../../../../../libs/core/src/state/widget/color_scheme.ts");
  assert.match(shared, /root\.style\.colorScheme = "light"/);
  assert.match(mode, /\.\.\.settings\.value\.byWidget\[SETTINGS_WIDGET\]/);
  assert.match(mode, /colorScheme: mode/);
  assert.match(controls, /setColorScheme\(mode\)/);
});
