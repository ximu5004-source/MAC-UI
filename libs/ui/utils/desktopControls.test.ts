import assert from "node:assert/strict";
import { test } from "node:test";
import { validateDesktopControlsPatch } from "./desktopControls.ts";
test("desktop settings accept only display controls with valid values", () => {
  for (
    const patch of [
      { horizontalSpacing: 65 },
      { verticalSpacing: -1 },
      { snapBoxes: 1 },
      { glassBlur: -1 },
      { glassTransparency: 101 },
      { glassBlur: 0.5 },
      { iconSize: 0 },
      { iconSize: NaN },
      { arrangeMode: "unknown" },
      { positions: {} },
      {
        layoutLocked: "true",
      },
    ]
  ) {
    assert.throws(() => validateDesktopControlsPatch(patch as never));
  }
  validateDesktopControlsPatch({ arrangeMode: "category", snapToGrid: false });
  validateDesktopControlsPatch({ glassTransparency: 100, glassBlur: 0 });
  validateDesktopControlsPatch({ horizontalSpacing: 0, verticalSpacing: 64, snapBoxes: false });
});
