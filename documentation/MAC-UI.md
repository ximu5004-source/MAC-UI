# MAC UI customization

Author / publisher: **JONA**. Public product name: **MAC UI**. License: AGPL-3.0-or-later; see `NOTICE.md` for upstream attribution. This is not an Authenticode signature.

## Current release — MAC UI 3.0.1, 2026-10-05

The user confirmed the Win-key issue resolved and authorized 3.0.1 packaging. This maintenance release adds an explicit Launchpad enable switch in its shortcut group, accepts standalone Win during editing, synchronizes file-based and initial settings updates, keeps capture running across shortcut disable/re-enable, and isolates shortcut recording completion and cancellation by request identity. The accepted 3.0 visuals remain unchanged; Genie remains disabled.

Application, service, CLI, Hook DLL and local-library manifests use 3.0.1. The new package is built alongside the preserved 3.0.0 installer; no installation or automatic upgrade is performed. See `release/2026-10-05/README-3.0.1.md` for packaging verification.

Packaging verification: the final stable installer is 36,373,915 bytes, SHA-256 `b63d7cd40420957ff77eec0b7e40c162c27d6aefc70e73f6700ef01ae6c20e73`. Nightly debug symbols are excluded at compile time. All 370 static resources and their internal signature pass verification; production settings/dialog JS and CSS are confirmed inside the 3.0.1 main executable. Frontend regression tests (139), core tests (5 plus 24 steps) and full type checks passed. Windows Authenticode remains NotSigned. Bilingual public release notes are in `documentation/releases/3.0.1.md`; this packaging record does not assert that a GitHub release has been published.

## Previous release — MAC UI 3.0.0, 2026-10-05

The user accepted the restarted test build and authorized version 3.0 packaging. The current macOS-inspired Launchpad is one large, crisp application grid without Quick Access/right sidebar; folders hide the underlying app grid. Neutral native-frost glass now covers the built-in popovers, calendar, account and session sheet, with edge-only gloss and no added icon halo. Cancel-first session confirmations and source-panel dismissal are retained.

For an enabled Launchpad, the editable default standalone Win shortcut has been restored. It triggers on strict key release, supports both Win keys and preserves native Win combinations; explicit saved shortcut overrides remain respected. A new configuration still defaults to native Windows Start until the custom Launchpad is enabled. Existing configuration is not replaced by the installer.

The user acceptance covers these current visual/shortcut changes, not an enabled Genie animation. Genie remains an experimental gated prototype and normal minimization uses Windows. Historical acceptance limitations below are retained as records, not current failures for the accepted normal-size visual panels.

Version manifests and local lockfiles use 3.0.0. The new installer is placed under `release/2026-10-05/`, without modifying earlier release copies or automatically installing over the active test application. Packaging verification is recorded in that directory's README.

Packaging completed: `release/2026-10-05/MAC-UI-3.0.0-x64-Setup.exe` (47,253,280 bytes), SHA-256 `61b65af873fac740ce5427219597d8e8e99a5fcd22be235d032e67fc1b141e99`. Production UI, optimized main/helpers/DLL and NSIS succeeded; 119 frontend tests and full type checks passed. All 370 static resources match their valid internal signature manifest, with no extra files. Eight Launchpad/calendar/account/power JS/CSS blobs match production output and are present in the new main executable. New NSIS inputs exclude private signing material and include license/attribution. Product version metadata is MAC UI / JONA / 3.0.0; Windows Authenticode remains NotSigned. No installer execution was performed.

## Compatibility

The installer publishes `mac-ui.exe`. The service recognizes this name and the legacy development executable. The existing application identifier, helper executable names, service task identifier, `@seelen/*` widget IDs and app-data paths are deliberately retained. Existing pinned applications, desktop stack positions, dimensions, collapsed state, wallpaper preferences and resource files continue to use the same storage. No migration deletes user files.

MAC UI does not install upstream application updates. It currently uses manual installers; a future automatic updater needs a MAC UI release endpoint and signing key. Seelen accounts and the resource marketplace are still explicitly upstream services.

