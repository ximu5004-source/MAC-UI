<script lang="ts">
  import { invoke, MacUiCommand } from "@macui/lib";
  import { t } from "../i18n/index.ts";
</script>

<button
  class="ft-corner-button"
  onclick={() => invoke(MacUiCommand.ShowDesktop)}
  title={$t("show_desktop")}
></button>
