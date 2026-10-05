# Classic Genie geometry and isolated API review — 2026-10-04

Status: **geometry measured; ordinary application minimization not accepted**.
The production experimental gate remains unchanged and disabled by default. The
user has accepted a Dock-only first implementation; that does not authorize a
global animation preference, DLL injection, or broad native-window experiments.

## Classic deformation, not affine scale

`src/background/widgets/weg/genie/geometry.rs` now uses two distinct phases over
the existing 420 ms renderer duration:

- 0–42%: the far boundary remains at the source window. The near boundary stretches
  to the Dock destination, and strip width/center vary smoothly along the axis to
  form a curved funnel.
- 42–100%: the near boundary stays anchored to the Dock while the far boundary
  travels into it. The remaining image is swallowed rather than uniformly scaled.

All four directions share the same smoothstep phase boundary. Seven focused Rust
tests passed: identity/end destination; a non-affine neck; four-direction positive
and contiguous strips; direction selection; source far-edge anchoring and visible
stretch; Dock near-edge anchoring throughout swallowing; phase continuity and
out-of-range time clamping. The continuity tests also use negative source monitor
coordinates. These are algorithm assertions, **not end-user animation acceptance**.

```powershell
cargo test -p seelen-ui --bin seelen-ui genie::geometry -- --nocapture
```

## Reproducible own-art preview

The isolated preview uses Rust-generated data from the **same geometry function**,
not a separate JavaScript approximation. It renders 256 strips of a program-drawn
sample window, not a screenshot of an application or desktop. It exposes four
directions, progress seeking, static 42%/71% checkpoints, one-shot 420 ms playback,
and explicit 4× slow playback for inspection. It does not autoplay and respects
`prefers-reduced-motion`. Neither server nor page has a Tauri/native bridge.

```powershell
cargo test -p seelen-ui --bin seelen-ui export_owned_genie_geometry_fixture -- --ignored --nocapture --test-threads=1
node scripts/qa/genie-preview.mjs 3587
node --test scripts/qa/genie-preview.test.mjs
```

Open `http://127.0.0.1:3587/`. Generated JSON stays in
`target/qa/genie-geometry.json`. The loopback server serves only this JSON and its
fixed local HTML; it has no arbitrary file route or user-data lookup.

Measured: export test **1 passed**; `node --check scripts/qa/genie-preview.mjs`
passed; three Node syntax/generated-data tests passed, checking finite bounded
crops and the four-direction phase anchoring. In a fresh checkout without generated
JSON, the two data tests explicitly skip until the exporter is run; module syntax
still checks. This agent's URL-selected Edge extension did not respond to the
browser connection; no successful tab creation/navigation was performed. Browser
visual checkpoints remain to be verified by the root agent/user's working session.
Root's working in-app browser subsequently verified the same Rust-generated
preview: the 42% funnel, 71% anchored swallowing, up/down/left/right layouts,
100% endpoint, and 420 ms playback interrupted at 70% by Stop. Full-page captures
were required because the canvas extends below a 720 px viewport. The single
preview remained stopped after interruption; it does not autoplay. OS-level
reduced-motion switching was not performed and remains a manual check.

Seeing a correct synthetic funnel does not prove native DWM duplicate suppression,
application capture compatibility, restoration, cancellation, or foreground focus.

## Hide → minimize state-only probe

A newly created `cfg(test)`/ignored probe operates solely on its own synthetic
off-screen taskbar-capable Win32 window and an exact self-spawned copy of its test
binary. It tests both `ShowWindow` and `ShowWindowAsync` with this sequence:

1. Save visible/noniconic state, normal placement, and current foreground.
2. `SW_HIDE`; confirm nonvisible/noniconic. Cancel with `SW_SHOWNA`; confirm original
   visibility and normal rectangle without foreground activation.
3. `SW_HIDE`; confirm hidden; `SW_SHOWMINNOACTIVE`; confirm visible/iconic.
4. Observe those flags for 420 ms. Restore with `SW_SHOWNOACTIVATE`; confirm normal
   visible state, unchanged normal rectangle and foreground. Repeat three cycles.
