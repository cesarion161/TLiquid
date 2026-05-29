<script lang="ts">
  // Startup section of Settings (P1-001, FR-053/054/055). A single toggle:
  // launch TLiquid at login. `startup` is server-authoritative — `setLaunchAtLogin`
  // persists it (enabled + the one-time consent) and applies it to the OS — so
  // this view never round-trips its (separate) settings copy for startup; it
  // only mutates `settings.startup` locally for display.
  import { onMount } from "svelte";
  import { isTauri } from "@tauri-apps/api/core";
  import { setLaunchAtLogin, isLaunchAtLogin, type Settings } from "./lib/tauri";

  let { settings }: { settings: Settings } = $props();

  let error = $state<string | null>(null);

  onMount(async () => {
    if (!isTauri()) return;
    // Reflect the real OS registration for an accurate toggle (this view's
    // settings copy can be stale, e.g. after consenting on the translate view).
    // Display-only: no persist — startup is owned by setLaunchAtLogin.
    try {
      settings.startup.enabled = await isLaunchAtLogin();
    } catch {
      /* fall back to the stored value */
    }
  });

  async function toggle(e: Event) {
    const enabled = (e.currentTarget as HTMLInputElement).checked;
    error = null;
    settings.startup.enabled = enabled; // optimistic; command persists it
    try {
      await setLaunchAtLogin(enabled);
    } catch (err) {
      settings.startup.enabled = !enabled; // revert if the OS rejected it
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
    <span class="grow">Launch T·Liquid at login</span>
  </label>
  <p class="hint">Starts in the menu bar, ready for the translate hotkey.</p>

  {#if error}
    <p class="error">{error}</p>
  {/if}
</div>
