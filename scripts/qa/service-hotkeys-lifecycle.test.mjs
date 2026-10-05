import assert from "node:assert/strict";
import { mkdirSync, readFileSync } from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../../", import.meta.url));
const hotkeys = readFileSync(path.join(root, "src/service/hotkeys.rs"), "utf8");
const processing = readFileSync(path.join(root, "src/service/cli/processing.rs"), "utf8");
const messages = readFileSync(path.join(root, "libs/slu-ipc/src/messages.rs"), "utf8");

test("all SetShortcuts requests, including an empty set, replace the registry", () => {
  const branch = processing.match(/SvcAction::SetShortcuts\(shortcuts\) => \{([\s\S]*?)\n\s*\}/)?.[1];
  assert.ok(branch, "SetShortcuts handler missing");
  assert.match(branch, /crate::hotkeys::apply_shortcuts\(shortcuts\)\?;/);
  assert.doesNotMatch(branch, /stop_app_shortcuts|stop_keyboard_capturing|is_empty/);
});

test("the production replacement path uses the tested process-wide capture gate", () => {
  const apply = hotkeys.match(/pub fn apply_shortcuts\([^\n]*\) -> Result<\(\)> \{([\s\S]*?)\n\}/)?.[1];
  assert.ok(apply, "apply_shortcuts function missing");
  assert.match(apply, /SHORTCUT_CAPTURE\.with_registry\(/);
  assert.match(apply, /!shortcuts\.is_empty\(\)/);
  assert.match(apply, /start_keyboard_capturing/);
  assert.match(apply, /replace_shortcut_registry\(shortcuts\)/);

  const replace = hotkeys.match(/fn replace_shortcut_registry\([^\n]*\) -> Result<\(\)> \{([\s\S]*?)\n\}/)?.[1];
  assert.ok(replace, "registry replacement function missing");
  assert.match(replace, /manager\.unregister_all\(\)\?;/);
  assert.ok(replace.indexOf("unregister_all") < replace.indexOf("shortcuts.is_empty"));
  assert.doesNotMatch(replace, /stop_keyboard_capturing|start_keyboard_capturing/);
});

test("standalone production lifecycle regression tests pass without a native hook", () => {
  const outDir = path.join(root, "target/qa-hotkeys");
  mkdirSync(outDir, { recursive: true });
  const binary = path.join(outDir, `service-hotkeys-lifecycle-${process.pid}.exe`);
  const compile = spawnSync("rustc", [
    "--edition", "2024", "--test",
    path.join(root, "src/service/hotkeys/lifecycle.rs"), "-o", binary,
  ], { cwd: root, encoding: "utf8", windowsHide: true });
  assert.ifError(compile.error);
  assert.equal(compile.status, 0, compile.stdout + compile.stderr);
  const run = spawnSync(binary, [], { cwd: root, encoding: "utf8", windowsHide: true });
  assert.ifError(run.error);
  assert.equal(run.status, 0, run.stdout + run.stderr);
  assert.match(run.stdout, /nonempty_empty_nonempty_keeps_capture_and_replaces_the_registry .* ok/);
  assert.match(run.stdout, /delayed_old_completion_cannot_remove_the_new_recording_listener .* ok/);
  assert.match(run.stdout, /8 passed; 0 failed/);
});

test("each recording event retains its request UUID through the IPC and CLI boundary", () => {
  assert.match(messages, /StartShortcutRegistration\s*\{\s*request_id: uuid::Uuid,/);
  assert.match(processing, /SvcAction::StartShortcutRegistration \{ request_id \} =>/);
  assert.match(processing, /start_shortcut_registration\(request_id\)\.await\?/);
  assert.match(hotkeys, /pub async fn start_shortcut_registration\(request_id: Uuid\)/);
  assert.match(hotkeys, /send_registering_to_app\(request_id, None\)/);
  assert.match(hotkeys, /send_registering_to_app\(request_id, Some\(keys\)\)/);
  assert.match(hotkeys, /send_registering_to_app\(request_id, Some\(vec!\[\]\)\)/);
  assert.match(hotkeys, /serde_json::to_string\(&hotkey\)\?,\s*request_id\.to_string\(\),/);
});

test("old free callbacks cannot clear the current listener and stop cleans up synchronously", () => {
  assert.match(hotkeys, /SHORTCUT_REGISTRATION\.finish\(request_id, \|\| \{/);
  assert.match(hotkeys, /SHORTCUT_REGISTRATION\.begin\(request_id, \|\| \{/);
  const stop = hotkeys.match(/pub async fn stop_shortcut_registration\(\) -> Result<\(\)> \{([\s\S]*?)\n\}/)?.[1];
  assert.ok(stop);
  assert.match(stop, /SHORTCUT_REGISTRATION\.cancel\(/);
  assert.ok(stop.indexOf("remove_global_keyboard_listener") < stop.indexOf("free_keyboard()"));
  assert.doesNotMatch(stop, /stop_keyboard_capturing/);
});
