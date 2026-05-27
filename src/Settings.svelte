<script lang="ts">
  // Settings view of the panel (not a separate window). The version is passed
  // down from App so this view doesn't re-fetch it.
  //
  // P0-003 lays out the section structure (PRD §10.6); the interactive content
  // of each section is filled by later tasks: Languages (P0-006), Shortcuts
  // (P0-007), Providers & Models (P0-009), Output/Privacy (P0-017), and
  // Updates/About (P0-018).
  let { version = "—" }: { version?: string } = $props();
</script>

<section class="body">
  <div class="section">
    <h2 class="section__title">Languages</h2>
    <p class="hint">Primary, secondary, and additional target languages.</p>
  </div>

  <div class="section">
    <h2 class="section__title">Shortcuts</h2>
    <p class="hint">Global hotkeys for primary, secondary, and manual translation.</p>
  </div>

  <div class="section">
    <h2 class="section__title">Providers</h2>
    <p class="hint">API keys and connection status for each LLM provider.</p>
  </div>

  <div class="section">
    <h2 class="section__title">Models</h2>
    <p class="hint">Default provider and model used for translation.</p>
  </div>

  <div class="section">
    <h2 class="section__title">Output</h2>
    <p class="hint">What happens with a translation result.</p>
  </div>

  <div class="section">
    <h2 class="section__title">Privacy</h2>
    <p class="hint">TLiquid sends text only to your chosen provider. No telemetry.</p>
  </div>

  <div class="section">
    <h2 class="section__title">Updates</h2>
    <p class="hint">Automatic updates are not enabled in Phase 0.</p>
  </div>

  <div class="section">
    <h2 class="section__title">About</h2>
    <p class="hint">TLiquid v{version}</p>
  </div>
</section>
