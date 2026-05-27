<script lang="ts">
  // Updates + About sections of Settings (P0-018, PRD §10.6.7/§10.6.8).
  //
  // Phase 0 has no auto-update system (FR-056) — we state that plainly and link
  // to the releases page so users can update manually (FR-057). Opening the link
  // uses the OS default browser via the opener plugin; no update check happens.
  import { openUrl } from "@tauri-apps/plugin-opener";

  let { version = "—" }: { version?: string } = $props();

  const REPO_URL = "https://github.com/cesarion161/TLiquid";
  const RELEASES_URL = `${REPO_URL}/releases`;

  function openExternal(url: string) {
    openUrl(url).catch(() => {
      /* best-effort; the URLs are also shown for manual navigation. */
    });
  }
</script>

<div class="section">
  <h2 class="section__title">Updates</h2>
  <p class="hint">
    Automatic updates are not enabled in Phase 0 — TLiquid never checks for
    updates in the background. Visit the releases page to update manually.
  </p>
  <div class="row">
    <button class="btn" onclick={() => openExternal(RELEASES_URL)}>View releases</button>
  </div>
</div>

<div class="section">
  <h2 class="section__title">About</h2>
  <p class="hint">TLiquid v{version}</p>
  <p class="hint">
    An open-source (MIT), macOS-first, bring-your-own-key LLM translator that
    lives in your menu bar.
  </p>
  <div class="row">
    <button class="btn" onclick={() => openExternal(REPO_URL)}>Source on GitHub</button>
  </div>
</div>
