# Frosted panels: scoped QA — 2026-10-04

Current status, 2026-10-05: **the user accepted the restarted test build and requested MAC UI 3.0.0 packaging.**
The current visual panels and restored Win-key Launchpad behavior are accepted. Genie remains disabled and is not a delivered minimize-animation feature. Earlier pending/rejected results below are historical evidence, not a replacement for the latest acceptance.

## Calendar, account, session sheet and Win-key acceptance — 2026-10-05

- Calendar and account surfaces now use the same neutral 10% coat, native 18 CSS px frost, 28 px corners and hollow edge-only gloss. The session sheet uses 32 px corners. None adds an exterior shadow or icon halo; foreground follows light/dark appearance rather than wallpaper-average selection.
- Exact native frost IDs now include calendar, account and power. The full-screen power dismissal area stays transparent and is never submitted as a frosted region. The wallpaper-sampling allowlist was not expanded for these three panels.
- Calendar retains month/year navigation and selected dates with semantic, translated buttons. Account directory opening and expansion are separate native buttons, with one bounded scroll owner; expansion never invokes file opening.
- The account session entry and Launchpad footer hide their source panel before opening power. The six session actions retain the existing confirmation policy; initial focus is Cancel, repeated/pending requests are guarded, and errors restore safe focus.
- Actual component previews with the legacy theme verified both light/dark foregrounds, 10% coating, 18 px blur, corners, no shadows, calendar selection/navigation, account expansion and mock power cancellation. Real native calendar/account/session frames were captured: underlying content is diffused, and the account window disappears before power appears. Only Cancel was operated in the real session sheet; no real logout, shutdown, restart, sleep or lock was executed.
- A 360×440 power preview uses a fictional monitor of the same size. Resizing the iframe without updating its monitor snapshot initially caused a fixture mismatch; the harness now reloads the mock after resizing. This is not evidence of production multi-monitor/DPI acceptance. Cancel-first focus may scroll the constrained sheet to the bottom.
- Final frontend regressions: 119/119 passed; complete type checking passed with zero Svelte errors/warnings. Targeted native frost scope, settings fallback and service shortcut checks passed. The user confirmed acceptance after the rebuilt service and test app were restarted on 2026-10-05.
- Installer packaging is a separate operation; this acceptance does not establish actual installer execution, long-run display recovery or an enabled Genie animation.

## Integration status

- Reference architecture was inspected in the two user-supplied Windhawk source files. The relevant pipeline is a real backdrop source followed by Gaussian blur and optional material layers; it is not a WebView CSS filter or wallpaper screenshot. No injected/XAML code was copied into MAC UI.
- MAC UI now registers exact rounded paint regions for desktop stacks/toolbar, Dock, Quick Settings, media popup, OSD, keyboard selector, network popup, notifications and system tray. The Windows implementation uses a host-backdrop brush and an 18 CSS px Gaussian effect behind the WebView, without native tint or saturation. A neutral 10% CSS coat and a separately masked 1.5 px edge highlight supply the material. Existing Dock opacity/gloss preferences are preserved.
- Removing outer shadows and the old desktop ±5 px input gutter does not expand desktop input coverage. Resize handles now stay inside each card; native desktop gaps remain available.
- Hidden OSD sends an empty paint-region list. Geometry writes are serialized/coalesced, including deduplicated show→hide and display recovery during an in-flight write. Display recovery reapplies native geometry after the WebView viewport is corrected.
- Native application failure is reported, not treated as proof of working blur. CSS is a preview fallback and cannot sample other applications behind a WebView. This native path requires supported Windows host-backdrop composition.
- Quick Settings/media/OSD and the four additional popovers can select black or white foreground from the Windows wallpaper under their own window. This changes text, not material color. It is a wallpaper-average heuristic and cannot guarantee contrast when a different application is underneath.

### Build / run evidence

