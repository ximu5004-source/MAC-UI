import assert from "node:assert/strict";
import { test } from "node:test";
import { createInputKeyDownHandler, navigateInDirection, pageShortcutStep, reconcilePageSelection, type SelectionScope } from "./keyboard-navigation.ts";
import { getItemId, pageSlice } from "./launchpad.ts";

const key = (value: string, overrides: Partial<Parameters<typeof pageShortcutStep>[0]> = {}) => ({
  key: value, ctrlKey: false, repeat: false, isComposing: false, defaultPrevented: false, ...overrides,
});

test("PageUp/Down and Ctrl+arrows issue exactly one page step per press", () => {
  for (const [event, expected] of [
    [key("PageDown"), 1], [key("PageUp"), -1],
    [key("ArrowRight", { ctrlKey: true }), 1], [key("ArrowLeft", { ctrlKey: true }), -1],
  ] as const) {
    assert.equal(pageShortcutStep(event), expected);
    assert.equal(pageShortcutStep({ ...event, repeat: true }), 0);
    assert.equal(pageShortcutStep(event), expected, "a fresh second press is never time-debounced");
  }
});

test("ordinary arrows, including held arrows, remain available for icon navigation", () => {
  for (const value of ["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", "Enter", "Escape"]) {
    assert.equal(pageShortcutStep(key(value)), null);
    assert.equal(pageShortcutStep(key(value, { repeat: true })), null);
  }
});

test("composing and already-handled key events never request page navigation", () => {
  for (const value of ["PageUp", "PageDown", "ArrowLeft", "ArrowRight"]) {
    assert.equal(pageShortcutStep(key(value, { ctrlKey: true, isComposing: true })), null);
    assert.equal(pageShortcutStep(key(value, { ctrlKey: true, defaultPrevented: true })), null);
    assert.equal(pageShortcutStep(key(value, { ctrlKey: true, repeat: true, isComposing: true })), null);
  }
});

test("a held shortcut with many OS repeats cannot skip intermediate pages", () => {
  const events = [key("ArrowRight", { ctrlKey: true }), ...Array.from({ length: 20 }, () => key("ArrowRight", { ctrlKey: true, repeat: true }))];
  const finalPage = events.reduce((page, event) => page + (pageShortcutStep(event) ?? 0), 0);
  assert.equal(finalPage, 1);
});

function inputKeyFixture(value: string, overrides: Partial<KeyboardEvent> = {}) {
  const calls = { prevented: 0, stopped: 0 };
  const input = { value: "web: fixture" } as HTMLInputElement;
  const event = {
    ...key(value), currentTarget: input,
    preventDefault: () => { calls.prevented++; },
    stopPropagation: () => { calls.stopped++; },
    ...overrides,
  } as unknown as KeyboardEvent;
  return { event, input, calls };
}

test("IME Enter confirms composition without launching web search, then a fresh Enter launches once", () => {
  let submitted = 0;
  const handler = createInputKeyDownHandler({ onEnter: (_event, input) => {
    assert.equal(input.value, "web: fixture");
    submitted++;
    return true;
  } });
  const composing = inputKeyFixture("Enter", { isComposing: true });
  handler(composing.event);
  assert.equal(submitted, 0);
  assert.deepEqual(composing.calls, { prevented: 0, stopped: 0 });
  const committed = inputKeyFixture("Enter");
  handler(committed.event);
  assert.equal(submitted, 1);
  assert.deepEqual(committed.calls, { prevented: 1, stopped: 1 });
});

test("an already-handled search Enter is not submitted or consumed again", () => {
  let submitted = 0;
  const handler = createInputKeyDownHandler({ onEnter: () => { submitted++; return true; } });
  const fixture = inputKeyFixture("Enter", { defaultPrevented: true });
  handler(fixture.event);
  assert.equal(submitted, 0);
  assert.deepEqual(fixture.calls, { prevented: 0, stopped: 0 });
});

test("search text-editing arrows stay local while Ctrl+arrows and non-web Enter keep bubbling", () => {
  const handler = createInputKeyDownHandler({ onEnter: () => false });
  for (const value of ["ArrowLeft", "ArrowRight"]) {
    const editing = inputKeyFixture(value);
    handler(editing.event);
    assert.deepEqual(editing.calls, { prevented: 0, stopped: 1 });
    const paging = inputKeyFixture(value, { ctrlKey: true });
    handler(paging.event);
    assert.deepEqual(paging.calls, { prevented: 0, stopped: 0 });
  }
  const entering = inputKeyFixture("Enter");
  handler(entering.event);
  assert.deepEqual(entering.calls, { prevented: 0, stopped: 0 });
});

test("page selection keeps a visible identity, replaces stale selection and permits an intentional null", () => {
  assert.equal(reconcilePageSelection("beta", ["alpha", "beta"]), "beta");
  assert.equal(reconcilePageSelection("gone", ["alpha", "beta"]), "alpha");
  assert.equal(reconcilePageSelection("gone", []), null);
  assert.equal(reconcilePageSelection(null, ["alpha", "beta"]), null);
  assert.equal(reconcilePageSelection(null, []), null);
});

