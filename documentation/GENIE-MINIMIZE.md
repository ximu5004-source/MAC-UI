# Dock Genie prototype — 2026-10-04

Status: **disabled experimental prototype, not a usable/accepted feature on this
host**. The normal Dock path still invokes Windows' own minimize and performs no
window capture. The backend is gated by the development-only process environment
variable `MAC_UI_EXPERIMENTAL_DOCK_GENIE=1`; no user configuration is changed.
This is not a replacement for every Windows minimization animation.

## Measured blocker on this host

An owned **hidden, never activated** test window was used to probe the native
transition attribute on 2026-10-04. `DwmGetWindowAttribute` failed for
`DWMWA_TRANSITIONS_FORCEDISABLED`. The safety guard therefore declined to set it,
and the prototype falls back even with the environment gate enabled. Guessing a
previous value of `false` would risk altering another application's behavior.
The [official attribute documentation](https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/ne-dwmapi-dwmwindowattribute)
only specifies this attribute for the setter. Replacing the native animation
without double-rendering or unsafe window-state changes needs a separate design.

The hide-then-minimize alternative was reviewed, not executed. The documented
[`ShowWindow` states](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-showwindow)
make `SW_HIDE` an additional visibility/activation transition, not an atomic
"minimize without DWM animation" operation; `SW_SHOWMINNOACTIVE` also changes
activation semantics. Neither is documented as a safe way to substitute arbitrary
third-party window animations. Applying them speculatively would risk application
hide handlers, focus and restore behavior, so they were not used as a workaround.

## Isolated cloak alternative probe — 2026-10-04

