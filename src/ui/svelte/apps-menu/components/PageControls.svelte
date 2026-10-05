<script lang="ts">
  import { t } from "../i18n";
  import { Icon } from "libs/ui/svelte/components/Icon";
  let { page, count, disabled = false, onchange }: { page: number; count: number; disabled?: boolean; onchange: (page: number) => void } = $props();
</script>

<nav class="launchpad-pages" aria-label={$t("launchpad.pages")}>
  <button class="page-arrow" disabled={disabled || page === 0} aria-label={$t("launchpad.previous_page")} onclick={() => onchange(page - 1)}>
    <Icon iconName="FiChevronLeft" />
  </button>
  <div class="page-dots">
    {#each Array(count) as _, index}
      <button class="page-dot" {disabled} class:current={page === index} aria-current={page === index ? "page" : undefined}
        aria-label={$t("launchpad.go_to_page", { page: String(index + 1) })} onclick={() => onchange(index)}><span></span></button>
    {/each}
  </div>
  <button class="page-arrow" disabled={disabled || page >= count - 1} aria-label={$t("launchpad.next_page")} onclick={() => onchange(page + 1)}>
    <Icon iconName="FiChevronRight" />
  </button>
  <span class="sr-only" aria-live="polite">{$t("launchpad.page_status", { page: String(page + 1), count: String(count) })}</span>
</nav>
