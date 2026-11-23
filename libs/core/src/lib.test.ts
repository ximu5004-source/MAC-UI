import { assert } from "@std/assert/assert";
import * as MacUiLib from "./lib.ts";

Deno.test("Library is importable on background (no window)", () => {
  assert(!!MacUiLib);
});
/// <reference lib="deno.ns" />
