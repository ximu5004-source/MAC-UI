# Launchpad interaction and layout

2026-10-30 changes, on the restored 3.0.1 baseline:

- The main toolbar always exposes **Clear Favorites**. A native modal first
  explains that only the personal Favorites/folder layout is cleared. Cancel
  receives initial focus. After a durable backup and successful save, the UI
  opens app selection; **Undo Clear** remains in the main toolbar until another
  edit. Backup/write failure keeps the previous arrangement and reports an
  error. All Apps, Windows Start Menu entries and installed software are never
  cleared by this action.
- The native window still covers the selected physical monitor in fullscreen.
  Its content is centered inside a maximum 1280 × 900 CSS-pixel composition,
  with responsive outside space. The light/dark neutral glass surface keeps
  labels legible over arbitrary live application backgrounds.
- Normal pages use 72 px native artwork, 16 px labels, at most 7 columns and
  4 rows, and non-stretching cell heights. Smaller windows retain 64/48 px
  geometry with the same two-line labels. A large catalog uses more pages,
  rather than expanding the grid across a 4K desktop.
- Clicking the blank grid or outside the central composition dismisses
  Launchpad; controls, folder/app items and open dialogs keep their normal
  interaction. Escape continues to back out of selection/search before hiding.

Pure tests cover layout capacity, clipping boundaries, safe clear ordering,
backup/write failure, concurrent edits, and the exposed confirmation controls.
`App.svelte` and `ClearFavoritesDialog.svelte` compile without warnings.
Native fullscreen, native blur, focus behavior and actual persisted user data
still require Windows acceptance; a browser fixture is visual QA only.

Visual fixture after the normal UI build:

```powershell
node scripts/qa/launchpad-server.mjs dist 3581
```

Open `http://127.0.0.1:3581/svelte/apps-menu/index.html` at 3834 × 2142,
1920 × 1080, and 1100 × 680. `?dark` exercises dark appearance. This fixture
uses its own mock application catalog and storage, not Windows or user files.