- Unified generated bindings, frontend build, final type check (0 errors / 0 warnings), and debug binary build succeeded.
- Frontend regression suite: 79 tests passed, including geometry, hide/write ordering, recovery and foreground selection.
- Core generation: 88 Rust tests passed; shared utilities: 47 tests passed. Additional targeted backend suites passed for 8 Genie safety/geometry checks and 2 frosted wallpaper-sampling scope checks.
- Earlier, starting `target/debug/mac-ui.exe` returned **Access denied** before any application log was produced. On this follow-up the exact same path launched successfully without an ACL, security-setting, filename or configuration workaround. The evidence supports a transient launch condition, not a specific file-lock diagnosis.
- The rebuilt `target/debug/mac-ui.exe` is now running (follow-up PID 79776), with project-root CWD and verified static resources. `mac-ui-frost-four.stdout.log` records native surface creation for the actual keyboard/network windows. Windows screenshots confirmed the new rounded frames and intact real network rows over the currently dark underlying application; those alone do not establish dynamic blur over a changing scene. Existing release files and installed application files were not replaced.
- Genie is a disabled experimental prototype; the local DWM attribute-read probe failed, so its safe path preserves normal Windows minimization. Neither Dock nor all-system Genie is accepted as a delivered feature. See `GENIE-MINIMIZE.md`.
- No new installer was produced and no version increment was made during this unfinished visual-validation pass.

## Scope

- Control Center uses compact three-column pill controls with labels below, preserving radio, hotspot, HDR, appearance and night-light actions. Wi-Fi/Bluetooth detail arrows are separate accessible buttons.
- Pending toggles disable repeated submission; failures have inline alert text and can be retried.
- Quick Settings, media popup, volume/brightness OSD, keyboard, network, notification and tray popovers opt into the shared neutral frosted surface. The account popup retains the user's restored previous surface; unrelated/third-party popovers are not globally restyled.
- OSD controls use semantic mute buttons, named sliders, rounded thin tracks, a 22 px surface radius and no outer shadow. Control Center uses 28 px corners.
- Pointer capture/cancellation, trailing slider update flushing and component teardown cleanup are covered. Brightness uses the last available level as its upper bound. Mute changes now reveal the OSD even when the numeric level is unchanged.

## Reproduce the isolated preview

```powershell
node scripts/qa/frosted-panels-build.mjs
node scripts/qa/frosted-panels-server.mjs
```

Open `http://127.0.0.1:3584/index.html`.

The fixture builds the **actual** `QuickToggle`, OSD `MediaDevices` / `Brightness`, `DesktopStack`, `BackgroundByLayers`, and shared material CSS. Generated files go only to `target/frosted-panels-qa`, never `dist` or the installer. Fixture controls affect fixture state only; no Windows settings or user data are read or changed. Every mock native backdrop call deliberately rejects as unsupported. The visible notice distinguishes CSS fallback from native composition.

## Verified

- `npm run type-check`: 0 errors / 0 warnings after the control/OSD changes.
- `tsx --test src/ui/svelte/flyouts/volumeChange.test.ts`: 3 tests pass (mute/unmute, device removal, unchanged/changed percentage).
- Preview bundle builds successfully.
- Browser-computed surface properties: neutral `rgba(255,255,255,.1)`, `blur(18px) saturate(1)`, shadow `none`, radius 28 px (Control Center) / 22 px (OSD and DesktopStack).
- Reflection pseudo-element has a 1.5 px hollow `exclude` mask. It does not paint the interior.
- Wi-Fi detail button works via Enter. Tab proceeds to the next control, with a visible 2 px focus outline.
- During the artificial 500 ms request, both toggle and detail buttons are disabled and “正在更新…” is visible. Injected failure produces the inline alert; Enter retry succeeds.
- Output volume changes 44 → 45 with ArrowRight, then 45 → 91 by pointer drag. The horizontal and vertical views stay synchronized. Mute retains the numeric level. Brightness End reaches 100.
- Light and dark layouts have no label or slider clipping in the normal preview viewport.

### Material pixel checks

Browser screenshots are JPEG, so corner comparisons permit compression noise. The view and material geometry were unchanged between captures.

| Check | Result |
| --- | --- |
| Dock center, gloss 0% versus 100%, 6,528 pixels | Identical; maximum RGB-channel delta 0 |
| Dock top-left and bottom-right exterior against hidden-material baseline | Delta 0 |
| DesktopStack top-left exterior against hidden baseline | Delta 0 |
| DesktopStack bottom-right exterior against hidden baseline | Maximum channel delta 2 (JPEG tolerance); no dark rectangular substrate |

Evidence files under `target/frosted-panels-qa/`:

- `dark.jpg`, `light.jpg`, `interaction.jpg`
- `material-gloss-0.jpg`, `material-gloss-100.jpg`, `material-hidden.jpg`

## Not passed / still pending