## Implemented changes

- Eight system popovers share a macOS 27-inspired tinted-glass design, consistent controls, accessible focus states and work-area/DPI bounds. Native actions and existing preferences are preserved. See [popover design and validation](MAC-PANELS.md).

- Windows Start is the default application menu as of MAC UI 2.8.7. The custom Launchpad is opt-in; its paged search, virtual folders, read-only Start Menu import and saved layout remain available for rollback. This does not install or modify Windhawk. See [Launchpad usage, import boundaries and validation](LAUNCHPAD.md).

- Desktop z-order uses Explorer's actual `SHELLDLL_DefView` host, not any window named `WorkerW`. It never inserts the full-screen desktop into the topmost band, falls back to the bottom while Explorer is unavailable, and periodically repairs the order without joining another application's input queue. Invisible helper windows are ignored as insertion anchors and predecessors; protected hidden helpers previously caused repeated access-denied errors. The generic widget z-order command cannot elevate this desktop. Keyboard interaction remains enabled.
- The Dock's primary click first resolves the executable and checks live native tray callbacks. WeChat / Weixin / QQ / WeCom use their tray double-click action. A matching live window is the fallback. A detected running process with no safe activation target reports an error instead of launching a duplicate login. Multiple matching tray icons are not guessed. An explicit middle-click “new instance” remains available.
- Settings has a macOS-inspired sidebar, page search, working window controls, grouped content surfaces, unified control tokens, light/dark themes, reduced-motion and reduced-transparency support. Home now shows actual module status rather than upstream marketing. All existing routes remain accessible. Unsaved changes get a close warning; failed saves do not clear the dirty flag.
- Desktop icon sizes: 48 / 64 / 80 / 96 CSS pixels, available in the desktop toolbar and background context menu. The preference is stored alongside the existing layout. Old or invalid values default to 64.
- Desktop and Dock applications share a frameless, rounded-square renderer. Transparent padding is normalized using alpha bounds without stretching the artwork. Square and rounded-square source tiles fill a consistent rounded mask; circular or irregular logos retain their complete mark on a neutral rounded-square plate. Version 2.8.7 removes the extra glass coat and added outer/artwork shadows; icon geometry is unchanged. Local artwork is reused, without downloading replacement icon packs. Ordinary files, folders, and shortcuts to documents/folders retain the existing glass tile without its added glow. Tauri asset CORS is used for alpha analysis; unreadable external artwork uses a safe contained fallback. The Dock checks `.lnk` targets with a read-only native command before applying the application treatment.
- Selected desktop labels no longer have a separate black background. The overall selection highlight and readable text shadow remain.
- Service startup waits for IPC readiness after both Task Scheduler and manual starts. A successful user-approved retry no longer returns the stale error from the first cancelled elevation attempt.
- Program metadata, Settings, welcome content, command-line display name, and NSIS publisher/product metadata are branded MAC UI / JONA. A new original vector mark supplies the settings and Windows installer icons. Original assets and license notices remain in the repository.

## Design and native references

