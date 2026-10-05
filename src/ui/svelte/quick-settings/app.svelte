<script lang="ts">
  import { invoke, SeelenCommand, Widget } from "@seelen-ui/lib";
  import type { WidgetId } from "@seelen-ui/lib/types";
  import { Icon } from "libs/ui/svelte/components/Icon";
  import BrightnessControl from "./components/BrightnessControl.svelte";
  import MediaDevices from "./components/MediaDevices.svelte";
  import RadioButtons from "./components/RadioButtons.svelte";
  import { t } from "./i18n";

  function openAppSettings() {
    invoke(SeelenCommand.TriggerWidget, {
      payload: { id: "@seelen/settings" as WidgetId },
    });
  }

  function openPowerMenu() {
    invoke(SeelenCommand.TriggerWidget, {
      payload: { id: "@seelen/power-menu" as WidgetId },
    });
  }

  $effect(() => {
    Widget.getCurrent().ready();
  });
</script>

<div class={["slu-std-popover", "mac-panel", "mac-frosted-surface", "quick-settings"]}>
  <h1 class="mac-panel-heading">{$t("title")}</h1>
  <RadioButtons />
  <div class="quick-settings-levels">
    <BrightnessControl />
    <MediaDevices />
  </div>

  <div class="quick-settings-footer">
    <button data-skin="transparent" onclick={openAppSettings} title={$t("app_settings")} aria-label={$t("app_settings")}>
      <Icon iconName="RiSettings4Fill" />
    </button>

    <button data-skin="transparent" onclick={openPowerMenu} title={$t("power")} aria-label={$t("power")}>
      <Icon iconName="IoPower" />
    </button>
  </div>
</div>
