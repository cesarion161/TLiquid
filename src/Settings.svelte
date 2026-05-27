<script lang="ts">
  // Settings view of the panel (not a separate window). The version is passed
  // down from App so this view doesn't re-fetch it.
  let { version = "—" }: { version?: string } = $props();
</script>

<section class="body">
  <p class="tagline">Languages · Shortcuts · Providers · Models</p>
  <p class="version">TLiquid v{version}</p>
  <p class="note">
    Phase 0 foundation scaffold. The settings sections are implemented in tasks
    P0-006 (languages), P0-007 (shortcuts) and P0-009 (providers &amp; models).
  </p>
</section>