The UI/UX skill informed separation of translucent navigation from readable content, consistent focus/selection states, reduced motion and stable control/icon sizing. Apple provides the visual direction, not a native macOS implementation: [Liquid Glass design introduction](https://developer.apple.com/videos/play/meet-with-apple/201/) and [app icon guidance](https://developer.apple.com/design/human-interface-guidelines/app-icons). Window-order safety follows Microsoft's [SetWindowPos documentation](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowpos).

## Validation — 2026-09-25

- Core binding regeneration: 84 tests passed.
- `cargo check --workspace`: passed.
- `cargo test -p slu-utils --lib`: 14 tests passed, including desktop insertion-order and hidden-helper coverage.
- Frontend regression suite: 23 tests passed, including background activation, repeated clicks, icon bounds/tile classification, icon-size compatibility, tray presentation, new files and stack layout.
- `npm run type-check`: passed with 0 Svelte errors / warnings.
- `npm run build:ui`: passed.
- Debug executable generated; Windows version resources report ProductName `MAC UI`, CompanyName `JONA`.

**Native UI acceptance is partially complete.** Following the user's service approval, the latest debug app and helper service start successfully. Computer Use verified Settings search/navigation, unsaved-close confirmation, continuing editing and reverting a draft. The draft was reverted without saving; desktop remains enabled and wallpaper management remains disabled. Settings minimize/restore and subsequent navigation clicks work while the desktop is active. Desktop application artwork, Dock artwork and unchanged document/folder styling were visually inspected. Selecting a desktop shortcut confirmed its label has no black background.

The read-only `cargo run -p sluhk --example desktop_order_probe` diagnostic reported `desktop_topmost=false`, 20 normal visible windows above the desktop and zero below it, both before and after desktop selection and Settings minimize/restore. This is a session-level result, not proof of every Explorer/recovery scenario.

Dock mouse automation was blocked by Computer Use's target validation: it identified the embedded WebView2 child as another process, including after activation and one fresh-screenshot retry. No WeChat activation click was sent in that test. A user-assisted check has been requested; WeChat background restoration is **not yet marked as runtime passed**. No chat or authentication interaction was performed.

Remaining acceptance checks:

1. Verify Settings save/reopen, minimum size and dark appearance. Normal-size navigation, search, revert and close confirmation have passed.
2. Change icon size, reopen the desktop, verify the preference survives and stack positions/sizes remain unchanged. Inspect rounded and square application artwork alongside unchanged document/folder icons.
3. Stress-test desktop/application switching and Explorer restart recovery separately when safe. Basic desktop selection and Settings minimize/restore/navigation have passed.
4. With the user's already signed-in WeChat running in the tray, click its pinned Dock item, close only its main window and reopen again. Confirm the original session is restored without a new login or new main process. Do not interact with chats or authentication.

## Release packaging — 2026-09-25

The Windows x64 NSIS installer is `release/MAC-UI-2.8.6-x64-Setup.exe` (47,013,785 bytes), with a SHA-256 sidecar and installation notes in the same directory. The installer, main executable and helper executables report ProductName `MAC UI`, CompanyName `JONA`, version `2.8.6`. The public main binary is `mac-ui.exe`; installer shortcuts and service launch resolution use this name. Window/process identification and the default unmanaged-application rule accept both branded and legacy development names.

Packaging validation:

- Production UI and optimized Rust builds, NSIS compilation: passed.
- `cargo check --workspace`: passed; `cargo test -p slu-utils --lib`: **18 passed**.
- Frontend regression suite: **31 passed**; `npm run type-check`: passed, 0 Svelte errors/warnings.
- Internal resource signature verified; all **369** source resources and staged resources match the signed manifest, with no extra staged files.
- Generated installer definitions, publisher, main-binary path, shortcuts, license/notice inclusion and absence of private signing material checked.
- Copied installer SHA-256 matches the build artifact: `d8e8c2aa97d1ceae4aa7da808492e4ca0a32a92831963ac5a00fa5228972092d`.

No installer execution or replacement of the active development environment was performed. These packaging checks do not supersede the native acceptance limitations above or in the Launchpad/popover validation notes. JONA is author/publisher metadata; Windows Authenticode status is **NotSigned**. Original license/attribution files are included, and no upstream signing certificate is used.

## Stability release packaging — 2026-09-27

Installer: `release/2026-09-27/MAC-UI-2.8.6-x64-Setup.exe` (47,063,823 bytes), with a SHA-256 sidecar and installation notes. The prior installer remains unchanged. This build incorporates the desktop layer/focus, display recovery/transparency, widget lifecycle, Dock activation and Launchpad motion repairs documented in `STABILITY-2026-09-27.md`.

- Production UI, optimized main/helper/DLL builds and NSIS packaging succeeded. Two release-only unused-import warnings remain in logger modules; no build errors occurred.
- Packaging-time frontend regression suite: 54 passed; complete type checks: passed, zero Svelte errors/warnings. This follows the 64 main Rust and 36 utility test passes for the included fixes.
- Resource signature verified; source and staged resources both match all 369 manifest entries, with no missing, extra or mismatched files. Production frontend output contains no source maps.
- Fresh generated NSIS definitions and Windows version resources report MAC UI / JONA / 2.8.6; main binary is `mac-ui.exe`. Helper executables, hook DLL, integrity manifest/signature, LICENSE and NOTICE inclusion checked. No private signing material is included.
- Installer SHA-256, verified after copying: `45fbda716a5dbb435e7421c494c7af4dded1fbf008b66571d9908110133acb99`.
- Windows Authenticode status remains **NotSigned**. No installer execution or automatic upgrade was performed; the active debug test process was left running. Previous native acceptance limits still apply.

## Appearance and Dock sizing release — 2026-10-03

Installer: `release/2026-10-03/MAC-UI-2.8.6-x64-Setup.exe` (47,138,450 bytes). The dated directory also contains installation notes and a SHA-256 sidecar; prior installers are retained unchanged.

- Launchpad and shared application icons use the restored September 27 appearance. The subsequent boundary fix removes the clipped outer shadow and explicitly keeps the window gutter transparent; see `LAUNCHPAD.md`.
- Dock overall size is a 16–128 px slider in both Dock and Desktop Management settings. Artwork remains proportionate with long window titles and vertical layouts. Dock transparency/gloss controls remain independent of icon artwork.
- Production UI, optimized binaries/DLL and NSIS packaging passed. Complete frontend regressions: **64 passed**. Type checking: zero Svelte errors/warnings. Two pre-existing release-only unused-import warnings remain.
- All **370** source and staged static resources match the signed manifest, with no extra files. Internal signature verification passed. Embedded Launchpad CSS matches the production output exactly and includes the boundary repair. No frontend source maps or QA fixtures are in the production output; no private signing material is in the installer inputs.
- Generated installer definitions and Windows product metadata confirm **MAC UI / JONA / 2.8.6**, main binary `mac-ui.exe`. SHA-256 verified after copying: `0c62d477d881990c4d25aa83e9fc3688acd066dafd2f0e91d3b750e53d7ff0f4`.
- Windows Authenticode is **NotSigned**. No installer execution or automatic upgrade was performed. Packaging validation does not replace installation or extended native acceptance testing.

## Native Start and icon-edge release — 2.8.7, 2026-10-04

Installer: `release/2026-10-04/MAC-UI-2.8.7-x64-Setup.exe` (47,113,019 bytes). Product manifests, local core-library metadata and lockfiles now use 2.8.7; the installer and main/helper executable version resources all report MAC UI / JONA / 2.8.7. Prior installers are retained unchanged.

- Windows Start is the default; custom Launchpad remains opt-in with its data intact. No Windhawk injection, installation or configuration change is included.
- Added icon-edge/artwork shadows and the extra icon coat are removed. Dock size/transparency/gloss controls and existing geometry remain intact.
- Packaging-time checks: **64 frontend tests and 20 core-library tests passed**, complete type checking passed with zero Svelte errors/warnings. The isolated icon QA fixture's indexed-access type error was corrected and its build rerun successfully; it is not bundled.
- Production frontend, optimized main/helper/DLL builds and NSIS packaging succeeded. Two existing unused-import warnings in logger modules remain. Current Launchpad, desktop and Dock JS/CSS exactly match the assets embedded by the build.
- All **370** source and staged static resources match the signed manifest; internal signature verification passed. Production output has no source maps or QA fixtures; installer inputs contain no private signing material. License and attribution files are included.
- SHA-256 verified after copying: `947cfec3ddac74820c0af627920049e5060665d5d513368b18835ff8b2f95bd5`.
- Windows Authenticode remains **NotSigned**. This package has not been automatically installed. Native Win-key/Dock menu interaction still awaits user confirmation; static/build checks do not replace native acceptance testing.
