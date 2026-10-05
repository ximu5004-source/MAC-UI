import { test } from "node:test";
import assert from "node:assert/strict";
import { needsPowerConfirmation } from "./actionPolicy";

test("destructive session actions require explicit confirmation", () => {
  for (const action of ["shutdown", "reboot", "log_out"] as const) assert.equal(needsPowerConfirmation(action), true);
  for (const action of ["lock", "suspend", "hibernate"] as const) assert.equal(needsPowerConfirmation(action), false);
});
