# Independent Desktop Manager

This surface maps user and public desktop entries without moving the source files.
Double-click opens the default Windows handler; F2 renames the selected item.
The context menu exposes open, reveal in Explorer, and rename operations.

## Category stacks

- Drag the title (not a file icon) to move a stack.
- Positions, expanded sizes, grid preference and collapsed categories are saved in the independent widget's `desktop-shell.json`.
- Show grid keeps the 20 px alignment grid visible without changing snapping or positions. This independent preference is available in the toolbar and background menu.
- When Show grid is off, the alignment grid still appears while dragging/resizing with snapping enabled. Disable Snap to grid for pixel-level placement.
- Drag any edge/corner to resize. The bottom-right handle is keyboard focusable; the adjustment menu also has width/height buttons.
- Icons reflow into columns as width changes; tall lists scroll within the chosen height.
- Saved coordinates are validated and clamped to the available primary-monitor area.
- The focused title supports arrow keys (20 px with snapping, 10 px without) and Shift for four times the step.
- The position button provides click-only movement controls (20 px).
- Collapse is separate from the drag handle. Long groups scroll internally.
- Reset layout clears custom positions/sizes and expands groups; it never moves files.
- Layout writes are serialized to prevent an older write from winning a rapid drag.

## New files

- Background right-click → New file includes Text and Markdown, even when Windows has no registered text template.
- Additional entries use installed Windows `ShellNew` templates (`NullFile`, `FileName`, binary `Data`, in that precedence).
- Command/COM/wizard-based types are omitted; no registry commands are executed and no placeholder Office/shortcut files are fabricated.
- Only the user's actual Desktop known folder is writable; public desktop contents are still displayed but never used as the creation destination.
- Creation is exclusive (`create_new`): duplicate names become `Name (2).ext`; existing files, folders and concurrent creations are not overwritten.
- The created item refreshes immediately and enters inline rename with only its stem selected. Enter confirms; Escape retains the original generated name. Collapsed stacks are expanded in memory so the field is reachable, without resetting saved positions or sizes.
- Reference: [Microsoft ShellNew documentation](https://learn.microsoft.com/en-us/windows/win32/shell/context#extending-the-new-submenu).

## Validation commands

Run from the repository root:

```powershell
node --import tsx --test src/ui/svelte/desktop-shell/modules/DesktopShell/stackLayout.test.ts src/ui/svelte/desktop-shell/modules/DesktopShell/newFiles.test.ts
cargo test -p slu-utils new_file --lib
npm run type-check
cargo check --workspace
```

Earlier combined-desktop smoke test, 2026-09-21, debug build:

- Dragged a stack away from its default position.
- Dragged past the bottom/right edges and confirmed the complete stack remained visible.
- Collapsed it, restarted the app, and confirmed both position and collapse state persisted.
- Created and opened a temporary folder, renamed it with F2, and observed the desktop update.
- Opened a temporary text file by double-click and verified its contents in Notepad.
- Removed only the temporary test file and empty test folder afterward.

Independent-desktop validation, 2026-09-21, debug build:

- Started with the built-in wallpaper widget disabled; the Windows wallpaper remained visible behind the independent desktop.
- Confirmed the old category positions/collapse state migrated and both settings/layout backups were created.
- Verified the default 260 px stacks retain two icon columns, including scrollable long groups.
- Used Windows Computer Use to drag a stack and verified the saved result aligned to the 20 px grid (`3060, 40`).
- Observed subsequent manual resizing writing custom widths/heights to the independent layout file.
- Stopped automated mouse input when concurrent user input was detected; did not reset the user's new layout or restart afterward.
- Six layout tests and three settings-migration tests passed, as did frontend type checking and the debug build.
- Post-resize restart persistence and coexistence with specific third-party wallpaper applications still need a separate interactive smoke test.

Do not run the reset smoke test over a user's custom layout without preserving it.

New-file/grid smoke test, 2026-09-22, debug build:

- Used Windows Computer Use to open the background menu, choose New file → Text with keyboard navigation, and rename the created item using Enter.
- Confirmed the actual Desktop contained the expected empty `.txt` file; removed only that temporary test file afterward.
- Confirmed the menu enumerated the installed ZIP template alongside Text and Markdown.
- Clicked Show grid on and off; observed the visible alignment grid and the persisted `showGrid` preference.
- Compared saved positions, sizes, collapsed categories, arrange mode and snapping before/after; all were unchanged.
- Fifteen frontend regression tests, four safe-file-creation tests, type checking and the debug build passed.

## Wallpaper independence and migration

The `@seelen/desktop-shell` widget owns the transparent interactive desktop window and Explorer icon visibility.
It has no wallpaper/player/accent-color subscriptions. Wallpaper rendering and its rotation loop are separate and
stop when `@seelen/wallpaper-manager` is disabled. Only native icon list windows are hidden, never WorkerW or DefView.

On first upgrade from the combined desktop, settings are backed up as `settings.before-independent-desktop.json`,
the desktop remains enabled and built-in wallpaper becomes disabled. An explicit independent-desktop setting is
never overwritten. The old widget's `desktop-shell.json` is backed up and copied to the new data directory only
if the destination is missing. Files and wallpaper collections are not moved/deleted.
