# MAC UI

MAC UI customizations © 2026 JONA. Author and publisher credit: JONA.

This is a modified distribution of [Seelen UI](https://github.com/eythaann/Seelen-UI), originally developed by eythaann
and the Seelen UI contributors. Their copyrights, notices and third-party licenses remain applicable. The project
remains licensed under **AGPL-3.0-or-later**; see [LICENSE](LICENSE). JONA is not represented as the author of the
upstream work.

MAC UI is an independent Windows customization project, not an Apple product or an official Seelen release.
macOS-inspired styling uses original interface code, Windows system fonts and an original MAC UI mark.

Author/publisher metadata is not a Windows Authenticode digital signature. No certificate or trusted-signature status is
claimed.

The public product name is MAC UI. The application uses `macui.exe`, `macui-cli.exe`, `macui-service.exe` and
`macui-hook.dll`, the `@macui/*` runtime namespace, the `@macui/lib` SDK and its own `com.jona.mac-ui` application-data
directory, IPC pipes and scheduled task. Configuration migration retains the original data. New resource bundles use
`.maui`; compatible legacy `.slu` resources can be imported.

MAC UI provides a built-in Liquid Glass interface and Windows system application icons. Settings offer original glass
and enhanced contrast modes. Online accounts, cloud backups, marketplace downloads, telemetry and Discord presence are
disabled. The upstream copyrights, source attribution and license obligations stated above remain applicable.

This custom build uses manual updates. Upstream application update installation is disabled to avoid replacing MAC UI
customizations with the original application. A future MAC UI updater requires its own release endpoint and signing key.

The settings category cards include an adaptation of React Bits SpotlightCard by David Haz. The component retains its
MIT + Commons Clause license in
[src/ui/react/settings/components/react-bits/LICENSE.md](src/ui/react/settings/components/react-bits/LICENSE.md).
Source: https://github.com/DavidHDev/react-bits/tree/main/src/ts-default/Components/SpotlightCard.
