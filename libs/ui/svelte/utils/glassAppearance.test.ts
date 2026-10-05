import { test } from "node:test";
import assert from "node:assert/strict";
import { DEFAULT_GLASS_APPEARANCE, normalizeGlassAppearance, glassAppearanceVariables, glassAppearanceStyle } from "../../utils/glassAppearance";

test("existing settings retain the neutral 10% material without new gloss", () => {
  for (const raw of [null, undefined, {}, "bad", []]) {
    assert.deepEqual(normalizeGlassAppearance(raw), DEFAULT_GLASS_APPEARANCE);
  }
});
test("glass controls clamp malformed values and preserve valid zeroes", () => {
  assert.deepEqual(normalizeGlassAppearance({ dockTransparency: -1, dockGloss: 200, iconEdgeTransparency: 0, iconEdgeGloss: 32.8 }), {
    dockTransparency: 0, dockGloss: 100, iconEdgeTransparency: 0, iconEdgeGloss: 33,
  });
  assert.deepEqual(normalizeGlassAppearance({ dockTransparency: NaN, dockGloss: Infinity, iconEdgeGloss: "80" }), DEFAULT_GLASS_APPEARANCE);
});
test("transparency increases as material opacity decreases; icon and dock are independent", () => {
  const vars = glassAppearanceVariables({ dockTransparency: 100, dockGloss: 45, iconEdgeTransparency: 0, iconEdgeGloss: 75 });
  assert.equal(vars["--mac-dock-alpha"], "0");
  assert.equal(vars["--mac-icon-edge-alpha"], "1");
  assert.equal(vars["--mac-dock-gloss"], "0.45");
  assert.equal(vars["--mac-icon-gloss"], "0.75");
  assert(!Object.keys(vars).includes("opacity"));
});
test("appearance JSON round-trips and generated CSS cannot contain arbitrary input", () => {
  const saved = { dockTransparency: 58, dockGloss: 72, iconEdgeTransparency: 81, iconEdgeGloss: 35 };
  assert.deepEqual(normalizeGlassAppearance(JSON.parse(JSON.stringify(saved))), saved);
  assert(!glassAppearanceStyle({ dockGloss: "1; background:red" }).includes("background"));
});
