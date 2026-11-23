# Full system tray

The popup shows a compact icon grid with 20-pixel images, accessible names and hover labels.
It includes hidden, system and non-messaging icons. Task Manager registers multiple CPU meter
frames on one callback window. They share one tile, and clicks use the currently visible frame's
native ID. Other applications retain their distinct native icons.

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

## Restored-baseline fixes (2026-10-29)

- Native GUID and HWND/uID identities resolve partial updates to the original
  stable item. Distinct explicit GUIDs remain separate. The tray event worker
  retires invalid callback windows every two seconds, including while idle.
- Repeated icon pixel changes trigger a bounded visual attention pulse in the
  toolbar and popup. A new real toast/balloon can pulse separately. Clicking
  acknowledges the visual pulse across webviews; it does not mark chats read.
  Existing notification history and the first icon snapshot are not new messages.
- Empty native tooltip/balloon updates clear old hints instead of preserving
  stale unread metadata. Explicit zero/no-unread wording is not a notification.
- Explicit blank native icon frames also participate in visual-change detection;
  the toolbar retains the last application image instead of disappearing. A
  blank icon alone does not create message content or a notification count.
- Bitmap decoding follows GetDIBits' unselected-bitmap requirement and checks
  bounded allocation and full scanline reads. Failed reads never publish a hash.
- A tray-local worker reconnects only to the verified Windows Explorer tray
  thread after shell replacement, with finite retry delays and hook-before-DLL
  cleanup. Its process identity checks are local to this module; it has no
  dependency on installation ownership, startup handoff or desktop recovery.

Windows notification access may be unavailable or denied; native tray metadata
and pixel-change attention remain independent of that permission. If a client
publishes no toast, balloon, unread hint or changing native icon, MAC UI has no
reliable signal and does not invent message content or unread counts.

This patch has pure regression coverage. Actual WeChat/WeCom incoming messages,
Task Manager relaunch and Explorer restart still require native acceptance.
