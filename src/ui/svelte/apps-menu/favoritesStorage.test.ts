import assert from "node:assert/strict";
import { test } from "node:test";
import { FAVORITES_FILE, loadFavoritesStorage } from "./favoritesStorage.ts";

const missing = () => ({ code: "not_found", message: "missing" });
const savedLayout = { version: 2, items: [{ type: "app", itemId: "my-custom!App" }] };

test("an existing v2 layout is loaded without reading migration sources or requesting a save", async () => {
  const calls: string[] = [];
  const result = await loadFavoritesStorage(async filename => {
    calls.push(filename);
    return JSON.stringify(savedLayout);
  }, []);
  assert.deepEqual(calls, [FAVORITES_FILE]);
  assert.deepEqual(result, { items: savedLayout.items, needsSave: false, migrated: false });
});

test("only proven NotFound can initialize empty favorites", async () => {
  const calls: string[] = [];
  const result = await loadFavoritesStorage(async filename => { calls.push(filename); throw missing(); }, []);
  assert.deepEqual(calls, [FAVORITES_FILE, "launchpad-v1.json", "favorites.json"]);
  assert.deepEqual(result, { items: [], needsSave: true, migrated: false });
});

test("the 3.0.1 string IPC supports missing file/path without matching localized message text", async () => {
  for (const error of [
    'Io(Os { code: 2, kind: NotFound, message: "系统找不到指定的文件。" })',
    'Io(Os { code: 3, kind: NotFound, message: "The system cannot find the path specified." })',
  ]) {
    const loaded = await loadFavoritesStorage(async () => { throw error; }, []);
    assert.deepEqual(loaded, { items: [], needsSave: true, migrated: false });
  }
});

test("v2 permission, sharing and unknown read errors stop migration and never request a replacement", async () => {
  for (const error of [
    { code: "permission_denied", message: "denied" },
    { code: "io_error", message: "sharing violation" },
    { code: "unavailable", message: "not_found in localized error text" },
    "Io(NotFound)", new Error("file not found"), undefined,
    'Io(Os { code: 5, kind: PermissionDenied, message: "NotFound" })',
    'Io(Os { code: 32, kind: Other, message: "sharing violation" })',
    'Io(Os { code: 2, kind: Other, message: "missing" })',
  ]) {
    const calls: string[] = [];
    let returned = false;
    try {
      await loadFavoritesStorage(async filename => { calls.push(filename); throw error; }, []);
      returned = true;
    } catch (actual) { assert.equal(actual, error); }
    assert.equal(returned, false);
    assert.deepEqual(calls, [FAVORITES_FILE]);
  }
});

test("unreadable legacy data is preserved instead of replaced with fresh empty favorites", async () => {
  const calls: string[] = [];
  const error = { code: "permission_denied" };
  await assert.rejects(loadFavoritesStorage(async filename => {
    calls.push(filename);
    if (filename === FAVORITES_FILE) throw missing();
    throw error;
  }, []), actual => actual === error);
  assert.deepEqual(calls, [FAVORITES_FILE, "launchpad-v1.json"]);
});

test("corrupt or unknown v2 formats remain untouched and do not fall back to old catalogs", async () => {
  for (const content of ["broken json", '{"version":3,"items":[]}', '{"version":2,"items":{}}']) {
    const calls: string[] = [];
    await assert.rejects(loadFavoritesStorage(async filename => { calls.push(filename); return content; }, []));
    assert.deepEqual(calls, [FAVORITES_FILE]);
  }
});

test("retry after a transient read failure loads the user's existing v2 unchanged", async () => {
  let attempt = 0;
  const read = async () => {
    if (++attempt === 1) throw { code: "io_error", message: "temporarily locked" };
    return JSON.stringify(savedLayout);
  };
  await assert.rejects(loadFavoritesStorage(read, []));
  assert.deepEqual(await loadFavoritesStorage(read, []), { items: savedLayout.items, needsSave: false, migrated: false });
});
