# MAC UI

English | [简体中文](README.zh-CN.md)

MAC UI is an independent Windows desktop customization project based on [eythaann/Seelen-UI](https://github.com/eythaann/Seelen-UI), maintained by JONA. It adds a macOS-inspired application grid, desktop organization and a consistent frosted-glass interface. JONA maintains the MAC UI modifications; authorship of the upstream work remains with eythaann and the Seelen UI contributors.

The project remains licensed under **AGPL-3.0-or-later**. The upstream application source, original assets and attribution are retained. Private signing materials and build outputs are excluded; upstream CI recipes are archived as inactive references rather than configured as MAC UI release automation. See [LICENSE](LICENSE), [NOTICE](NOTICE.md) and the [preserved upstream introduction](README.upstream.md). MAC UI is neither an Apple product nor an official Seelen UI release.

![MAC UI desktop overview with personal information redacted](documentation/images/mac-ui/desktop-overview.png)
<img width="380" height="620" alt="image" src="https://github.com/user-attachments/assets/128b9800-8205-47be-8bdc-604d24406d56" />
<img width="340" height="333" alt="image" src="https://github.com/user-attachments/assets/742d58d4-95a5-439d-9e7c-6cb8f96ea370" />
<img width="708" height="72" alt="image" src="https://github.com/user-attachments/assets/680b17f5-205d-4fbc-944a-49f0c38aa37e" />

## Features

- **Neutral frosted glass.** Translucent interiors use a neutral coat and native frost, with restrained highlights confined to the edges. The background supplies the visible color rather than a fixed glass tint; text stays sharp and application icons receive no added halo.
- **Adjustable Dock.** Sliders control overall Dock size, surface transparency and gloss independently. Changing size keeps icon artwork proportionate, rather than stretching or cropping it.
- **Desktop stacks.** Organize desktop items in draggable, resizable stacks, with grid snapping, collapse controls and saved layouts.
- **macOS-inspired Launchpad.** Search applications in a paged icon grid and organize them into virtual folders. The application area has no Quick Access section or right sidebar. The custom Launchpad and its standalone Win shortcut are optional.
- **Consistent system panels.** Control Center, Wi-Fi, Bluetooth, keyboard selection, notifications, system tray, audio controls, calendar, account and power panels share the same material and readable controls.

### Desktop organization

![Desktop stacks with personal application and file information redacted](documentation/images/mac-ui/desktop-stacks.png)

Desktop Management contains the organizer switch and appearance controls. Its Dock controls adjust the same preferences as the Dock settings page.

![MAC UI Desktop Management settings](documentation/images/mac-ui/desktop-settings.png)

### Dock settings

The Dock settings page provides the overall-size slider, position and layout controls. Glass transparency and gloss are configured in Desktop Management. These settings affect the Dock surface and layout, not the colors of application artwork.

![MAC UI Dock size and material settings](documentation/images/mac-ui/dock-settings.png)

### Launchpad

![MAC UI application grid with personal application information redacted](documentation/images/mac-ui/launchpad.png)

Enable the custom Launchpad and its shortcut in Settings to use a standalone Win press. It triggers on release, supports both Windows keys and does not treat Win combinations such as Win+E or Win+D as a Launchpad press. Saved shortcut overrides remain respected; a fresh configuration leaves the native Windows Start menu available until the custom Launchpad is enabled. See the [Launchpad documentation](documentation/LAUNCHPAD.md).

### Control Center and session controls

![MAC UI Control Center](documentation/images/mac-ui/control-center.png)

![MAC UI power and session panel with account information redacted](documentation/images/mac-ui/power-session.png)

Power and session controls use a separate sheet with Cancel-first confirmations. See the [panel documentation](documentation/MAC-PANELS.md) for material, accessibility and validation details.

The screenshots illustrate the interface. Personal application, file, folder and account information, tray details and unrelated background content are covered by opaque masks; every pixel outside these masks is preserved. The Desktop Management and Dock settings screenshots contain no private data and are shown unchanged.

## Changes in 3.0.1

Version 3.0.1 fixes the Win shortcut workflow without changing the accepted 3.0 visual design:

- Adds an explicit Launchpad enable switch alongside its shortcut settings and accepts standalone Win when editing the binding.
- Synchronizes initial and file-based shortcut settings, and keeps keyboard capture alive across disable and re-enable operations.
- Isolates shortcut recording, completion and cancellation by request identity so an older recording cannot cancel a newer one.

The **Genie minimization animation remains disabled and experimental**. Normal minimization uses Windows; MAC UI does not currently provide an accepted replacement for macOS's Genie effect. See [Genie status and limitations](documentation/GENIE-MINIMIZE.md).

## Installation and updates

Use the **Windows x64** installer and matching checksum from the [MAC UI 3.0.1 release page](https://github.com/ximu5004-source/MAC-UI/releases/tag/v3.0.1). Keep Microsoft Edge and the WebView2 runtime installed. Upstream Seelen UI downloads, Microsoft Store packages and Winget entries do not contain the MAC UI modifications.

**The MAC UI installer is not Authenticode code-signed.** Windows may display an unknown-publisher warning. Verify the repository, release notes and SHA-256 checksum before deciding whether to install. Product metadata naming JONA is not a trusted digital signature; internal resource signatures serve a different purpose.

Updates are currently manual. Upstream automatic application updates are disabled to avoid replacing MAC UI with Seelen UI. Existing compatibility identifiers and application-data paths are retained; back up your configuration before upgrading.

MAC UI does not install or configure Windhawk and does not replace Windows system files. Its own application and helper components provide the customization. Seelen accounts and the resource marketplace remain upstream services, not services operated by MAC UI.

## Development

The project uses Rust and Tauri for native components, with TypeScript and Svelte or React for the interface. A Windows development environment needs Node.js and npm, Deno, the Rust MSVC toolchain, Microsoft C++ Build Tools and the Windows SDK, plus WebView2.

Run these commands from the repository root after preparing the toolchains:

```powershell
npm install
npm run build:ui
npm run type-check
cargo check --workspace
npm run dev
```

`npm install` also builds the local core library through the repository's preinstall script. `npm run dev` builds debug native binaries and starts the Tauri development workflow. Use `cargo check` for routine Rust validation, rather than a release build.

Production packaging has additional integrity-signing requirements in [src/build.rs](src/build.rs). Private signing material is not distributed in this repository, and setting up your own integrity trust requires corresponding configuration. The commands above are development entry points, not a promise to reproduce an official signed installer from a clean checkout. See [AGENTS.md](AGENTS.md) and the [customization notes](documentation/MAC-UI.md) before changing native or packaging behavior.

## Documentation and attribution

- [MAC UI customization and release notes](documentation/MAC-UI.md)
- [Launchpad](documentation/LAUNCHPAD.md)
- [System panels](documentation/MAC-PANELS.md)
- [Genie prototype status](documentation/GENIE-MINIMIZE.md)
- [Upstream project](https://github.com/eythaann/Seelen-UI) and [preserved upstream introduction](README.upstream.md)
- [License](LICENSE) and [copyright notices](NOTICE.md)

When reporting an issue, include the MAC UI version, Windows version, display scale and steps to reproduce it. Redact account names, network details and personal files from screenshots. Report MAC UI customization issues in [this repository](https://github.com/ximu5004-source/MAC-UI/issues); do not present this fork as an upstream release.
