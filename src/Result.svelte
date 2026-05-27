<script lang="ts">
  // Output pane of the translate view (P0-011/P0-012). Presentational: the
  // parent owns the translation state and the clipboard call; this renders the
  // output, a copy affordance (icon + "Press Enter to copy", placed in the
  // header row to save vertical space), and errors.
  let {
    output = null,
    error = null,
    busy = false,
    copied = false,
    showPermissionHelp = false,
    onCopy,
    onOpenAccessibility,
  }: {
    output?: string | null;
    error?: string | null;
    busy?: boolean;
    copied?: boolean;
    // Shown for a capture error: a shortcut to grant Accessibility (P0-016).
    showPermissionHelp?: boolean;
    onCopy: () => void;
    onOpenAccessibility?: () => void;
  } = $props();
</script>

<div class="field grow">
  <div class="output">
    <div
      class="output-body"
      class:hint={!output && !error}
      role="region"
      aria-label="Translation"
      aria-live="polite"
    >
      {#if error}
        <span class="error">{error}</span>
      {:else if output}
        <!-- Shown while streaming too: output grows as deltas arrive (busy stays
             true until the stream ends), so it takes precedence over "Translating…". -->
        {output}
      {:else if busy}
        Translating…
      {:else}
        Translation output appears here.
      {/if}
    </div>

    {#if output && !busy}
      <!-- Pinned in the field's bottom-right corner, inside the box. Only once
           the stream has finished, so Enter/Copy act on the complete text. -->
      <div class="output-actions">
        <span class="hint">{copied ? "Copied!" : "Press Enter to copy"}</span>
        <button
          class="icon-btn"
          onclick={onCopy}
          aria-label="Copy translation"
          title="Copy translation"
        >
          <svg
            width="15"
            height="15"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
          >
            <rect x="9" y="9" width="11" height="11" rx="2"></rect>
            <path d="M5 15V5a2 2 0 0 1 2-2h10"></path>
          </svg>
        </button>
      </div>
    {/if}
  </div>

  {#if error && showPermissionHelp}
    <div class="row">
      <button class="btn" onclick={onOpenAccessibility}>
        Open Accessibility Settings
      </button>
    </div>
  {/if}
</div>
