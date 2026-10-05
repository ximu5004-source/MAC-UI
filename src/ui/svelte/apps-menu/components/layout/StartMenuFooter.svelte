<script lang="ts">
  import { t } from "../../i18n";
  import { invoke, SeelenCommand, Widget } from "@seelen-ui/lib";
  import type { WidgetId } from "@seelen-ui/lib/types";
  import { globalState } from "../../state/mod.svelte";
  import { StartDisplayMode } from "../../constants";
  import { Icon } from "libs/ui/svelte/components/Icon";
  import { convertFileSrc } from "@tauri-apps/api/core";

  async function openUserMenu() {
    await Widget.self.hide();
    await invoke(SeelenCommand.TriggerWidget, {
      payload: { id: "@seelen/user-menu" as WidgetId },
    });
  }

  function openAppSettings() {
    invoke(SeelenCommand.TriggerWidget, {
      payload: { id: "@seelen/settings" as WidgetId },
    });
  }

  async function openPowerMenu() {
    // Dismiss the source before showing the sheet: another WebView's sharp
    // grid must not be mistaken for the new panel's own content.
    await Widget.self.hide();
    await invoke(SeelenCommand.TriggerWidget, {
      payload: { id: "@seelen/power-menu" as WidgetId },
    });
  }
</script>

<div class="apps-menu-footer">
  <div class="apps-menu-footer-left">
    <button data-skin="transparent" class="user-profile" aria-label={$t("launchpad.user_menu")} title={$t("launchpad.user_menu")} onclick={openUserMenu}>
      {#if globalState.user.profilePicturePath}
        <img
          class="user-profile-picture"
          src={convertFileSrc(globalState.user.profilePicturePath)}
          alt=""
        />
      {:else}
        <Icon class="user-profile-picture" iconName="PiFolderUser" />
      {/if}
      <span class="user-profile-name">{globalState.user.name}</span>
    </button>
  </div>

  <span class="launchpad-footer-hint">{$t("launchpad.keyboard_hint")}</span>

  <div class="apps-menu-footer-right">
    <button data-skin="transparent" onclick={openAppSettings} aria-label={$t("app_settings")} title={$t("app_settings")}>
      <Icon iconName="RiSettings4Fill" />
    </button>

    <button data-skin="transparent" onclick={openPowerMenu} aria-label={$t("power_menu")} title={$t("power_menu")}>
      <Icon iconName="IoPower" />
    </button>

    <button
      data-skin="transparent"
      aria-label={globalState.displayMode === StartDisplayMode.Fullscreen ? $t("launchpad.windowed") : $t("launchpad.fullscreen")}
      title={globalState.displayMode === StartDisplayMode.Fullscreen ? $t("launchpad.windowed") : $t("launchpad.fullscreen")}
      onclick={() => {
        globalState.displayMode =
          globalState.displayMode === StartDisplayMode.Normal
            ? StartDisplayMode.Fullscreen
            : StartDisplayMode.Normal;
      }}
    >
      <Icon
        iconName={globalState.displayMode === StartDisplayMode.Fullscreen
          ? "IoContract"
          : "IoExpand"}
      />
    </button>
  </div>
</div>
