import assert from "node:assert/strict";
import { test } from "node:test";
import type { StartMenuItem } from "@seelen-ui/lib/types";
import { disbandFolder, importStartMenu, isApplicationItem, moveIntoFolder, moveRootItem, normalizeLayout, pageGeometry, pageSlice, removeApp, startMenuCatalog } from "./launchpad.ts";

const app = (name: string, folder: string[] = [], umid: string | null = null): StartMenuItem => ({
  path: `C:\\Programs\\${folder.join("\\")}\\${name}.lnk`, display_name: name,
  umid, target: null, toast_activator: null, start_menu_folder: folder,
});

test("imports root apps, package apps and nested Start Menu directories", () => {
  const result = importStartMenu([], [app("微信"), app("Paint", ["工具", "绘图"]), { ...app("Store", [], "pkg!Store"), path: "" }]);
  assert.equal(result.addedApps, 3);
  assert.equal(result.addedFolders, 1);
  const folder = result.items.find(i => i.type === "folder");
  assert.equal(folder?.name, "工具 / 绘图");
  assert.equal(folder?.itemIds.length, 1);
});

test("repeat import is idempotent and preserves user folder names, positions and memberships", () => {
  const a = app("One", ["Tools"]), b = app("Two", ["Tools"]);
  const initial = importStartMenu([], [a]);
  const folder = initial.items[0]!;
  assert.equal(folder.type, "folder");
  if (folder.type !== "folder") return;
  folder.name = "My tools";
  const twice = importStartMenu(initial.items, [a, b]);
  assert.equal(twice.addedApps, 1);
  assert.equal(twice.addedFolders, 0);
  assert.equal(twice.items[0]?.type === "folder" && twice.items[0].name, "My tools");
  assert.deepEqual(importStartMenu(twice.items, [a, b]).items, twice.items);
  assert.equal(folder.itemIds.length, 1, "must not mutate previous state");
});

test("package identities are deduplicated but distinct launch shortcuts are preserved", () => {
  assert.equal(startMenuCatalog([app("One", [], "APP!A"), app("Alias", [], "app!a"), app("Help"), { ...app("desktop"), path: "C:\\desktop.ini" }]).length, 2);
});

test("search and import agree on launchable shortcut types", () => {
  for (const extension of ["lnk", "EXE", "url", "appref-ms"]) {
    assert.equal(isApplicationItem({ path: `C:\\Fixture\\App.${extension}`, umid: null }), true);
  }
  assert.equal(isApplicationItem({ path: "", umid: "Package!App" }), true);
  for (const extension of ["png", "pdf", "txt", "ini"]) {
    assert.equal(isApplicationItem({ path: `C:\\Fixture\\File.${extension}`, umid: null }), false);
  }
});

test("moving or removing an app never deletes a folder's remaining sibling", () => {
  const folder = { type: "folder" as const, itemId: "f", name: "Tools", itemIds: ["a", "b"] };
  const other = { type: "folder" as const, itemId: "g", name: "More", itemIds: [] };
  assert.deepEqual(removeApp([folder], "a"), [{ ...folder, itemIds: ["b"] }]);
  assert.deepEqual(moveIntoFolder([folder, other], "a", "g"), [{ ...folder, itemIds: ["b"] }, { ...other, itemIds: ["a"] }]);
  assert.deepEqual(disbandFolder([folder], "f"), [{ type: "app", itemId: "a" }, { type: "app", itemId: "b" }]);
});

test("normalization retains single/empty folders and rejects corrupt and duplicate items", () => {
  assert.deepEqual(normalizeLayout([null, { type: "app", itemId: 1 }, { type: "folder", itemId: "f", name: "F", itemIds: ["a", "A", null] }, { type: "app", itemId: "a" }]), [{ type: "folder", itemId: "f", name: "F", itemIds: ["a"] }]);
  assert.equal(normalizeLayout([{ type: "folder", itemId: "f", itemIds: [] }]).length, 1);
});

test("responsive pages stay bounded and page movement keeps all applications", () => {
  const items = Array.from({ length: 75 }, (_, i) => ({ type: "app" as const, itemId: String(i) }));
  assert.equal(pageGeometry(1920, 1080).pageSize, 40);
  assert.equal(pageGeometry(375, 500).columns, 3);
  assert.equal(pageSlice(items, 99, 35).page, 2);
  assert.equal(pageSlice([], 9, 0).count, 1);
  assert.deepEqual(moveRootItem(items, "0", 35)[35], items[0]);
  assert.equal(moveRootItem(items, "74", 35).length, 75);
});

test("macOS-style pages reserve full native icons and two-line labels after page controls", () => {
  assert.deepEqual(pageGeometry(1028, 740), { columns: 8, rows: 5, pageSize: 40, iconSize: 64 });
  assert.deepEqual(pageGeometry(1028, 432), { columns: 8, rows: 2, pageSize: 16, iconSize: 64 });
  assert.deepEqual(pageGeometry(500, 240), { columns: 4, rows: 1, pageSize: 4, iconSize: 48 });
  assert.deepEqual(pageGeometry(300, 160), { columns: 2, rows: 1, pageSize: 2, iconSize: 48 });
  assert.equal(pageGeometry(1028, 447).rows, 2, "do not add a clipped third large-icon row");
  assert.equal(pageGeometry(1028, 448).rows, 3);
  assert.equal(pageGeometry(500, 275).rows, 1, "compact cells retain 14px two-line labels");
  assert.equal(pageGeometry(500, 276).rows, 2);
});

test("compact geometry keeps narrow, short and invalid measurements bounded", () => {
  for (const [width, height] of [[0, 0], [1, 1], [NaN, Infinity], [-10, -5]]) {
    const layout = pageGeometry(width!, height!);
    assert.ok(Number.isFinite(layout.pageSize));
    assert.ok(layout.columns >= 1 && layout.columns <= 8);
    assert.ok(layout.rows >= 1 && layout.rows <= 5);
  }
});

test("calculated page cells cannot clip icons or labels on supported measured viewports", () => {
  for (const width of [300, 375, 576, 640, 1028, 1440]) {
    for (const height of [200, 276, 360, 432, 448, 600, 740, 1080]) {
      const geometry = pageGeometry(width, height);
      const compact = geometry.iconSize === 48;
      const columnGap = compact ? 16 : 24, rowGap = compact ? 12 : 16;
      const cellWidth = compact ? 88 : 104, cellHeight = geometry.iconSize + 56;
      assert.ok(geometry.columns * cellWidth + (geometry.columns - 1) * columnGap + 16 <= width);
      assert.ok(geometry.rows * cellHeight + (geometry.rows - 1) * rowGap + 56 <= height);
    }
  }
});

test("folder sheets keep an independent full-size icon grid on compact root pages", () => {
  const root = pageGeometry(500, 260);
  const sheet = pageGeometry(440, 360, 64);
  assert.equal(root.iconSize, 48);
  assert.equal(sheet.iconSize, 64);
  assert.deepEqual(sheet, { columns: 3, rows: 2, pageSize: 6, iconSize: 64 });
});