test("resizing and clamped page changes never leave a selected identity outside the displayed page", () => {
  const items = ["alpha", "beta", "gamma", "delta", "epsilon", "zeta"];
  let selected: string | null = "epsilon";
  let pagination = pageSlice(items, 1, 3);
  selected = reconcilePageSelection(selected, pagination.items);
  assert.equal(selected, "epsilon", "capacity changes preserve an identity if it remains on-page");
  pagination = pageSlice(items, 1, 2);
  selected = reconcilePageSelection(selected, pagination.items);
  assert.equal(selected, "gamma", "shrinking capacity chooses the new page's safe first item");
  pagination = pageSlice(items.slice(0, 2), 2, 2);
  assert.equal(pagination.page, 0);
  selected = reconcilePageSelection(selected, pagination.items);
  assert.equal(selected, "alpha", "removing the last page also repairs selection after clamping");
  assert.equal(reconcilePageSelection(selected, pageSlice([], 0, 2).items), null);
});

test("selection uses the canonical renderer identities for legacy shortcut paths and package apps", () => {
  const shortcut = getItemId({ path: "C:\\Programs\\Fixture.LNK", umid: null });
  const packaged = getItemId({ path: "", umid: "Fixture.Package!App" });
  const folder = "folder:fixture";
  const visibleIds = [shortcut, packaged, folder];
  assert.equal(reconcilePageSelection("c:\\programs\\fixture.lnk", visibleIds), shortcut);
  assert.equal(reconcilePageSelection("Fixture.Package!App", visibleIds), packaged);
  assert.equal(reconcilePageSelection(folder, visibleIds), folder);
});

// Tiny synthetic options exercise the focus/selection contract without opening
// real windows, adding a DOM dependency, or invoking any application launcher.
function gridFixture() {
  let selected: string | null = "alpha";
  const searchInput = { kind: "search" };
  const doc = { activeElement: searchInput as unknown };
  const focusCalls: string[] = [];
  const options = [
    { id: "alpha", left: 0, top: 0 },
    { id: "beta", left: 100, top: 0 },
    { id: "folder", left: 0, top: 100 },
  ].map(({ id, left, top }) => ({
    dataset: { itemId: id },
    ownerDocument: doc,
    getBoundingClientRect: () => ({ left, top }),
    contains: () => false,
    focus: (settings: FocusOptions) => {
      assert.equal(settings.preventScroll, true);
      doc.activeElement = options.find(option => option.dataset.itemId === id);
      focusCalls.push(id);
    },
  }));
  const scope: SelectionScope = {
    container: () => ({ querySelectorAll: () => options }) as unknown as ParentNode,
    getSelected: () => selected,
    setSelected: value => { selected = value; },
  };
  return { scope, options, doc, searchInput, focusCalls, get selected() { return selected; } };
}

test("grid arrows move DOM focus with selection, so Enter targets the highlighted option", () => {
  const fixture = gridFixture();
  fixture.doc.activeElement = fixture.options[0];
  navigateInDirection("right", fixture.scope);
  assert.equal(fixture.selected, "beta");
  assert.equal(fixture.doc.activeElement, fixture.options[1]);
  navigateInDirection("left", fixture.scope);
  navigateInDirection("down", fixture.scope);
  assert.equal(fixture.selected, "folder");
  assert.equal(fixture.doc.activeElement, fixture.options[2]);
  assert.deepEqual(fixture.focusCalls, ["beta", "alpha", "folder"]);
});

test("search arrows preselect without stealing search focus or interrupting typing", () => {
  const fixture = gridFixture();
  navigateInDirection("right", fixture.scope);
  assert.equal(fixture.selected, "beta");
  assert.equal(fixture.doc.activeElement, fixture.searchInput);
  assert.deepEqual(fixture.focusCalls, []);
});

test("initial selection follows grid focus but remains selection-only from search", () => {
  for (const hasGridFocus of [false, true]) {
    const fixture = gridFixture();
    fixture.scope.setSelected(null);
    if (hasGridFocus) fixture.doc.activeElement = fixture.options[1];
    navigateInDirection("down", fixture.scope);
    assert.equal(fixture.selected, "alpha");
    assert.equal(fixture.doc.activeElement, hasGridFocus ? fixture.options[0] : fixture.searchInput);
    assert.deepEqual(fixture.focusCalls, hasGridFocus ? ["alpha"] : []);
  }
});

test("an arrow at a grid boundary does not disturb the current focus", () => {
  const fixture = gridFixture();
  fixture.doc.activeElement = fixture.options[0];
  navigateInDirection("left", fixture.scope);
  assert.equal(fixture.selected, "alpha");
  assert.equal(fixture.doc.activeElement, fixture.options[0]);
  assert.deepEqual(fixture.focusCalls, []);
});

test("page reconciliation only changes selection, without stealing search input focus", () => {
  const fixture = gridFixture();
  fixture.scope.setSelected("off-page");
  const visibleIds = fixture.options.map(option => option.dataset.itemId);
  fixture.scope.setSelected(reconcilePageSelection(fixture.selected, visibleIds));
  assert.equal(fixture.selected, "alpha");
  assert.equal(fixture.doc.activeElement, fixture.searchInput);
  assert.deepEqual(fixture.focusCalls, []);
  navigateInDirection("right", fixture.scope);
  assert.equal(fixture.selected, "beta");
  assert.equal(fixture.doc.activeElement, fixture.searchInput);
});

test("an arrow safely recovers stale selection before the reactive page reconciliation runs", () => {
  for (const hasGridFocus of [false, true]) {
    const fixture = gridFixture();
    fixture.scope.setSelected("off-page");
    if (hasGridFocus) fixture.doc.activeElement = fixture.options[1];
    navigateInDirection("down", fixture.scope);
    assert.equal(fixture.selected, "alpha");
    assert.equal(fixture.doc.activeElement, hasGridFocus ? fixture.options[0] : fixture.searchInput);
    assert.deepEqual(fixture.focusCalls, hasGridFocus ? ["alpha"] : []);
  }
});
