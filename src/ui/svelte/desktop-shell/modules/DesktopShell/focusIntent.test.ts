import assert from "node:assert/strict";
import test from "node:test";
import { DesktopFocusGuard, isDesktopInteractionWindow } from "./focusIntent.ts";

test("file completion can focus only the same still-focused interaction", () => {
  const guard = new DesktopFocusGuard();
  const create = guard.begin();
  assert.equal(guard.canFocus(create, true), true);
  assert.equal(guard.canFocus(create, false), false);
  guard.blur();
  assert.equal(guard.isCurrent(create), true, "the file result may still update its own editor");
  assert.equal(guard.canFocus(create, true), false, "returning later must not revive focus permission");
});

test("a menu action may edit after natural focus return, without native activation", () => {
  const guard = new DesktopFocusGuard();
  const menu = guard.begin();
  guard.blur();
  const rename = guard.resumeMenu(menu, menu.operation)!;
  assert.ok(rename);
  assert.equal(guard.canFocus(rename, false), false);
  assert.equal(guard.canFocus(rename, true), true);
  assert.equal(guard.resumeMenu(menu, menu.operation), null, "a menu callback is consumed only once");
});

test("external focus or a newer interaction rejects delayed menu callbacks", () => {
  const guard = new DesktopFocusGuard();
  const menu = guard.begin();
  guard.blur();
  guard.invalidate();
  assert.equal(guard.resumeMenu(menu, menu.operation), null);
  const rename = guard.begin();
  guard.begin();
  assert.equal(guard.canFocus(rename, true), false);
});

test("a delayed callback from an older menu cannot consume the newest menu intent", () => {
  const guard = new DesktopFocusGuard();
  const oldMenu = guard.begin();
  const currentMenu = guard.begin();
  assert.equal(guard.resumeMenu(currentMenu, oldMenu.operation), null);
  assert.equal(guard.resumeMenu(currentMenu, undefined), null);
  assert.equal(guard.resumeMenu(currentMenu, String(currentMenu.operation)), null);
  assert.ok(guard.resumeMenu(currentMenu, currentMenu.operation));
});

test("stale rename failures and unmounted components cannot restore focus", () => {
  const guard = new DesktopFocusGuard();
  guard.begin();
  const failedCommit = guard.capture();
  const newerRename = guard.begin();
  assert.equal(guard.canFocus(failedCommit, true), false);
  guard.dispose();
  assert.equal(guard.isCurrent(newerRename), false);
  assert.equal(guard.canFocus(guard.begin(), true), false);
});

test("only the desktop and its observed owned menu chain retain the interaction", () => {
  const menus = new Set<number>();
  assert.equal(isDesktopInteractionWindow({ hwnd: 10, ownerHwnd: 0 }, 10, menus), true);
  assert.equal(isDesktopInteractionWindow({ hwnd: 20, ownerHwnd: 10 }, 10, menus), true);
  assert.equal(isDesktopInteractionWindow({ hwnd: 30, ownerHwnd: 20 }, 10, menus), true);
  assert.equal(isDesktopInteractionWindow({ hwnd: 20, ownerHwnd: 10 }, 10, menus), true);
  assert.equal(isDesktopInteractionWindow({ hwnd: 40, ownerHwnd: 0 }, 10, menus), false);
  menus.clear();
  assert.equal(isDesktopInteractionWindow({ hwnd: 30, ownerHwnd: 20 }, 10, menus), false);
  assert.equal(isDesktopInteractionWindow({ hwnd: 0, ownerHwnd: 0 }, 10, menus), false);
});
