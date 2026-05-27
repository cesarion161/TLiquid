<script lang="ts">
  // Output pane of the translate view (P0-011/P0-012). Presentational: the
  // parent owns the translation state and the clipboard call; this renders the
  // output, a Copy button, the "Press Enter to copy" affordance, and errors.
  let {
    output = null,
    error = null,
    busy = false,
    copied = false,
    onCopy,
  }: {
    output?: string | null;
    error?: string | null;
    busy?: boolean;
    copied?: boolean;
    onCopy: () => void;
  } = $props();
</script>

<div class="field grow">
  <span id="translation-label" class="label">Translation</span>
  <div
    class="output"
    class:hint={!output && !error}
    role="region"
    aria-labelledby="translation-label"
    aria-live="polite"
  >
    {#if error}
      <span class="error">{error}</span>
    {:else if busy}
      Translating…
    {:else if output}
      {output}
    {:else}
      Translation output appears here.
    {/if}
  </div>
  <div class="row">
    <button class="btn" onclick={onCopy} disabled={!output}>Copy</button>
    <span class="hint">{copied ? "Copied!" : "Press Enter to copy"}</span>
  </div>
</div>
