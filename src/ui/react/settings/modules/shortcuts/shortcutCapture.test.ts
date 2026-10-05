import assert from "node:assert/strict";
import test from "node:test";
import {
  acceptedShortcut,
  createShortcutCaptureEvent,
  type ShortcutCapturePorts,
  type ShortcutCaptureResult,
  startShortcutCapture,
} from "./shortcutCapture.ts";

function captureHarness(request?: (finish: (keys: ShortcutCaptureResult) => void) => Promise<unknown>) {
  let handler: (keys: ShortcutCaptureResult) => void = () => {};
  let cleanupCount = 0;
  const calls: string[] = [];
  const ports: ShortcutCapturePorts = {
    listen: (onFinished) => {
      calls.push("listen");
      handler = onFinished;
      return Promise.resolve(() => {
        calls.push("unlisten");
        cleanupCount++;
      });
    },
    request: async () => {
      calls.push("request");
      return await request?.((keys) => handler(keys));
    },
  };
  return { ports, calls, finish: (keys: ShortcutCaptureResult) => handler(keys), cleanupCount: () => cleanupCount };
}

test("standalone Win and either physical Win key are accepted", () => {
  for (const key of ["Win", "LWin", "RWin"]) {
    assert.deepEqual(acceptedShortcut([key]), ["Win"]);
  }
});

test("ordinary single keys and modifier-only alternatives do not intercept typing", () => {
  for (const key of ["A", "1", "Space", "Ctrl", "Shift", "Alt"]) {
    assert.equal(acceptedShortcut([key]), null);
  }
});

test("existing combinations are retained without mutating the captured sequence", () => {
  const keys = ["Ctrl", "LWin", "K"];
  assert.deepEqual(acceptedShortcut(keys), ["Ctrl", "Win", "K"]);
  assert.deepEqual(keys, ["Ctrl", "LWin", "K"]);
  assert.deepEqual(acceptedShortcut(["Alt", "A"]), ["Alt", "A"]);
});

test("null cancellation and empty bindings keep the old value", async () => {
  for (const result of [null, []]) {
    const harness = captureHarness();
    let selected = ["Alt", "A"];
    let rejectionCount = 0;
    const capture = startShortcutCapture(harness.ports, (keys) => selected = keys, () => rejectionCount++);
    await capture.ready;
    harness.finish(result);
    assert.deepEqual(selected, ["Alt", "A"]);
    assert.equal(rejectionCount, 0);
    assert.equal(harness.cleanupCount(), 1);
  }
});

test("listener is ready before request and an immediate Win completion is saved once", async () => {
  const harness = captureHarness((finish) => {
    finish(["Win"]);
    return Promise.resolve();
  });
  const selected: string[][] = [];
  const capture = startShortcutCapture(harness.ports, (keys) => selected.push(keys));
  await capture.ready;
  harness.finish(["Ctrl", "K"]);
  assert.deepEqual(harness.calls, ["listen", "request", "unlisten"]);
  assert.deepEqual(selected, [["Win"]]);
  capture.cancel();
  assert.equal(harness.cleanupCount(), 1);
});

test("request waits for asynchronous listener registration", async () => {
  let resolveListener: ((unlisten: () => void) => void) | undefined;
  let requested = false;
  const capture = startShortcutCapture({
    listen: () => new Promise((resolve) => resolveListener = resolve),
    request: () => {
      requested = true;
      return Promise.resolve();
    },
  }, () => {});
  assert.equal(requested, false);
  resolveListener!(() => {});
  await capture.ready;
  assert.equal(requested, true);
  capture.cancel();
});

test("invocation failure releases the listener and ignores a late completion", async () => {
  const failure = new Error("service unavailable");
  const harness = captureHarness(() => Promise.reject(failure));
  const selected: string[][] = [];
  const capture = startShortcutCapture(harness.ports, (keys) => selected.push(keys));
  await assert.rejects(capture.ready, (error) => error === failure);
  assert.equal(harness.cleanupCount(), 1);
  harness.finish(["Win"]);
  assert.deepEqual(selected, []);
  capture.cancel();
  assert.equal(harness.cleanupCount(), 1);
});

