<script lang="ts">
  import { invoke, MacUiCommand, Widget } from "@macui/lib";
  import type { WidgetId } from "@macui/lib/types";
  import { Icon } from "libs/ui/svelte/components/Icon";
  import BrightnessControl from "./components/BrightnessControl.svelte";
  import MediaDevices from "./components/MediaDevices.svelte";
  import RadioButtons from "./components/RadioButtons.svelte";
  import { t } from "./i18n";

  function openWindowsSettings() {
    invoke(MacUiCommand.OpenFile, { path: "ms-settings:" });
  }

  function openPowerMenu() {
    invoke(MacUiCommand.TriggerWidget, {
      payload: { id: "@macui/power-menu" as WidgetId },
    });
  }

  $effect(() => {
    Widget.getCurrent().ready();
  });
</script>

<div class={["macui-std-popover", "mac-panel", "mac-frosted-surface", "quick-settings"]}>
  <h1 class="mac-panel-heading">{$t("title")}</h1>
  <RadioButtons />
  <div class="quick-settings-levels">
    <BrightnessControl />
    <MediaDevices />
  </div>

  <div class="quick-settings-footer">
    <button data-skin="transparent" onclick={openWindowsSettings} title={$t("windows_settings")} aria-label={$t("windows_settings")}>
      <Icon iconName="RiSettings4Fill" />
    </button>

    <button data-skin="transparent" onclick={openPowerMenu} title={$t("power")} aria-label={$t("power")}>
      <Icon iconName="IoPower" />
    </button>
  </div>
</div>
