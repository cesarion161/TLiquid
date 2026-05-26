<script lang="ts">
  import { onMount } from "svelte";
  import { isTauri } from "@tauri-apps/api/core";
  import { appVersion } from "./lib/tauri";

  let version = $state("…");

  onMount(async () => {
    if (!isTauri()) {
      version = "—";
      return;
    }
    try {
      version = await appVersion();
    } catch {
      version = "unknown";
    }
  });
</script>

<main>
  <h1>Settings</h1>
  <p class="tagline">Languages · Shortcuts · Providers · Models</p>
  <p class="version">TLiquid v{version}</p>
  <p class="note">
    Phase 0 foundation scaffold. The settings sections are implemented in tasks
    P0-006 (languages), P0-007 (shortcuts) and P0-009 (providers &amp; models).
  </p>
</main>
