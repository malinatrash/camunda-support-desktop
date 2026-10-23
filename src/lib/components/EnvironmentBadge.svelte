<script lang="ts">
  import type { Environment } from "../types";
  import { ENVIRONMENT_LABEL, environmentTone } from "../format";

  let { environment }: { environment: Environment } = $props();
</script>

<span class="pill pill-{environmentTone(environment)}">{ENVIRONMENT_LABEL[environment].short}</span>
