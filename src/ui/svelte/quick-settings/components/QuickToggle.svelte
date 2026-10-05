<script lang="ts">
  import Icon from "libs/ui/svelte/components/Icon/Icon.svelte";
  import type { IconName } from "libs/ui/icons";
  import { t } from "../i18n";

  interface Props {
    icon: IconName;
    label: string;
    status: string;
    enabled: boolean;
    disabled?: boolean;
    onToggle: () => Promise<unknown>;
    onDetails?: () => Promise<unknown>;
  }

  let { icon, label, status, enabled, disabled = false, onToggle, onDetails }: Props = $props();
  let pending = $state(false);
  let failed = $state(false);

  async function run(action: () => Promise<unknown>) {
    if (pending || disabled) return;
    pending = true;
    failed = false;
    try {
      await action();
    } catch (error) {
      console.error(error);
      failed = true;
    } finally {
      pending = false;
    }
  }
</script>

<div class="quick-toggle" data-enabled={enabled} data-pending={pending} data-disabled={disabled} aria-busy={pending || disabled}>
  <div class="quick-toggle-pill" data-split={!!onDetails}>
    <button
      class="quick-toggle-action"
      aria-label={`${label} — ${status}`}
      aria-pressed={enabled}
      disabled={disabled || pending}
      onclick={() => run(onToggle)}
    >
      <span class="quick-toggle-glyph" aria-hidden="true"><Icon iconName={icon} /></span>
    </button>
    {#if onDetails}
      <button
        class="quick-toggle-details"
        aria-label={$t("open_details", { name: label })}
        title={$t("open_details", { name: label })}
        disabled={disabled || pending}
        onclick={() => run(onDetails!)}
      >
        <Icon iconName="IoChevronForward" aria-hidden="true" />
      </button>
    {/if}
  </div>
  <span class="quick-toggle-label">{label}</span>
  <span class="quick-toggle-status" class:failed role={failed ? "alert" : undefined}>
    {failed ? $t("action_failed") : pending || disabled ? $t("updating") : status}
  </span>
</div>
