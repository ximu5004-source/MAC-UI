<script lang="ts">
  import type { User, WidgetId } from "@seelen-ui/lib/types";
  import { invoke, SeelenCommand, Widget } from "@seelen-ui/lib";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { path } from "@tauri-apps/api";
  import Icon from "libs/ui/svelte/components/Icon/Icon.svelte";
  import { t } from "../i18n";

  interface Props {
    user: User;
  }

  let { user }: Props = $props();

  async function openUserFolder() {
    invoke(SeelenCommand.OpenFile, { path: await path.homeDir() });
  }

  function openOneDrive() {
    if (user.oneDrivePath) {
      invoke(SeelenCommand.OpenFile, { path: user.oneDrivePath });
    }
  }

  function openAccountSettings() {
    invoke(SeelenCommand.OpenFile, { path: "ms-settings:accounts" });
  }

  let openingPower = $state(false);
  async function openPowerMenu() {
    if (openingPower) return;
    openingPower = true;
    try {
      await Widget.self.hide();
      await invoke(SeelenCommand.TriggerWidget, {
        payload: { id: "@seelen/power-menu" as WidgetId },
      });
    } catch (error) {
      console.error("Failed to open power confirmation:", error);
    } finally { openingPower = false; }
  }
</script>

<div class="user-profile-container">
  <div class="user-profile-picture-container">
    {#if user.profilePicturePath}
      <img
        class="user-profile-picture"
        src={convertFileSrc(user.profilePicturePath)}
        alt={user.name}
      />
    {:else}
      <div class="user-profile-picture-fallback">
        <Icon iconName="PiFolderUser" />
      </div>
    {/if}
  </div>

  <div class="user-profile-information">
    <div class="user-profile-name">
      <span>{user.name}</span>
      <button
        data-skin="transparent"
        onclick={openUserFolder}
        title={$t("profile.open_user_folder")}
        aria-label={$t("profile.open_user_folder")}
      >
        <Icon iconName="PiFolderUser" />
      </button>
    </div>

    {#if user.email}
      <div class="user-profile-email">{user.email}</div>
    {/if}

    <div class="user-profile-actions">
      <button data-skin="transparent" onclick={openAccountSettings} title={$t("profile.accounts")} aria-label={$t("profile.accounts")}>
        <Icon iconName="RiSettings3Fill" />
      </button>
      <button data-skin="transparent" onclick={openPowerMenu} disabled={openingPower} title={$t("profile.power_and_session")} aria-label={$t("profile.power_and_session")}>
        <Icon iconName="BiLogOut" />
      </button>
    </div>

    {#if user.oneDrivePath}
      <button data-skin="transparent" onclick={openOneDrive} title={$t("profile.open_onedrive")}>
        <Icon iconName="ImOnedrive" />
        <span>OneDrive</span>
      </button>
    {/if}
  </div>
</div>
