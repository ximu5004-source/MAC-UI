import { strict as assert } from "node:assert";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { compile } from "svelte/compiler";

const stylesheet = readFileSync(new URL("../control-center.css", import.meta.url), "utf8");
const component = readFileSync(new URL("./QuickToggle.svelte", import.meta.url), "utf8");

function rule(selector: string) {
  const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const matches = [...stylesheet.matchAll(new RegExp(`${escaped}\\s*\\{([^}]*)\\}`, "g"))];
  assert.ok(matches.length > 0, `Missing material rule: ${selector}`);
  // The first rule is the normal material. System high-contrast overrides below
  // keep the same plain glyph and use CanvasText / Highlight ink.
  return matches[0]![1]!;
}

test("enabled and disabled quick toggles have the same plain glyph; only ink changes", () => {
  const enabled = rule('#root .quick-settings .quick-toggle[data-enabled="true"] .quick-toggle-glyph');
  assert.match(enabled, /color:\s*var\(--quick-enabled-ink\)/);
  assert.doesNotMatch(enabled, /(?:background|border(?:-color|-radius)?|box-shadow|filter|opacity)\s*:/);
  const glyph = rule("#root .quick-settings .quick-toggle-glyph");
  assert.match(glyph, /background:\s*transparent/);
  assert.match(glyph, /border:\s*0\s*;/);
  assert.match(glyph, /border-radius:\s*0\s*;/);
  assert.match(glyph, /color:\s*var\(--mac-ink\)/);
  const pill = rule("#root .quick-settings .quick-toggle-pill");
  assert.match(pill, /border:\s*1px solid var\(--mac-line\)/);
  assert.match(pill, /background:\s*var\(--mac-card\)/);
  assert.doesNotMatch(stylesheet, /quick-toggle[^}]*quick-toggle-glyph\s*\{[^}]*background:\s*(?!transparent)[A-Za-z#]/);
  const surface = rule("#root .slu-std-popover.mac-panel.quick-settings");
  assert.match(surface, /--quick-enabled-ink:\s*color-mix\(in srgb, var\(--mac-accent\) 50%, var\(--mac-ink\)\)/);
});

test("glyph artwork remains solid and has no added glow or nested blur", () => {
  for (const selector of [
    "#root .quick-settings .quick-toggle-glyph",
    "#root .quick-settings .quick-toggle-glyph .slu-icon",
    "#root .quick-settings .quick-toggle-pill button:disabled",
  ]) {
    const styles = rule(selector);
    assert.match(styles, /opacity:\s*1\s*;/);
    for (const effect of styles.matchAll(/(?:backdrop-filter|filter):\s*([^;]+);/g)) {
      assert.equal(effect[1]!.trim(), "none");
    }
  }
  assert.match(rule("#root .quick-settings .quick-toggle-glyph"), /box-shadow:\s*none/);
});

test("state glyph contrast does not depend on a white plate in representative light/dark frost", () => {
  function luminance(rgb: number[]) {
    return rgb.reduce((sum, channel, index) => {
      const srgb = channel / 255;
      const linear = srgb <= .04045 ? srgb / 12.92 : ((srgb + .055) / 1.055) ** 2.4;
      return sum + linear * [.2126, .7152, .0722][index]!;
    }, 0);
  }
  const samples = [
    // Fixture's medium-light blue and dark purple samples. This checks the
    // uniform outer pill coat only, not arbitrary apps beneath native glass.
    { background: [84, 117, 140], accent: [8, 107, 220], ink: 0, lightPill: true },
    { background: [43, 31, 44], accent: [121, 184, 255], ink: 255, lightPill: false },
  ];
  for (const sample of samples) {
    const frosted = sample.background.map((channel) => channel * .9 + 255 * .1);
    const outerPill = frosted.map((channel) => sample.lightPill ? channel * .86 + 255 * .14 : channel * .88);
    const glyph = sample.accent.map((channel) => (channel + sample.ink) / 2);
    const background = luminance(outerPill);
    const foreground = luminance(glyph);
    const contrast = (Math.max(background, foreground) + .05) / (Math.min(background, foreground) + .05);
    assert.ok(contrast >= 3, `Functional glyph sample contrast is ${contrast}`);
  }
});

test("restyling preserves native button semantics, split details and pending guard", () => {
  const result = compile(component, { filename: "QuickToggle.svelte", generate: "server" });
  assert.equal(result.warnings.length, 0);
  assert.match(component, /aria-pressed=\{enabled\}/);
  assert.match(component, /aria-label=\{\$t\("open_details", \{ name: label \}\)\}/);
  assert.equal((component.match(/disabled=\{disabled \|\| pending\}/g) || []).length, 2);
  assert.match(component, /if \(pending \|\| disabled\) return/);
  assert.match(component, /data-pending=\{pending\}/);
  assert.match(component, /role=\{failed \? "alert" : undefined\}/);
  assert.match(stylesheet, /button:focus-visible\s*\{[^}]*outline:\s*2px solid var\(--quick-enabled-ink\)/);
  assert.match(stylesheet, /prefers-reduced-motion:\s*reduce/);
  assert.match(stylesheet, /forced-colors:\s*active/);
});
