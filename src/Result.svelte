<script lang="ts">
  // Output pane of the translate view (a region in the panel, not its own
  // window). This is the UI foundation (P0-003): a scrollable output area, a
  // Copy button, and the "Press Enter to copy" affordance. Real result
  // rendering, clipboard copy, and Enter-to-copy are wired in P0-012.
</script>

<div class="field grow">
  <!-- The output is a non-form region, so it gets a plain label paired with
       aria-labelledby (a <label for> only binds to form controls). aria-live
       lets screen readers announce the translation when P0-012 fills it. -->
  <span id="translation-label" class="label">Translation</span>
  <div
    class="output hint"
    role="region"
    aria-labelledby="translation-label"
    aria-live="polite"
  >
    Translation output appears here.
  </div>
  <div class="row">
    <button class="btn" disabled>Copy</button>
    <span class="hint">Press Enter to copy</span>
  </div>
</div>
