<script lang="ts">
  // Privacy section of Settings (P0-016 + P1-007). Phase 0 sends nothing to
  // TLiquid: no telemetry, no diagnostics upload. The only outbound calls are
  // the BYOK provider requests you trigger. The diagnostics bundle (metadata +
  // recent log tail + error summary) can be copied or saved to a file for a bug
  // report — local only, never uploaded, and never contains keys or text.
  import { writeText } from "@tauri-apps/plugin-clipboard-manager";
  import { revealItemInDir } from "@tauri-apps/plugin-opener";
  import { diagnostics, exportDiagnostics } from "./lib/tauri";

  let copied = $state(false);
  let saved = $state(false);
  let error = $state<string | null>(null);

  async function copyDiagnostics() {
    error = null;
    copied = false;
    try {
      await writeText(await diagnostics());
      copied = true;
      setTimeout(() => (copied = false), 2000);
    } catch (e) {
      error = `Could not copy diagnostics: ${e}`;
    }
  }

  // Save the bundle to a file and reveal it in Finder so the user can attach it.
  async function saveDiagnostics() {
    error = null;
    saved = false;
    try {
      const path = await exportDiagnostics();
      saved = true;
      setTimeout(() => (saved = false), 2000);
      try {
        await revealItemInDir(path);
      } catch {
        /* the file is written; revealing is best-effort. */
      }
    } catch (e) {
      error = `Could not save diagnostics: ${e}`;
    }
  }
</script>

<div class="section">
  <h2 class="section__title">Privacy</h2>
  <p class="hint">
    TLiquid sends your text only to the provider you choose, and your API keys
    stay in the macOS Keychain. There is no telemetry and no diagnostics upload.
  </p>
  <p class="hint">
    The diagnostics bundle includes app/OS info, your settings shape, and a tail
    of the local log — never keys, translated text, or provider responses.
  </p>
  <div class="row">
    <button class="btn" onclick={copyDiagnostics}>Copy diagnostics</button>
    <button class="btn" onclick={saveDiagnostics}>Save to file…</button>
    <span class="hint grow">
      {#if copied}Copied!{:else if saved}Saved{:else}Local only — for bug reports{/if}
    </span>
  </div>
  {#if error}
    <p class="error">{error}</p>
  {/if}
</div>
