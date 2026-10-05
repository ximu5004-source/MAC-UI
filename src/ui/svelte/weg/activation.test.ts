import assert from "node:assert/strict";
import { test } from "node:test";
import { isDockPrimaryClick, restoreOrLaunch } from "./activation.ts";

test("Dock ignores secondary buttons and clicks already consumed by a drag", () => {
  assert.equal(isDockPrimaryClick({ button: 0, defaultPrevented: false }), true);
  assert.equal(isDockPrimaryClick({ button: 0, defaultPrevented: true }), false);
  assert.equal(isDockPrimaryClick({ button: 1, defaultPrevented: false }), false);
  assert.equal(isDockPrimaryClick({ button: 2, defaultPrevented: false }), false);
});

test("tray-resident applications are restored without relaunching", async () => {
  let launches = 0;
  await restoreOrLaunch("wechat", async () => true, async () => {
    launches++;
  });
  assert.equal(launches, 0);
});
test("native launch fallback runs only on an explicit restore decline, never an error", async () => {
  let launches = 0;
  await restoreOrLaunch("closed", async () => false, async () => {
    launches++;
  });
  assert.equal(launches, 1);
  await assert.rejects(restoreOrLaunch("failed", async () => {
    throw Error("tray unavailable");
  }, async () => {
    launches++;
  }));
  assert.equal(launches, 1);
});
test("rapid repeated clicks share one activation", async () => {
  let complete!: (value: boolean) => void;
  const restore = new Promise<boolean>((resolve) => {
    complete = resolve;
  });
  let launches = 0;
  const first = restoreOrLaunch("repeat", () => restore, async () => {
    launches++;
  });
  const second = restoreOrLaunch("repeat", () => restore, async () => {
    launches++;
  });
  assert.equal(first, second);
  complete(false);
  await first;
  assert.equal(launches, 1);
});