5. Request helper cleanup and assert its HWND is destroyed. Its owning thread has
   an independent 12-second cleanup deadline; the parent may terminate only that
   exact spawned helper if normal cleanup fails.

```powershell
cargo test -p seelen-ui --bin seelen-ui owned_hide_minimize_probe -- --ignored --nocapture --test-threads=1
```

Measured on this host: **1 passed**. All same-process/cross-process and synchronous/
asynchronous cases reached the required states; normal rectangle was preserved
and foreground remained unchanged. Cross-process synchronous show operations took
about 13–19 ms; asynchronous submissions returned in about 4–156 µs and hidden-to-
iconic confirmation took about 4.4–9.0 ms in this run. The helper observed explicit
show/hide, size and window-position messages, with zero activation messages.

**Limits:** this probe is off-screen and records state flags, not compositor pixels.
`IsIconic` does not prove that DWM's animated source representation has disappeared.
It does not claim visual de-duplication, preservation of a maximized source, or
behaviour of any ordinary third-party application. No real application was hidden,
captured, minimized, restored, or otherwise changed.

The [ShowWindow documentation](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-showwindow)
specifies that `SW_HIDE` activates another window and `SW_SHOWMINNOACTIVE` does not
activate the minimized one. This introduces a real visibility transition, unlike
an atomic custom minimize. An application may respond to its hide notification by
going to its tray, stopping rendering, or closing. The
[WM_SHOWWINDOW documentation](https://learn.microsoft.com/en-us/windows/win32/winmsg/wm-showwindow)
confirms that hide/show state changes notify the source application.

[ShowWindowAsync](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-showwindowasync)
only posts a show-window event; its successful return confirms submission, not
completion. A timeout/generation change cannot remove a hide event already queued
to another application's thread. Synchronous `ShowWindow` avoids that queue lag
but may block while the source thread processes messages. Either production route
requires bounded, identity-checked recovery and visual/restore/cancellation
acceptance; successful flag checks alone do not settle those risks.

## Other route review

- [DWM thumbnails](https://learn.microsoft.com/en-us/windows/win32/dwm/thumbnail-ovw)
  are live copies into an owned destination. Their
  [properties](https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/ns-dwmapi-dwm_thumbnail_properties)
  contain source/destination rectangles and opacity, not source-window deformation.
  Multiple cropped copies could form strips, but cannot remove the original source.
  Merely delaying minimization reveals a stationary original beneath the shrinking
  mesh; a fake background hides real underlying applications and is not acceptable.
- A temporary empty region is **not implemented**.
  [SetWindowRgn](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowrgn)
  changes drawing/hit area and sends `WM_WINDOWPOSCHANGING/CHANGED`; success transfers
  ownership of the HRGN to Windows. Region restoration races the application's own
  region/layout changes, and failed restoration can leave an invisible window.
  [GetWindowRgn](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getwindowrgn)
  returns `ERROR` for both no explicit region and failure, so neither may be guessed
  to mean an original null region. Root declined this production route.
- The existing `native::animate` still creates its transition guard using the
  unreadable `DWMWA_TRANSITIONS_FORCEDISABLED` getter. It therefore cannot deliver
  even an owned-source animation unchanged on this host. Same-process cloak would
  be a separately designed owned-window route; it is not implemented by this work.

UI/UX guidance influenced the explicit spatial anchoring, interruptible preview,
one-shot playback, reduced-motion handling and distinction between visual preview
and native-operation completion. It did not authorize system preference changes.

## Required acceptance before calling the Dock feature complete

- Actual source pixels stretch into a curved funnel before swallowing; never only
  scale/fade or play a second Windows shrink under the mesh.
- Native visible source disappears without a flash, stale rectangular substrate,
  black source, fake desktop or input-blocking overlay.
- The correct icon anchors the neck at 100/150/200% DPI and all Dock directions.
- Normal/maximized, repeated toggle, immediate restore, external close, capture
  failure, hung source, session/display changes and motion preferences retain a
  reachable source window and promptly remove all owned overlay resources.
- Scope remains clearly **Dock-only**, and synthetic geometry acceptance is never
  represented as title-bar/keyboard/global minimization support.
