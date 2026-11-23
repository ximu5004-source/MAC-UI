import assert from "node:assert/strict";
import { test } from "node:test";
import { clearFavoritesSafely } from "./favoritesClear.ts";

function deferred() {
  let resolve!: () => void;
  let reject!: (error: Error) => void;
  const promise = new Promise<void>((done, fail) => { resolve = done; reject = fail; });
  return { promise, resolve, reject };
}

function fixture() {
  const backup = deferred(), write = deferred();
  let revision = 1;
  let items = ["original-app", "custom-folder"];
  let undo: string[] | null = null;
  let state = "saved";
  const written: string[][] = [];
  const operations = {
    getRevision: () => revision,
    backup: () => backup.promise,
    clear: () => {
      ++revision; items = []; undo = null; state = "saving";
      written.push([...items]);
      return { revision, saved: write.promise };
    },
    restore: (snapshot: string[]) => { ++revision; items = [...snapshot]; undo = [...snapshot]; state = "error"; },
    complete: (snapshot: string[]) => { undo = [...snapshot]; state = "saved"; },
  };
  return {
    backup, write, operations, written,
    snapshot: [...items],
    get items() { return items; },
    get undo() { return undo; },
    get state() { return state; },
    edit(next: string[]) { ++revision; items = [...next]; undo = null; state = "saved"; },
    retrySave() { written.push([...items]); state = "saved"; },
  };
}

test("failed clear restores original favorites so Retry never persists an accidental empty list", async () => {
  const f = fixture();
  const clearing = clearFavoritesSafely(f.snapshot, f.operations);
  f.backup.resolve();
  await Promise.resolve();
  assert.deepEqual(f.items, []);
  f.write.reject(new Error("disk full"));
  await assert.rejects(clearing, /disk full/);
  assert.deepEqual(f.items, f.snapshot);
  assert.deepEqual(f.undo, f.snapshot);
  assert.equal(f.state, "error");
  f.retrySave();
  assert.deepEqual(f.written, [[], f.snapshot]);
});

test("backup failure never submits a clear and retains the recovery selection", async () => {
  const f = fixture();
  const clearing = clearFavoritesSafely(f.snapshot, f.operations);
  f.backup.reject(new Error("backup denied"));
  await assert.rejects(clearing, /backup denied/);
  assert.deepEqual(f.written, []);
  assert.deepEqual(f.items, f.snapshot);
  assert.deepEqual(f.undo, f.snapshot);
});

test("user edits during backup supersede the clear without changing the newer layout", async () => {
  const f = fixture();
  const clearing = clearFavoritesSafely(f.snapshot, f.operations);
  f.edit(["new-user-selection"]);
  f.backup.resolve();
  await clearing;
  assert.deepEqual(f.items, ["new-user-selection"]);
  assert.deepEqual(f.written, []);
  assert.equal(f.undo, null);
});

test("late clear failure cannot roll back a newer edit or replace its saved status", async () => {
  const f = fixture();
  const clearing = clearFavoritesSafely(f.snapshot, f.operations);
  f.backup.resolve();
  await Promise.resolve();
  f.edit(["new-user-selection"]);
  f.write.reject(new Error("old write failed"));
  await assert.rejects(clearing, /old write failed/);
  assert.deepEqual(f.items, ["new-user-selection"]);
  assert.equal(f.undo, null);
  assert.equal(f.state, "saved");
});

test("late clear success cannot attach a stale undo snapshot to a newer edit", async () => {
  const f = fixture();
  const clearing = clearFavoritesSafely(f.snapshot, f.operations);
  f.backup.resolve();
  await Promise.resolve();
  f.edit(["new-user-selection"]);
  f.write.resolve();
  await clearing;
  assert.deepEqual(f.items, ["new-user-selection"]);
  assert.equal(f.undo, null);
});
