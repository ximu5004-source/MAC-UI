import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import { batch, computed, signal } from "@preact/signals";
import { lazySignal } from "../../../../../libs/ui/react/utils/LazySignal.ts";

interface SettingsSnapshot {
  byWidget: { "@seelen/apps-menu": { enabled: boolean } };
}

function snapshot(enabled: boolean): SettingsSnapshot {
  return { byWidget: { "@seelen/apps-menu": { enabled } } };
}

test("settings subscribes to change events before starting the initial fetch", () => {
  const source = readFileSync(new URL("./mod.ts", import.meta.url), "utf8");
  assert.match(source, /settings = lazySignal\(\(\) => invoke\(SeelenCommand.StateGetSettings/);
  const subscribe = source.indexOf("await subscribe(SeelenEvent.StateSettingsChanged");
  const initialize = source.indexOf("await settings.init()");
  assert.ok(subscribe >= 0 && initialize > subscribe);
  assert.match(source.slice(subscribe, initialize), /settings\.value = payload;/);
});

test("a settings event during fetch wins over its stale enabled/disabled snapshot", async () => {
  for (const eventEnabled of [false, true]) {
    let resolveFetch: ((value: SettingsSnapshot) => void) | undefined;
    const settings = lazySignal(() => new Promise<SettingsSnapshot>((resolve) => resolveFetch = resolve));
    const initialSettings = signal("");
    const initializing = settings.init();
    const latest = snapshot(eventEnabled);

    // This is the subscribed StateSettingsChanged assignment, using the real
    // LazySignal rather than a separate test implementation of its race guard.
    batch(() => {
      settings.value = latest;
      initialSettings.value = JSON.stringify(latest);
    });
    const inputDisabled = computed(() => !settings.value.byWidget["@seelen/apps-menu"].enabled);
    assert.equal(inputDisabled.value, !eventEnabled);

    resolveFetch!(snapshot(!eventEnabled));
    await initializing;
    initialSettings.value = JSON.stringify(settings.value);

    assert.equal(settings.value, latest);
    assert.equal(inputDisabled.value, !eventEnabled);
    assert.equal(initialSettings.value, JSON.stringify(latest));
    assert.equal(initialSettings.value !== JSON.stringify(settings.value), false);
  }
});

test("an event before init is retained, while no-event initialization still uses its fetch", async () => {
  const settings = lazySignal(() => Promise.resolve(snapshot(false)));
  const latest = snapshot(true);
  settings.value = latest;
  await settings.init();
  assert.equal(settings.value, latest);

  const initial = snapshot(false);
  const noEvent = lazySignal(() => Promise.resolve(initial));
  await noEvent.init();
  assert.equal(noEvent.value, initial);
  assert.equal(!noEvent.value.byWidget["@seelen/apps-menu"].enabled, true);
});
