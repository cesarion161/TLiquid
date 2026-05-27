<script lang="ts">
  // Appearance section of Settings (P2-012). Toggles the macOS panel
  // translucency ("Liquid Glass"). `set_translucency` both persists the
  // preference and applies the vibrancy to the live window; we also flip the
  // body class (via onChange) so the panel background goes transparent to reveal
  // it. Respects the system "Reduce transparency" setting (handled by AppKit).
  import { setTranslucency, type Settings } from "./lib/tauri";

  let {
    settings,
    // Called after a successful toggle so App can flip the body `.translucent`
    // class that makes the panel background transparent for the vibrancy.
    onChange = () => {},
  }: {
    settings: Settings;
    onChange?: (enabled: boolean) => void;
  } = $props();

  let busy = $state(false);
  let error = $state<string | null>(null);

  async function toggle(enabled: boolean) {
    if (busy) return;
    busy = true;
    error = null;
    try {
      await setTranslucency(enabled); // persists ui.translucent + applies vibrancy
      settings.ui.translucent = enabled; // reflect in the in-memory settings
      onChange(enabled);
    } catch (e) {
      error = `Could not change translucency: ${e}`;
    } finally {
      busy = false;
    }
  }
</script>

<div class="section">
  <h2 class="section__title">Appearance</h2>
  <label class="row check">
    <input
      type="checkbox"
      checked={settings.ui.translucent}
      disabled={busy}
      onchange={(e) => toggle(e.currentTarget.checked)}
    />
    <span>Translucent panel background</span>
  </label>
  <p class="hint">
    A frosted “glass” look behind the panel (macOS). Turn it off for a solid
    background. Automatically becomes solid when macOS “Reduce transparency” is on.
  </p>
  {#if error}
    <p class="error">{error}</p>
  {/if}
</div>

<style>
  /* Checkbox + label row: align the box with the text, normal cursor. */
  .check {
    cursor: pointer;
    gap: var(--tl-sp-2);
  }
  .check input {
    margin: 0;
  }
</style>
