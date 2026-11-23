<script lang="ts">
  import type { ClassValue } from "svelte/elements";
  import InlineSVG from "./InlineSVG.svelte";
  import type { IconName } from "libs/ui/icons";

  interface Props {
    name?: IconName;
    iconName?: IconName;
    class?: ClassValue;
    [key: string]: any;
  }

  let { name, iconName, class: className, ...rest }: Props = $props();

  let filename = $derived(name || iconName);
</script>

{#if filename}
  <InlineSVG {...rest} src={`/icons/${filename}.svg`} class={["macui-icon", className]} />
{/if}

<style>
  @layer svg-icon {
    :global(.macui-icon) {
      height: 1em;
      width: max-content;
      min-width: max-content;
      display: inline-block;

      > :global(svg) {
        vertical-align: middle;
      }
    }
  }
</style>
