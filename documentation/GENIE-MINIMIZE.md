# Dock Genie minimize and restore

The Dock setting selects **Genie** or **Native** (default: Genie). The Genie path
supports 4K windows using adaptive internal rendering. Window size alone does not
switch a 3840 x 2160 window to Native. There is no environment-variable gate.

Right-click the empty Dock background → **Dock window animation** to change the
mode immediately. The submenu reads fresh settings and saves only this field,
serializing repeated choices and reporting save failures. Its settings link opens
the existing Dock page; edits made on that page still use the normal Save action.

This covers windows toggled through the Dock and native title-bar minimize
buttons. A per-Dock registry publishes measured resting physical icon bounds
for each window, including grouped windows and the hidden Dock. Preview buttons
use that registry when no explicit target is supplied. A custom restore requires
a recent snapshot captured by our minimize.

The native-button path uses a dedicated low-level mouse hook with a bounded
12 ms native hit test. Only a confirmed HTMINBUTTON with a registered Dock target
is handled; capture and playback run on another thread. Pressing and releasing
must refer to the same HWND/process and unchanged foreground; dragging off the
button cancels normally. Background-window minimize buttons are supported without
activating them. Unsupported owner-drawn client buttons, Win+D, Alt+Tab and other
keyboard/native commands keep Windows behavior. No DLL is injected into apps.

## Geometry and 4K rendering

The real Dock icon rectangle and source bounds remain in physical screen pixels,
including negative monitor coordinates. Independent cosine easing deforms a
26 x 26 shared-vertex mesh (25 x 25 cells, 1,250 textured triangles). A neck first
stretches towards the icon and then swallows the window. Restore samples the same
geometry backwards. This is original code informed by static architecture
findings; no MyDockFinder machine code, shaders or artwork is incorporated.

PrintWindow writes the original-size window into a temporary DIB. Native GDI
crops/downsamples it before the BGRA pixels enter Rust/cache, avoiding scalar
channel conversion or retained full-size 4K screenshots. Redirected full-content capture is tried first; ordinary WM_PRINT is a compatibility path for a GDI app with an empty redirected frame. Both use PrintWindow and keep display-affinity/black-image restrictions. Texture budget:
1,048,576 pixels; a 3840 x 2160 source becomes 1365 x 768 (4,193,280 bytes).

The animated framebuffer starts at 786,432 pixels (1182 x 665 for 4K), preserving
aspect ratio. Mesh vertices are projected into it while the actual window and
layered overlay remain full physical size. Native GDI expands the frame into the
complete 3840 x 2160 presentation DIB. A local 2 x 2 -> 4 x 4 DIB probe first
verifies that 32-bit SRCCOPY preserves both transparent and opaque BGRA pixels.
The fast path is used only on success; otherwise alpha-correct AlphaBlend is used.

Before hiding anything, a wide-neck frame is measured. If it exceeds 20 ms,
internal detail is reduced, down to 196,608 pixels. A presented frame over 25 ms
also reduces subsequent internal detail. It does **not** abandon 4K animation:
late frames follow a 420 ms wall-clock timeline. This trades moving-image detail
for responsiveness; native window content becomes sharp again at the handover.

The transient overlay is a non-activating, click-through, alpha-layered tool
window, without a webview. It receives no input and has scoped HWND/DC/bitmap
cleanup. The screenshot fades out during its final approach to the icon. Every
exit replaces the retained layered screenshot with a preallocated transparent
surface before hiding/destroying the HWND and flushing queued DWM cleanup in an
active display session. The main UI event thread does not perform capture or
frame rendering.

## Native state, cancellation and recovery

One action generation spans custom animation and native fallback. Newer Dock
actions cancel older work; an old fallback cannot overwrite a newer action.
One animation and one PrintWindow worker are allowed at a time. Only short
window-state commits hold a mutex; capture and rendering do not.

The active path neither reads nor writes the setter-only
`DWMWA_TRANSITIONS_FORCEDISABLED`, nor changes Windows animation preferences or
foreign window cloaking. Minimize paints the snapshot, briefly hides the real
window, submits SW_SHOWMINNOACTIVE and verifies visible/iconic state. The real
window is already minimized while its snapshot enters the Dock.

