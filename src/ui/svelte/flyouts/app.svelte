<script lang="ts">
  import { Widget } from "@macui/lib";
  import { NotificationsMode } from "@macui/lib/types";

  import { state as gState } from "./state/mod.svelte";
  import { RendererState } from "./state/placement.svelte";
  import { ConfigState } from "./state/config.svelte";
  import { debounce } from "lodash";
  import Notification from "../notifications/components/Notification.svelte";
  import CommunicationNotification from "../notifications/components/CommunicationNotification.svelte";
  import { NotificationFlyoutTracker } from "libs/ui/svelte/utils/notificationCenter";
  import Workspace from "./app/Workspace.svelte";
  import MediaDevices from "./app/MediaDevices.svelte";
  import MediaPlaying from "./app/MediaPlaying.svelte";
  import Brightness from "./app/Brightness.svelte";
  import ShortcutsState from "./app/ShortcutsState.svelte";
  import { didVolumeChange } from "./volumeChange";

  $effect(() => {
    Widget.getCurrent().ready();
  });

  let lastChanged = $state<string | null>(null);
  let orientation = $derived(
    ["left", "right"].includes(ConfigState.config.placement) ? "vertical" : "horizontal",
  );

  let output = $derived.by(() => {
    return gState.mediaOutputs.find((o) => o.isDefaultMultimedia);
  });

  let recomendedPlayer = $derived.by(() => {
    return gState.mediaPlaying.find((p) => p.default);
  });

  let vd = $derived.by(() => {
    if (RendererState.primary) {
      return gState.workspaces?.monitors[RendererState.primary.id] ?? null;
    }
    return null;
  });

  let activeWorkspaceData = $derived.by(() => {
    return vd?.workspaces
      .flatMap((row, rowIdx) => row.flatMap((workspace, colIdx) => ({ ...workspace, rowIdx, colIdx })))
      .find((workspace) => workspace.id == vd.active_workspace);
  });

  const notificationTracker = new NotificationFlyoutTracker();
  let notificationKey = $state<string | null>(null);
  let notification = $derived(gState.notifications.find((item) => item.key === notificationKey) ?? null);
  let isDnd = $derived(gState.notificationsMode !== NotificationsMode.All);
  const popupPermission = $derived(
    !ConfigState.config.showNotifications ? false : gState.notificationsModePending ? null : !isDnd,
  );
  let volume = $derived(output?.volume || 0);
  let muted = $derived(output?.muted || false);
  let playingTitle = $derived(recomendedPlayer?.title);
  let brightnessLevel = $derived(gState.brightness?.currentBrightness);
  let activeWorkspace = $derived(vd?.active_workspace);
  let shortcutsPaused = $derived(gState.shortcutsPaused);

  $effect(() => {
    // Only a visible notification explicitly requests the topmost layer; the
    // remaining flyouts keep their existing placement and activation behavior.
    RendererState.notificationShowing = lastChanged === "notification" && !!notification;
  });

  // svelte-ignore state_referenced_locally
  const prev = {
    volume,
    muted,
    playingTitle,
    brightnessLevel,
    activeWorkspace,
    shortcutsPaused,
  };

  const hideWithDelay = $derived(
    debounce(() => {
      RendererState.showing = false;
    }, ConfigState.config.timeToShow * 1000),
  );

  $effect(() => {
    let somethingChanged = false;

    // Guard with `output` so we never show an empty flyout when the device is removed.
    if (
      ConfigState.config.showVolumeChange &&
      didVolumeChange(prev, output)
    ) {
      lastChanged = "mediaDevices";
      somethingChanged = true;
    }

    // Guard with `playing` so we never show an empty flyout when the player is closed.
    if (
      ConfigState.config.showMediaPlayerChange &&
      prev.playingTitle !== playingTitle &&
      recomendedPlayer &&
      !isDnd
    ) {
      lastChanged = "mediaPlaying";
      somethingChanged = true;
    }

    // Guard with `gState.brightness` so we never show an empty flyout when brightness is unavailable.
    if (
      ConfigState.config.showBrightnessChange &&
      prev.brightnessLevel !== brightnessLevel &&
      gState.brightness
    ) {
      lastChanged = "brightness";
      somethingChanged = true;
    }

    // Guard with `activeWorkspaceData` so we never show an empty flyout on workspace edge cases.
    if (
      ConfigState.config.showWorkspaceChange &&
      prev.activeWorkspace !== activeWorkspace &&
      activeWorkspaceData
    ) {
      lastChanged = "workspace";
      somethingChanged = true;
    }

    const newNotification = notificationTracker.consume(
      gState.notifications,
      Date.now(),
      popupPermission,
      gState.notificationSourcesReady,
      gState.notificationKnownKeys,
    );
    if (newNotification) {
      notificationKey = newNotification.key;
      lastChanged = "notification";
      somethingChanged = true;
    }

    if (prev.shortcutsPaused !== shortcutsPaused && shortcutsPaused !== null) {
      lastChanged = "shortcuts";
      somethingChanged = true;
    }

    if (somethingChanged) {
      RendererState.showing = true;
      hideWithDelay();
    }

    // Hide immediately when the currently displayed flyout loses its data source.
    if (
      (lastChanged === "mediaPlaying" && !recomendedPlayer) ||
      (lastChanged === "notification" && (!notification || isDnd || !ConfigState.config.showNotifications))
    ) {
      hideWithDelay.cancel();
      RendererState.showing = false;
      lastChanged = null;
    }

    prev.volume = volume;
    prev.muted = muted;
    prev.playingTitle = playingTitle;
    prev.brightnessLevel = brightnessLevel;
    prev.activeWorkspace = activeWorkspace;
    prev.shortcutsPaused = shortcutsPaused;
  });
</script>

<div
  class="flyout mac-frosted-surface"
  data-placement={ConfigState.config.placement}
  data-showing={RendererState.showing}
  aria-live={lastChanged === "notification" ? "polite" : "off"}
  aria-atomic={lastChanged === "notification" ? "true" : undefined}
>
  {#if lastChanged === "notification" && notification}
    {#if notification.source === "windows"}
      <Notification notification={notification.notification} />
    {:else}
      <CommunicationNotification notification={notification.notification} />
    {/if}
  {/if}

  {#if lastChanged === "workspace" && activeWorkspaceData}
    <Workspace workspace={activeWorkspaceData} />
  {/if}

  {#if lastChanged === "mediaDevices" && output}
    <MediaDevices {output} {orientation} />
  {/if}

  {#if lastChanged === "brightness" && gState.brightness}
    <Brightness brightness={gState.brightness} {orientation} />
  {/if}

  {#if lastChanged === "mediaPlaying" && recomendedPlayer}
    <MediaPlaying playing={recomendedPlayer} />
  {/if}

  {#if lastChanged === "shortcuts" && shortcutsPaused !== null}
    <ShortcutsState paused={shortcutsPaused} />
  {/if}
</div>

<style>
  .flyout[data-showing="false"] {
    opacity: 0;
  }
</style>