test("cancel during listener setup releases it when ready without requesting input", async () => {
  let resolveListener: ((unlisten: () => void) => void) | undefined;
  let requested = false;
  let cleanupCount = 0;
  const capture = startShortcutCapture({
    listen: () => new Promise((resolve) => resolveListener = resolve),
    request: () => {
      requested = true;
      return Promise.resolve();
    },
  }, () => {});
  capture.cancel();
  resolveListener!(() => cleanupCount++);
  await capture.ready;
  assert.equal(requested, false);
  assert.equal(cleanupCount, 1);
});

test("listener setup failure never starts a backend capture", async () => {
  const failure = new Error("listener unavailable");
  let requested = false;
  const capture = startShortcutCapture({
    listen: () => Promise.reject(failure),
    request: () => {
      requested = true;
      return Promise.resolve();
    },
  }, () => {});
  await assert.rejects(capture.ready, (error) => error === failure);
  assert.equal(requested, false);
  capture.cancel();
});

test("replacing a capture prevents the old row from consuming the new result", async () => {
  const previous = captureHarness();
  const next = captureHarness();
  const previousKeys: string[][] = [];
  const nextKeys: string[][] = [];
  const firstCapture = startShortcutCapture(previous.ports, (keys) => previousKeys.push(keys));
  await firstCapture.ready;
  firstCapture.cancel();
  const nextCapture = startShortcutCapture(next.ports, (keys) => nextKeys.push(keys));
  await nextCapture.ready;
  previous.finish(["Win"]);
  next.finish(["Win"]);
  assert.deepEqual(previousKeys, []);
  assert.deepEqual(nextKeys, [["Win"]]);
  assert.equal(previous.cleanupCount(), 1);
  assert.equal(next.cleanupCount(), 1);
});

test("an unsupported single-key capture reports rejection, not a binding change", async () => {
  const harness = captureHarness();
  const selected: string[][] = [];
  let rejectionCount = 0;
  const capture = startShortcutCapture(harness.ports, (keys) => selected.push(keys), () => rejectionCount++);
  await capture.ready;
  harness.finish(["A"]);
  assert.deepEqual(selected, []);
  assert.equal(rejectionCount, 1);
  assert.equal(harness.cleanupCount(), 1);
});

test("capture events are unique and valid Tauri event names", () => {
  const events = Array.from({ length: 20 }, () => createShortcutCaptureEvent());
  assert.equal(new Set(events).size, events.length);
  for (const event of events) assert.match(event, /^shortcut-finished-\d+$/);
});

test("old-session cancellation cannot consume a newly registered capture", async () => {
  const previousEvent = createShortcutCaptureEvent();
  const nextEvent = createShortcutCaptureEvent();
  const listeners = new Map<string, (keys: ShortcutCaptureResult) => void>();
  const selected: string[][] = [];

  function portsFor(event: string): ShortcutCapturePorts {
    return {
      listen: (onFinished) => {
        listeners.set(event, onFinished);
        return Promise.resolve(() => {
          listeners.delete(event);
        });
      },
      request: () => {
        // Backend cancels the prior request after the new listener is ready.
        if (event === nextEvent) listeners.get(previousEvent)?.(null);
        return Promise.resolve();
      },
    };
  }

  const previousCapture = startShortcutCapture(portsFor(previousEvent), () => {});
  await previousCapture.ready;
  const nextCapture = startShortcutCapture(portsFor(nextEvent), (keys) => selected.push(keys));
  await nextCapture.ready;
  assert.equal(listeners.has(previousEvent), false);
  assert.equal(listeners.has(nextEvent), true);
  listeners.get(nextEvent)?.(["Win"]);
  assert.deepEqual(selected, [["Win"]]);
  previousCapture.cancel();
  nextCapture.cancel();
  assert.equal(listeners.size, 0);
});
