<script lang="ts">
  // Privacy section of Settings (P0-016). Phase 0 sends nothing to TLiquid: no
  // telemetry, no diagnostics upload. The only outbound calls are the BYOK
  // provider requests you trigger. P0-017 expands the privacy safeguards/audit.
  import { writeText } from "@tauri-apps/plugin-clipboard-manager";
  import { diagnostics } from "./lib/tauri";

  let copied = $state(false);
  let error = $state<string | null>(null);

  // Local export only (FR-064/FR-065): gathers non-sensitive metadata and copies
  // it to the clipboard for a bug report. Never uploaded; never includes keys
  // or translated text.
  async function copyDiagnostics() {
    error = null;
    copied = false;
    try {
      await writeText(await diagnostics());
      copied = true;
      // Revert the affordance so it doesn't read "Copied!" indefinitely.
      setTimeout(() => (copied = false), 2000);
    } catch (e) {
      error = `Could not copy diagnostics: ${e}`;
    }
  }
</script>

<div class="section">
  <h2 class="section__title">Privacy</h2>
  <p class="hint">
    TLiquid sends your text only to the provider you choose, and your API keys
    stay in the macOS Keychain. There is no telemetry and no diagnostics upload.
  </p>
  <div class="row">
    <button class="btn" onclick={copyDiagnostics}>Copy diagnostics</button>
    <span class="hint">{copied ? "Copied!" : "Local only — for bug reports"}</span>
  </div>
  {#if error}
    <p class="error">{error}</p>
  {/if}
</div>