- A 10% neutral surface cannot guarantee readable white text over every bright wallpaper. The bright fixture exposes low contrast in dark-theme secondary labels. No large colored/dark tint was added against the requested material direction; real wallpaper readability needs separate native review and a suitable contrast fallback if necessary.
- Browser CSS blur does **not** prove Windows host-backdrop composition, native clipped-corner behavior, OSD hide/show region cleanup, or multi-monitor/DPI recovery. Those require the real application.
- Opening actual Wi-Fi/Bluetooth details, changing real radio/HDR/night-light states and real audio-device levels were not exercised by this fixture.
- Reduced-motion and forced-color fallbacks exist in CSS but OS-level preference switching has not been visually accepted in this scoped check.
- User acceptance has not yet been received. Do not label the entire glass update complete solely from these results.

## Adaptive-foreground follow-up

- The fixture now imports the real `frostedForeground` function, estimates luminance from its known gradient/stripe palette, and assigns `data-frosted-ink` to the Control Center and OSD surfaces. Its UI explicitly labels this as **simulated wallpaper sampling, not native sampling**. Both bright and dark palette controls are available.
- Fixed a specificity collision: the generic adaptive rule previously tied the base `.slu-std-popover.mac-panel` rule, allowing its later-loaded `--mac-secondary` to win. The adaptive selector now also requires `.mac-panel` or `.flyout`, preserving the selected pure black/white ink without changing the 10% neutral material.
- After these changes: preview bundle builds; `frostedContrast.test.ts` + `volumeChange.test.ts` pass 5/5; `npm run type-check` passes with 0 errors / 0 warnings.
- **Visual re-acceptance remains pending.** The previously used in-app browser disconnected before the refreshed page could be inspected. Documented recovery selected an available Chrome connection, but lightweight calls and the one retry failed. Read-only diagnostics report Chrome running, its extension enabled, and the native-host manifest valid; no browser settings, extension installation, or native-host configuration were changed. No new temporary browser tab was confirmed created, and no existing user tab was touched.
- The earlier screenshots above predate adaptive foreground selection and must not be presented as proof that the new contrast behavior was visually accepted.

## Four additional popovers — follow-up acceptance

The keyboard, network, notification and tray screenshots revealed that these four
entry points still used the 97% utility material and did not register native frost.
Both native allowlists now explicitly include their exact widget IDs; arbitrary
widgets and the account popup remain excluded. All four use 28 px painted corners.

Reproduce the actual-entry fixture (no copied/retyped component replicas):

```powershell
node scripts/qa/frosted-popovers-build.mjs
node scripts/qa/mac-panels-server.mjs target/frosted-popovers-qa 3585 keyboard-selector,network-popup,notifications,system-tray
```

The bridge rejects native frost, so web screenshots cannot falsely establish
native rendering. All list items, notifications and operations are fictional.
Generated SVG icon routes tolerate the production cache's `?hash=` query; no
native icon paths or user files are read.

Verified in the browser:

- All four outer surfaces: neutral white 10% coat, 18 px unsaturated CSS preview
  blur, 28 px radius, no outside shadow, hollow `exclude` rim. Bright fixture
  sampling selects black text; dark sampling selects white text.
- A 448 px work-area-constrained network panel initially shrank hotspot/connected
  rows to 18/19 px. The implementation was repaired after this failed check.
  Recheck: complete 48/50 px rows, one 304 px scrolling body, and fixed radio/footer.
- Selecting network 29 scrolls the real list to the final row and expands its
  password/detail controls. No real network connection was attempted.
- Keyboard selection changes the pressed item and records the expected layout
  command. Escape records `hide`; repeat/concurrent close requests are guarded.
- Three real notification components retain complete card height inside one scroll
  body with a fixed footer. Clearing the fictional notifications reaches the empty
  state; no real notifications were cleared or replied to.
- All 40 fictional tray entries, including hidden entries, remain in the list.
  Scrolling/clicking entry 39 preserves the fixed heading and records the existing
  native `LeftClick` mapping. Icons load at 64 px intrinsic size and remain contained
  in their 24 px slots; no additional icon reflection or halo was introduced.
- A no-adapter network state stays within the frame and retains the 10% material.
- Follow-up frontend regression suite: 79 passed. Native frost/sampling scope:
  3 passed. Updated frontend build and type checks passed.

Native screenshots of all four actual popovers were inspected. Keyboard/network
frames have rounded transparent corners with no visible rectangular substrate;
real network rows remain complete while scanning updates their order/count.
Notifications retain full cards, and the real tray retains all 14 entries in its
scroll body with contained icons. The blue striped test page underneath the lower
part is diffused rather than showing sharp stripes through those surfaces.
This confirms these particular frames, not every wallpaper/DPI combination.

