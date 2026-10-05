import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { compile } from "svelte/compiler";

const source = (name: string) => readFileSync(new URL(name, import.meta.url), "utf8");
const app = source("./app.svelte");
const css = source("./user-menu.css");
const profile = source("./components/UserProfile.svelte");
const folder = source("./components/UserFolder.svelte");

test("account menu opts into the same neutral native frost, without wallpaper-only foreground sampling", () => {
  assert.match(app, /slu-std-popover mac-panel mac-frosted-surface user-popup/);
  assert.match(source("./index.ts"), /trackFrostedSurfaces\("\.user-popup\.mac-frosted-surface"\)/);
  assert.match(css, /border-radius: 28px/);
  assert.match(css, /background: var\(--mac-glass-surface\)/);
  assert.match(css, /backdrop-filter: var\(--mac-glass-blur\)/);
  assert.match(css, /html\[data-native-frosted="true"\][^{]*\{\s*backdrop-filter: none;/);
  assert.doesNotMatch(css, /drop-shadow|linear-gradient/);
  for (const effect of css.matchAll(/text-shadow:\s*([^;]+);/g)) assert.equal(effect[1]!.trim(), "none");
});

test("bounded account sheet has one scroll owner, and no nested file-list scrolling", () => {
  assert.match(app, /class="user-popup-content"/);
  assert.match(css, /\.user-popup-content\s*\{[^}]*overflow-y: auto;/s);
  assert.match(css, /\.file-list\s*\{[^}]*max-height: none;[^}]*overflow: visible;/s);
  assert.match(css, /width: min\(368px, var\(--mac-panel-max-width, 400px\)\)/);
  assert.match(css, /max-height: var\(--mac-panel-max-height, 680px\)/);
  assert.match(css, /prefers-reduced-transparency/);
  assert.match(css, /forced-colors/);
});

test("directory open and disclosure are separate native buttons; existing explorer/file actions remain", () => {
  assert.match(folder, /<button class="user-directory-open"[^>]*onclick=\{openOnExplorer\}/);
  assert.match(
    folder,
    /<button class="user-directory-toggle"[^>]*onclick=\{toggleFolder\}[^>]*aria-expanded=\{isOpen\}/,
  );
  assert.match(folder, /aria-controls=\{`user-files-\$\{type\}`\}/);
  assert.doesNotMatch(folder, /role="button"|onkeydown=/);
  assert.match(folder, /SeelenCommand\.OpenFile/);
  assert.match(source("./components/FilePreview.svelte"), /SeelenCommand\.SelectFileOnExplorer/);
  assert.match(css, /button:focus-visible\s*\{\s*outline: 2px solid var\(--mac-accent\)/);
});

test("session entry dismisses the account menu before opening the existing power confirmation", () => {
  assert.match(profile, /await Widget\.self\.hide\(\);\s*await invoke\(SeelenCommand\.TriggerWidget/s);
  assert.match(profile, /id: "@seelen\/power-menu" as WidgetId/);
  assert.doesNotMatch(profile, /SeelenCommand\.LogOut|SeelenCommand\.Shutdown|SeelenCommand\.Lock/);
  assert.match(profile, /disabled=\{openingPower\}/);
  assert.match(profile, /aria-label=\{\$t\("profile\.power_and_session"\)\}/);
  assert.match(app, /event\.key !== "Escape" \|\| event\.repeat \|\| closing/);
});

test("account and directory components compile without accessibility warnings", () => {
  for (const name of ["app.svelte", "components/UserProfile.svelte", "components/UserFolder.svelte"]) {
    const result = compile(source("./" + name), { filename: name, generate: "server" });
    assert.deepEqual(result.warnings, []);
  }
});