A second public-API approach was tested using **only synthetic top-level windows**.
`DWMWA_CLOAKED` is readable and `DWMWA_CLOAK` can temporarily hide a composed
window without changing its visibility style. Microsoft's
[DirectComposition animation example](https://learn.microsoft.com/en-us/windows/win32/directcomp/how-to--animate-the-bitmap-of-a-layered-child-window)
uses cloak/uncloak when rendering a replacement for an owned layered child.
It does not establish permission to cloak another process's top-level window.

Measured result on this host:

- Same-process synthetic top-level: cloak, cancel/uncloak, native `SW_MINIMIZE`,
  uncloak, nonactivating normal restore, and restore while cloaked passed. The
  normal placement was retained and the corrected probe never became foreground.
- A top-level created by an independently spawned copy of the **test binary**:
  setting `DWMWA_CLOAK` returned **`0x80070005` (`E_ACCESSDENIED`)**. Its cloak and
  placement states remained unchanged. This prevents using the cloak approach to
  replace ordinary third-party application animations on this host.
- The first probe used `SW_RESTORE`; it activated the off-screen synthetic window
  despite `WS_EX_NOACTIVATE`, and the foreground assertion failed. Cleanup
  immediately destroyed that owned window. The probe was corrected to the
  documented nonactivating `SW_SHOWNOACTIVATE` normal restore. This intentionally
  does **not** claim equivalence with the production Dock's activating restore or
  prove preservation of an arbitrary maximized application's state.

The test lives in `src/background/widgets/weg/genie/cloak_probe.rs`, is compiled
only under `cfg(test)`, and is ignored by default. Explicit invocation:

```powershell
cargo test -p seelen-ui --bin seelen-ui owned_cloak_probe -- --ignored --nocapture --test-threads=1
```

Result: **1 passed**, including the explicit unsupported cross-process outcome.
The helper uses an exact test filter and private handshake, validates the expected
process before every mutation, owns a 10-second cleanup deadline, clears its own
cloak and destroys its own HWND. Parent cleanup requests normal exit, then stops
only that exact spawned helper if necessary. Windows are off-screen, tool windows;
the corrected probe does not activate them. No user applications, global settings,
keyboard hooks, injected modules, or normal runtime commands are modified.

This probe rules out that alternative on this host. It is **not** a Genie feature
implementation or visual acceptance; default native minimization remains in use.

## Scope

Only clicking a single-window application in MAC UI's Dock to minimize its focused
window supplies an animation destination. Other callers retain native behavior:
title-bar minimize, Win+D, native taskbar, workspace changes, and multi-window Dock
preview actions. Restore remains Windows-native. No injected DLL, Explorer patch,
global keyboard/event interception, or change to a system-wide animation setting.

The renderer captures the window once into memory, bends a 256-strip mesh toward
the actual Dock icon, then collapses the neck into that destination over 420 ms.
It is deliberately a frozen snapshot, not a live-streaming window or just a scale
and fade. The first frame covers the original window; Windows itself still receives
`SW_MINIMIZE`. A temporary per-window `DWMWA_TRANSITIONS_FORCEDISABLED` flag avoids
double animation. Its previous value must be successfully read and is restored
with RAII as soon as minimization is observed, including error/cancellation paths.

The short-lived overlay is a non-activating, click-through, tool window with
per-pixel alpha and no webview. Surface/DC/bitmap/window resources all have scoped
cleanup. It neither takes keyboard focus nor creates an input-blocking screen.

## Safety and fallback

- CSS reduced motion and Windows client-area animation preference skip the effect.
- Protected-capture, elevated/unmanageable, wholly black, invalid or moved windows
  fall back to ordinary native minimize.
- Screen-spanning or off-screen windows and targets outside the source monitor
  take the native path. This avoids incorrect mixed-DPI/cross-monitor projections.
- The frontend converts the icon's CSS coordinates from the actual webview origin,
  not the much smaller Dock hitbox. Non-finite and zero-sized targets are rejected.
- Capture waits at most 80 ms. At most **one** capture worker can remain blocked in
  another application's `PrintWindow`; further requests immediately fall back.
  A late result is discarded, not displayed or stored on disk.
- Input surface dimensions are bounded before capture and each owned DIB is capped
  at 16,777,216 pixels. Only one animation runs at a time. Work is outside the UI
  event thread; late frames are skipped against a wall-clock duration.
- A newer Dock toggle/restore cancels an older generation. External restoration,
  source closure, display/DPI change, locking or display-off removes the overlay.
  Capture/renderer failure never makes the native minimize depend on animation end.

## Automated checks

The pure geometry test suite verifies identity/end position, a genuinely non-affine
neck, positive finite strips with contiguous axes, and all four Dock directions.
The frontend tests cover fractional DPI, negative monitor coordinates and invalid
targets. Backend tests additionally cover bounded capture leases, invalid allocation
dimensions and transparent/opaque pixels in off-screen native GDI surfaces.

Measured results: `cargo check -p seelen-ui --bin seelen-ui` passed; the 8 focused
Rust tests passed, including the explicit unsupported-attribute fallback probe;
the 2 frontend coordinate tests passed. No other applications were manipulated by
these tests. Passing the fallback probe does not mean the animation is available.

These do **not** prove a smooth/accepted end-user visual result. Use the actual
debug build to complete the checklist below before considering this feature done.

## Pending acceptance checklist

- Single-window Dock: Edge/Explorer/ordinary Win32 app visibly bends and enters its
  correct icon; no duplicate Windows shrink, black flash or leftover overlay.
- Restore through Dock and Alt+Tab during the effect; close source mid-effect;
  rapid alternation; ensure no stolen focus, unreachable app, or stuck topmost surface.
- 100%, 150%, 200% scaling; Dock top/bottom/left/right; secondary monitor with negative
  coordinates; maximized/restored windows. Verify documented native fallbacks.
- Reduced motion: instant ordinary native behavior with no custom deformation.
- Capturing a protected/elevated/hung app: action still minimizes normally, memory
  and worker counts do not accumulate across repeated requests.
- Measure frame pacing, visible-step quality and memory on this user's 4K display.
- Confirm the user accepts the **Dock-only** scope. If every title-bar/keyboard
  minimization must use Genie, that requirement remains unimplemented and unaccepted.

## Primary API references

- [UpdateLayeredWindow](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-updatelayeredwindow)
  supplies per-pixel transparent composition and recommends keeping overlays small.
- [AlphaBlend](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-alphablend)
  scales premultiplied-alpha bitmap strips into an independent memory surface.
- [WinEvent constants](https://learn.microsoft.com/en-us/windows/win32/winauto/event-constants)
  provide observation of native transitions, not a supported API for replacing
  every process's window-deformation compositor.
