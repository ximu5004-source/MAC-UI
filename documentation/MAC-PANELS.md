# MAC UI system popovers

## Current material and acceptance — MAC UI 3.0.0, 2026-10-05

The user accepted the restarted test build and requested packaging. Built-in glass panels now opt into a neutral 10% coat, real-time native 18 CSS px frost and a hollow edge highlight. Calendar/account use 28 px corners; the session sheet uses 32 px. The older 24 px/strong-tint design described below is historical, not the current opted-in material.

Typography, focus, single-owner scrolling and Cancel-first power confirmations remain consistent. Launchpad/account are hidden before opening power, so their sharp content cannot bleed underneath. No real power, radio, authentication or notification action was executed for visual acceptance. See `GLASS-PANELS-QA-2026-10-04.md` for scoped evidence and remaining platform limitations.

The account menu, system tray, keyboard selector, Bluetooth, Wi-Fi, sound/media,
notifications and Control Center now share the MAC UI glass design language.
Visual reference: Apple's [Modernize your AppKit app — WWDC26](https://developer.apple.com/videos/play/wwdc2026/289/),
including the macOS 27 refinements to Liquid Glass, concentric corners and restrained interaction feedback.
This is a Windows/WebView implementation, not Apple's native material renderer.

## Design and compatibility

- Shared opt-in stylesheet: `libs/ui/svelte/styles/mac-panels.css`. The `.mac-panel`
  marker limits it to these eight widgets. A root selector deliberately outranks
  the legacy theme, which the widget runtime injects after the frontend bundle.
- Consistent 24 px outer corners, 12–18 px inner cards, neutral tinted surfaces,
  blue active glyphs, pill sliders, readable notification cards and keyboard focus.
- Light/dark color-scheme tokens, reduced-transparency, reduced-motion,
  higher-contrast and forced-color fallbacks. CSS backdrop filtering cannot be
  assumed to sample native windows behind a transparent WebView; a strong tint
  supplies a readable fallback. There is no wallpaper capture or new wallpaper dependency.
- Work-area bounds use native monitor dimensions and DPI, not the popup's own
  viewport height. This avoids the auto-size shrinking feedback loop. Long lists
  scroll, while notification header/footer and tray header remain accessible.
- Existing native routes and commands remain: all tray entries (including hidden
  icons), single/double/right/middle clicks, keyboard switching, pairing, network
  connection, default audio roles, media actions, notification replies and account actions.
- Account actions no longer overlap the avatar. MAC UI folder labels replace
  upstream product wording without changing paths. Several Chinese control labels
  were clarified. Notification and directory child controls no longer bubble Enter
  or Space into the parent action.
- No desktop layout migration, icon changes, wallpaper setting changes, certificate
  changes or installer packaging are part of this revision.

The UI/UX skill informed contrast, restrained glass layers, consistent control sizes,
keyboard focus and accessibility fallbacks. No downloaded UI artwork is required.

## Validation — 2026-09-25

- `npm run type-check`: passed, 0 Svelte errors/warnings.
- `npm run build:ui`: passed.
- All frontend `*.test.ts` files under `libs/ui` and `src/ui`: **31 passed**.
  New tests cover work-area/DPI conversion, invalid inputs and tiny displays.
- Browser inspection used the **actual built components plus the compiled legacy
  theme**, with an isolated native bridge. All eight panels rendered without
  horizontal overflow in the 200% DPI fixture. Light/dark, empty devices/tray/
  notifications, short work areas, long lists and Wi-Fi password expansion were checked.
- A 40-entry tray retained every item and its last item was reachable and emitted
  the expected native action request. Input method selection updated the active state.
  Control Center's test radio updated its state and ArrowRight changed test volume
  from 36% to 37%. Directory expansion by keyboard did not emit an open-file request.
- Typing Space/Enter in a notification reply emitted **no** activation. Reply emitted
  one activation with the input data; keyboard dismissal emitted only one close
  request and removed the intended notification.
- The development application was restarted. Windows Computer Use captured the
  real account popover with the new styling. Other transient popovers closed or
  changed before capture, so full native acceptance of all eight is **not claimed**.
  Native radio toggles, pairing, network connections, logout/power and real notification
  replies/deletions were deliberately not exercised.

## Reproduce the isolated UI fixture

Run from the repository root:

```powershell
npm run build:ui
npx --yes --package=sass@1.93.2 sass scripts/qa/mac-panels-theme.scss target/qa/mac-panels-theme.css --no-source-map
node scripts/qa/mac-panels-server.mjs
```

Open `http://127.0.0.1:3582/` or a panel such as
`http://127.0.0.1:3582/svelte/quick-settings/index.html`.
Query flags: `dark`, `empty`, `short` (480 px work area), `dpi` (200%),
`long` (40 tray entries), `en`. The bridge uses synthetic content; all native
actions are recorded in the visible status line, never executed. Native icon-pack
extraction is stubbed, so missing artwork in this fixture is not an icon extraction test.
The server binds only to localhost and is not part of the production build.

Sass may report deprecations from existing upstream theme syntax; it is a QA-only
compiler and does not change the project's dependency manifest or lockfile.
