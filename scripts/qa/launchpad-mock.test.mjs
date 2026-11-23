import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import vm from "node:vm";

const source = await readFile(new URL("./launchpad-mock.js", import.meta.url), "utf8");

function fixture({ files = new Map(), search = "", failRead, failWrite } = {}) {
  const status = { textContent: "" };
  const scope = {
    window: {}, location: { search }, innerWidth: 3834, innerHeight: 2142,
    URLSearchParams, structuredClone,
    document: { getElementById: () => status, createElement: () => ({ style: {} }), body: { append() {} } },
    requestAnimationFrame() {},
    fetch: async (address, options = {}) => {
      const filename = new URL(address, "http://fixture.invalid").searchParams.get("key");
      if (options.method === "POST") {
        if (failWrite) return new Response("Save unavailable", { status: failWrite });
        files.set(filename, options.body);
        return new Response("ok");
      }
      if (failRead) return new Response("Read unavailable", { status: failRead });
      return files.has(filename) ? new Response(files.get(filename)) : new Response("Missing", { status: 404 });
    },
  };
  vm.runInNewContext(source, scope, { filename: "launchpad-mock.js" });
  return { invoke: scope.window.__TAURI_INTERNALS__.invoke, widget: scope.window.__MACUI_WIDGET_INSTANCE, files };
}

test("fresh fixture seeds a separate usable favorites layout, focused window and cursor", async () => {
  const qa = fixture();
  const catalog = await qa.invoke("get_start_menu_items");
  const favorites = JSON.parse(await qa.invoke("read_data_file", { filename: "launchpad-favorites-v2.json" }));
  assert.equal(favorites.version, 2);
  assert.ok(favorites.items.length > 28);
  const identities = new Set(catalog.map(item => item.path.toLowerCase()));
  for (const item of favorites.items) for (const id of item.type === "folder" ? item.itemIds : [item.itemId]) assert.ok(identities.has(id));
  assert.ok(favorites.items.some(item => item.type === "folder"));
  const focused = await qa.invoke("get_focused_app");
  assert.equal(focused.hwnd, qa.widget.windowId);
  assert.equal(await qa.widget.window.isFocused(), true);
  const cursor = await qa.invoke("get_mouse_position");
  assert.equal(cursor.x, 1917);
  assert.equal(cursor.y, 1071);
  const [monitor] = await qa.invoke("get_connected_monitors");
  assert.equal(focused.monitor, monitor.id);
  assert.equal(monitor.rect.right, 3834);
});

test("clear writes survive fixture reload and do not repopulate favorites from the catalog", async () => {
  const files = new Map();
  const first = fixture({ files });
  await first.invoke("read_data_file", { filename: "launchpad-favorites-v2.json" });
  const empty = JSON.stringify({ version: 2, items: [] });
  await first.invoke("write_data_file", { filename: "launchpad-favorites-v2.json", content: empty });
  const reloaded = fixture({ files });
  assert.equal(await reloaded.invoke("read_data_file", { filename: "launchpad-favorites-v2.json" }), empty);
  assert.ok((await reloaded.invoke("get_start_menu_items")).length > 28);
});

test("optional files have explicit not-found errors but ambiguous read failures remain failures", async () => {
  await assert.rejects(fixture().invoke("read_data_file", { filename: "launchpad-v1.json" }), error => error?.code === "not_found");
  await assert.rejects(fixture({ failRead: 403 }).invoke("read_data_file", { filename: "launchpad-favorites-v2.json" }), /Fixture read failed \(403\)/);
  await assert.rejects(fixture({ failWrite: 500 }).invoke("read_data_file", { filename: "launchpad-favorites-v2.json" }), /Fixture save failed \(500\)/);
});

test("explicit empty-first-run fixture keeps its own empty personal list", async () => {
  const qa = fixture({ search: "?empty-favorites" });
  const favorites = JSON.parse(await qa.invoke("read_data_file", { filename: "launchpad-favorites-v2.json" }));
  assert.deepEqual(favorites.items, []);
});
