<script lang="ts">
  // Startup section of Settings (P1-001, FR-053/054/055). A single toggle:
  // launch TLiquid at login. The setting is persisted (config), and applied to
  // the OS via the autostart command; toggling it also marks the one-time
  // consent as shown so the first-run prompt won't reappear.
  import { onMount } from "svelte";
  import { isTauri } from "@tauri-apps/api/core";
  import { setLaunchAtLogin, isLaunchAtLogin, type Settings } from "./lib/tauri";

  let { settings, onChange }: { settings: Settings; onChange: () => void } =
    $props();

  let error = $state<string | null>(null);

  onMount(async () => {
    if (!isTauri()) return;
    // Reflect the real OS registration in case it drifted from the stored
    // setting (e.g. a hand-edited file, or a failed reconcile).
    try {
      const real = await isLaunchAtLogin();
      if (real !== settings.startup.enabled) {
        settings.startup.enabled = real;
        onChange();
      }
    } catch {
      /* fall back to the stored value */
    }
  });

  async function toggle(e: Event) {
    const enabled = (e.currentTarget as HTMLInputElement).checked;
    settings.startup.enabled = enabled;
    settings.startup.prompted = true; // engaging the toggle counts as consent
    error = null;
    onChange();
    try {
      await setLaunchAtLogin(enabled);
    } catch (err) {
      // Revert the UI if the OS rejected the change.
      settings.startup.enabled = !enabled;
      onChange();
      error = `Could not change launch-at-login: ${err}`;
    }
  }
</script>

<div class="section">
  <h2 class="section__title">Startup</h2>

  <label class="row" style="cursor: pointer;">
    <input
      type="checkbox"
      checked={settings.startup.enabled}
      onchange={toggle}
    />
    <span class="grow">Launch TLiquid at login</span>
  </label>
  <p class="hint">Starts in the menu bar, ready for the translate hotkey.</p>

  {#if error}
    <p class="error">{error}</p>
  {/if}
</div>