An explicit, ignored Rust visual fixture creates only its own non-activating,
click-through checkerboard source behind the current debug network popup. It
validates the exact process image, alternates its own source every 5 seconds,
destroys its window/brushes after 90 seconds or target movement/closure, and does
not capture pixels or operate other windows. Two lifecycle runs passed (the second
exited when the real network scan changed the popup height). These are **cleanup
tests, not dynamic-blur acceptance**: the initial captured source changes were
inconclusive. The source was repaired to be topmost and its color phase changed to
blue/cyan (rather than two colors averaging almost neutral gray).

The subsequent native network screenshots, taken 5.5 seconds apart, show the same
frame switching from a gray frosted interior over a black/white checkerboard to a
blue frosted interior over a blue/cyan checkerboard. Outside the rounded frame the
48 px squares remain sharp; inside, the square boundaries are diffused and the rim
does not wash over the interior. This is direct visual evidence of real background
sampling, not a wallpaper image or browser-only CSS effect. The temporary source
then destroyed its own HWND/brushes cleanly. Escape closed the real network popup;
the runtime log confirms removal of native frost for HWND 4719138. This is scoped
to the captured Windows setup, not every display/DPI or wallpaper.

The application CLI triggered only its own test popovers; no Win-key input,
third-party tray action, real radio switch, account/authentication or system setting
was automated. The prior launch/browser connection failures are historical, not
current blockers.

Genie remains unimplemented for all entry points. A follow-up synthetic-window
cloak probe passed its state tests but returned `E_ACCESSDENIED` for the independent
helper process, ruling out that public-API workaround on this host. This does not
count as animation acceptance; see `GENIE-MINIMIZE.md`.

## Bluetooth, plain state glyphs and requested Launchpad — latest follow-up

User acceptance is still required. The first new Launchpad was explicitly
rejected for low-contrast labels, a black autofocus search ring, Windows-style
partitions/quick access, and sharp root icons bleeding through its folder sheet.
Those initial screenshots are failure evidence, not acceptance evidence.

Bluetooth uses the same opt-in native frost and 28 px frame as network. Browser
checks of the actual entry passed: 30 fictional devices in one scrolling body,
448 px constrained frame/304 px body, fixed radio and footer, last-row expansion,
empty scan, radio-disabled and no-adapter states. No real radio/pairing action was
attempted. A native source fixture was extended only to exact debug network,
Bluetooth and Launchpad titles; process-image and lifecycle checks remain in place.
The real Bluetooth window changed from its dark application backdrop to blue/cyan
frost over the owned source, while the exterior checkerboard stayed sharp. Real
scan updates changed the popup height, so the fixture destroyed its own source
after 0.87 s; this run proves cleanup and the captured background response, not a
long-running two-phase Bluetooth capture or all DPI layouts.

Quick Settings: the first removal of opaque white discs left neutral inner circles.
User rejected those too. All six glyphs now have **transparent background, zero
border/radius, opacity 1, no filter or shadow**. Enabled/disabled artwork has the
same form and differs only in ink; the outer pill is unchanged. Browser screenshots
and computed styles verified light and dark variants. One request at a time,
disabled main/details buttons, injected failure, retry, and details Enter activation
were rechecked with fictional commands: Wi-Fi toggle count 3, detail count 1,
pending count returned to 0. Real connectivity/HDR/night-light were not changed.

Launchpad keeps its existing persisted data/search/import/keyboard behavior, but
is being reworked after the above failed acceptance into the user's requested
macOS-style grid. Wallpaper-only text sampling is removed **for Launchpad only**:
it chose black labels over a dark live application because the wallpaper was
brighter. Toolbar wallpaper-adaptive ink is unchanged. This improves the observed
dark-appearance case, not a guarantee of contrast over every arbitrary window.
Actual revised grid/folder/search acceptance must be recorded below before calling
this implementation finished. A 10% glass surface still cannot itself guarantee
readability over every background without a more restrictive contrast fallback.

Classic Genie remains a Dock-only requested feature, not an enabled feature. The
Rust-generated synthetic preview has been visually checked; native source
de-duplication/restore/cancellation are still pending. See the separate geometry QA
record for the exact distinction. No global animation preference or injection was
used. No new installer or version change was produced by this follow-up.
