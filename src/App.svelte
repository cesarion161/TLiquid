<script lang="ts">
  import { onMount } from "svelte";
  import { isTauri } from "@tauri-apps/api/core";
  import { info } from "@tauri-apps/plugin-log";
  import { appVersion } from "./lib/tauri";

  let version = $state("…");
  let error = $state<string | null>(null);

  onMount(async () => {
    // The Tauri runtime is only present inside the app's own webview. Opening
    // this page in a regular browser (e.g. http://localhost:1420) has no IPC,
    // so guard rather than throwing a raw TypeError.
    if (!isTauri()) {
      version = "—";
      error =
        "Not running inside the TLiquid app. Launch it with `pnpm tauri dev` and open it from the menu-bar icon — don't open the dev URL in a browser.";
      return;
    }

    try {
      version = await appVersion();
      await info(`TLiquid UI ready (v${version})`);
    } catch (e) {
      error = String(e);
    }
  });
</script>

<main>
  <h1>TLiquid</h1>
  <p class="tagline">BYOK LLM translator · macOS menu-bar utility</p>
  <p class="version">v{version}</p>
  {#if error}
    <p class="error">{error}</p>
  {/if}
  <p class="note">
    Phase 0 foundation scaffold. The manual translation popup is implemented in
    task P0-011.
  </p>
</main>
