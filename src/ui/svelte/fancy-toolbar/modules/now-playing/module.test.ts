import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { load } from "js-yaml";
import { NOW_PLAYING_PLUGIN_ID } from "./module.ts";

const source = (path: string) => readFileSync(new URL(path, import.meta.url), "utf8");

test("music has its own discoverable, localized module, separate from volume", () => {
  const plugin = load(source("../../../../../static/plugins/tb_now_playing/metadata.yml")) as any;
  assert.equal(plugin.id, NOW_PLAYING_PLUGIN_ID);
  assert.notEqual(plugin.id, "@macui/tb-media-popup");
  assert.equal(plugin.target, "@macui/fancy-toolbar");
  assert.equal(plugin.metadata.displayName["zh-CN"], "音乐播放");
  assert.equal(plugin.metadata.displayName.en, "Music Playback");
  assert.deepEqual(plugin.plugin.scopes, []);
  assert.match(plugin.plugin.onClick, /view: "now-playing"/);
});

test("music is a normal sortable module, not an unconditional group accessory", () => {
  assert.doesNotMatch(source("../../components/ItemsGroup.svelte"), /NowPlaying/);
  const item = source("../../components/Item.svelte");
  assert.match(item, /pluginId === NOW_PLAYING_PLUGIN_ID/);
  assert.match(item, /<NowPlaying[^>]+\{sortable\}[^>]+oncontextmenu=\{handleContextMenu\}/);
  assert.match(source("../../components/Toolbar.svelte"), /pluginId=\{overlay\.pluginId\}/);
});

test("music keeps keyboard, drag protection and the dedicated popup view", () => {
  const component = source("./NowPlaying.svelte");
  assert.match(component, /<button/);
  assert.match(component, /\{@attach sortable\?\.attach/);
  assert.match(component, /isPrimaryToolbarClick\(event, sortable\?\.isDragging\)/);
  assert.match(component, /customArgs:\s*\{\s*view: "now-playing",/);
  assert.match(component, /__macuiPopupToggleSource:/);
  assert.match(component, /onMount\(\(\) => nowPlayingState\.retain\(\)\)/);
});

test("new and restored layouts include music without reinserting it into saved layouts", () => {
  const front = source("../../state/items.svelte.ts");
  assert.equal((front.match(/NOW_PLAYING_PLUGIN_ID,/g) ?? []).length, 1);
  assert.match(front, /right: \[\s*NOW_PLAYING_PLUGIN_ID,/);
  const back = source("../../../../../background/state/application/toolbar_items.rs");
  assert.match(back, /right: vec!\[\s*ToolbarItem2::Plugin\("@macui\/tb-now-playing"/);
  assert.doesNotMatch(back.slice(0, back.indexOf("fn initial_items")), /tb-now-playing/);
});

test("session subscription follows mount lifetime with stale async work guarded", () => {
  const state = source("./state.svelte.ts");
  assert.doesNotMatch(state, /\nawait subscribe/);
  assert.ok(state.indexOf("await subscribe") < state.indexOf("await value.init()"));
  assert.match(state, /if \(version !== generation\) \{\s*release\(\);/);
  assert.match(state, /if \(--consumers === 0\) \{\s*generation\+\+;\s*unlisten\?\.\(\)/);
  // isInitialized is not a rune; the initial empty render must track a reactive readiness flag.
  assert.match(state, /let ready = \$state\(false\)/);
  assert.match(state, /selectNowPlayingSession\(ready && sessions \? sessions\.value : \[\]\)/);
});

test("locking reordering never declares module controls disabled", () => {
  assert.match(
    source("../../components/SortableItem.svelte"),
    /draggable: false, droppable: toolbarState\.isReorderDisabled/,
  );
  assert.match(
    source("../../components/Toolbar.svelte"),
    /createDragDropManager\(\(\) => !toolbarState\.isReorderDisabled\)/,
  );
  assert.match(source("../../../../../../libs/ui/dnd.ts"), /preventActivation: \(\) => !isDragAllowed\(\)/);
  assert.match(source("../../components/Toolbar.svelte"), /<DragDropProvider\s+\{manager\}\s+\{sensors\}/);
});
