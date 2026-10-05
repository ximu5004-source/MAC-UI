# Full system tray

The popup displays every icon returned by the native tray bridge, including non-messaging
apps, hidden icons, and system icons. Messaging clients are sorted first, but are not a filter.
Only duplicate stable IDs are merged; multiple icons owned by one app remain separate.

## Layout and loading

- `autoSizeByContent` sizes the native popup. Never constrain this surface with `vh` / `dvh`:
  a height tied to the popup's own viewport creates a shrinking resize feedback loop.
- Limits come from the target monitor's work area, converted to logical pixels. Only the
  app list scrolls; the title, count, and refresh action remain visible.
- Rows never shrink to fit. Broken icon images have a fallback, and labels have native tooltips.
- Tray loading and optional Windows notifications initialize independently. Listeners are
  registered before fetching; a late refresh response cannot overwrite a newer tray event.
- Refresh re-reads the native bridge's current snapshot. It does not restart Explorer or apps.
- Loading, empty, failed-load, and failed-action states have localized feedback.

## Verification (2026-09-21, Windows debug app)

- Reproduced the old issue: accessibility exposed 10 app buttons, while the screenshot
  showed only the heading/description in a roughly 126 px popup.
- After the fix, the real popup displayed 11 current tray icons in a stable 600 px panel.
- Verified scrolling exposes the last item, refresh retains the list, and reopening does
  not shrink the popup. The changing count reflects the live Windows tray.
- Right-clicked HuionTablet and visually confirmed its native context menu. Did not select
  its Exit action or change any app settings.
- Four presentation regression tests plus two existing messaging tests passed.
- `npm run type-check` passed with zero Svelte errors/warnings; `git diff --check` passed.

Tests:

```powershell
node --import tsx --test src/ui/svelte/system-tray/trayPresentation.test.ts libs/ui/svelte/utils/communication.test.ts
npm run type-check
```

Specific applications can still differ in native single/double-click behavior. This popup
forwards the original Windows actions; it does not synthesize tray icons for processes that
never register one. No installer was rebuilt for this change.

## Stability follow-up (2026-09-27)

- Dock activation and tray clicks are now separate operations. Dock restores a real app
  window first. Only the known WeChat / Weixin / QQ / WeCom clients use tray double-click
  restoration; other apps fall back to their own executable/UMID entry point. Their tray
  icon's left click may legitimately open a menu and must not stand in for a Dock click.
- The hook selects the callback protocol only on `NIM_SETVERSION` (including version 0).
  A balloon's overlapping `uTimeout` field no longer changes click message packing.
  Partial icon updates preserve hidden state unless `NIF_STATE` masks `NIS_HIDDEN`.
- Negative multi-monitor coordinates are packed without sign-extending into the other word.
- Explorer's IPC callback copies the HICON and queues the event before acknowledgement.
  One bounded worker performs image decoding, file writes and frontend notifications in
  order; it destroys each copied icon after processing. On queue overflow it drains the
  burst and requests a native tray snapshot refresh instead of blocking Explorer.
- No-op icon updates do not repeatedly redraw all tray consumers. Image save failure does
  not panic the tray worker. Context menu handlers suppress WebView default menus and
  bubbling; Dock drag overlays are inert and consumed drag-clicks are ignored.

Regression coverage: hook protocol/visibility tests, signed-coordinate packing tests,
nonblocking FIFO/overflow recovery tests, Dock restore-routing tests, and pointer/keyboard
activation tests. Native third-party apps were not clicked by these automated tests.