Restore plays the cached snapshot backwards while the app stays iconic. The
full-size final overlay covers its short native restore commit. Saved
WINDOWPLACEMENT preserves maximized state and the normal restore rectangle.
A hide guard queues a reveal on error/cancellation, observes visibility within a
bounded deadline and clears its HWND/process/generation lease. A hung app can
outlast the deadline; recovery is still queued after the hide. Closed or changed
processes are never revealed by cleanup. Native fallback restoration is queued
asynchronously rather than blocking on an unresponsive application's UI thread.

The app-window tracker ignores only this exact transient hide/state-change
lease. A queued ObjectHide also checks current visibility, retaining an already
visible/iconic Dock item. Real hide and destroy events remain actionable, without
a timed event-suppression window. A newer Dock action, external restoration or
intervening foreground activation dismisses playback without stealing focus back.

Cached textures are memory-only: at most sixteen entries / 64 MiB, eligible for five
minutes, with lazy expiry on cache access. HWND/process/class and saved placement
must match. Stale, missing or changed snapshots restore
natively. Screenshots are never written to disk.

## Exclusions and resource bounds

- Native is the opt-out. Selecting Genie explicitly requests this effect even
  when Windows motion is disabled; Dock geometry is measured independently of
  that system signal. Existing power/performance optimization modes use Native.
  Changing the selected preference stops an active custom effect.
- Protected-capture, elevated/unmanageable, cloaked, black or changed windows
  retain native behavior; no capture protection is bypassed.
- Off-screen, cross-monitor or mixed-monitor destinations remain native. Input
  and physical surfaces are bounded to 8192 per dimension and 16,777,216 pixels;
  4K fits these allocation limits.
- Capture has a 250 ms deadline, including native downsampling, and checks action
  cancellation every 8 ms. At most one worker can remain blocked in PrintWindow;
  later actions do not accumulate capture threads.
- Each hide/state observation has an 80 ms deadline with queued recovery. Display
  or DPI changes, lock, display-off and source closure stop the overlay safely.

## Verification

```powershell
cargo check -p macui --bin macui
cargo test -p macui --bin macui widgets::weg::genie -- --test-threads=1
cargo test -p macui --bin macui owned_window_commit_cancel_close_and_maximized_restore -- --ignored --test-threads=1 --nocapture
cargo test -p macui --bin macui owned_four_k_capture_minimize_restore_probe -- --ignored --test-threads=1 --nocapture
```

The normal suite checks all four mesh directions/endpoints/phases, allocation
bounds, cancellation, cache identity/expiry and stale native actions. A full
3840 x 2160 synthetic frame test checks **every** output pixel for correct opaque
color or transparent exterior, including the actual target on a negative-coordinate
monitor. On 2026-11-01, optimized debug draw + full 4K upscaling measured
8.71 / 10.64 / 9.15 / 8.13 / 5.53 / 5.14 ms at phases 0 / .15 / .35 / .65 / .85 / 1.
These are local CPU/GDI measurements, excluding UpdateLayeredWindow/compositor
work, not a universal frame-rate guarantee.

Measured result: all 15 normal Genie tests passed, including 4K alpha/target and quality-reduction checks. cargo check -p macui --bin macui passed. The explicit normal/maximized/cancellation/destruction probe passed.

Explicit probes touch only self-owned windows; capture is performed on an opaque off-screen fixture before it is made transparent for playback. The ordinary
state probe covers normal/maximized placement, hide cancellation and destruction.
The 4K probe measured PrintWindow/downsampling at 168 ms, below the 250 ms allowance. It verified all 1,048,320 texture pixels, then submitted 55 UpdateLayeredWindow frames through real native minimize + restore in 942 ms, preserving visibility and placement. It runs native playback
with an invisible test overlay; it does not cover the user's desktop with artwork
or manipulate other applications.

Visual acceptance on ordinary Explorer/Edge/Win32 windows, real Dock clicks,
rapid alternation and 100/150/200% scaling remains necessary. No global
keyboard interception, live-video deformation or GPU renderer is claimed.
API reference: [Microsoft PrintWindow documentation](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-printwindow) describes owner-rendered capture and its synchronous behavior.
