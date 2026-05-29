<script lang="ts">
  // Appearance section of Settings (P2-012). Toggles the macOS panel
  // translucency ("Liquid Glass"). `set_translucency` both persists the
  // preference and applies the vibrancy to the live window; we also flip the
  // body class (via onChange) so the panel background goes transparent to reveal
  // it. Respects the system "Reduce transparency" setting (handled by AppKit).
  import {
    setTranslucency,
    setTheme,
    type Settings,
    type Theme,
  } from "./lib/tauri";

  let {
    settings,
    // Called after a successful toggle so App can flip the body `.translucent`
    // class that makes the panel background transparent for the vibrancy.
    onChange = () => {},
    // Apply a text-size change live (App sets the `--tl-fs-scale` root var); fired
    // continuously as the slider moves, so it must stay cheap (no IPC).
    onFontScaleChange = () => {},
    // Persist the current settings (the parent's save). Fired once when the
    // slider is released, not per tick.
    onFontScalePersist = () => {},
  }: {
    settings: Settings;
    onChange?: (enabled: boolean) => void;
    onFontScaleChange?: (scale: number) => void;
    onFontScalePersist?: () => void;
  } = $props();

  let busy = $state(false);
  let error = $state<string | null>(null);

  const THEMES: { value: Theme; label: string }[] = [
    { value: "system", label: "System" },
    { value: "light", label: "Light" },
    { value: "dark", label: "Dark" },
  ];

  async function changeTheme(theme: Theme) {
    if (busy || settings.ui.theme === theme) return;
    busy = true;
    error = null;
    try {
      await setTheme(theme); // persists ui.theme + applies the live appearance
      settings.ui.theme = theme; // reflect in the in-memory settings
    } catch (e) {
      error = `Could not change theme: ${e}`;
    } finally {
      busy = false;
    }
  }

  // Slider bounds match `ui.fontScale`'s documented range. Kept conservative so
  // the fixed-width panel layout stays intact at the extremes.
  const FS_MIN = 0.8;
  const FS_MAX = 1.4;
  const FS_STEP = 0.1;

  // Fill fraction (0–100%) for the custom slider track gradient. Derived from the
  // live value so the accent fill tracks the thumb as it moves.
  const fillPct = $derived(
    ((settings.ui.fontScale - FS_MIN) / (FS_MAX - FS_MIN)) * 100,
  );

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

  // Live preview: update the in-memory setting + the root CSS var on every tick.
  function previewFontScale(scale: number) {
    settings.ui.fontScale = scale;
    onFontScaleChange(scale);
  }
</script>

<div class="section">
  <h2 class="section__title">Appearance</h2>

  <span class="label" id="theme-label">Theme</span>
  <div class="seg" role="group" aria-labelledby="theme-label">
    {#each THEMES as t (t.value)}
      <button
        type="button"
        class="seg__btn"
        class:seg__btn--on={settings.ui.theme === t.value}
        aria-pressed={settings.ui.theme === t.value}
        disabled={busy}
        onclick={() => changeTheme(t.value)}
      >
        {t.label}
      </button>
    {/each}
  </div>

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

  <div class="size">
    <div class="row size__head">
      <span class="label" id="text-size-label">Text size</span>
      <span class="size__value"
        >{Math.round(settings.ui.fontScale * 100)}%</span
      >
    </div>
    <div class="row size__slider">
      <span class="size__tick" aria-hidden="true">A</span>
      <input
        type="range"
        class="size__range"
        style="--fill: {fillPct}%"
        aria-labelledby="text-size-label"
        min={FS_MIN}
        max={FS_MAX}
        step={FS_STEP}
        value={settings.ui.fontScale}
        oninput={(e) => previewFontScale(e.currentTarget.valueAsNumber)}
        onchange={() => onFontScalePersist()}
      />
      <span class="size__tick size__tick--lg" aria-hidden="true">A</span>
    </div>
    <p class="hint">Size of the text you type and the translation result.</p>
    <!-- Live sample at the current size. Below the slider so growing it can't
         shift the slider; scales off the same `--tl-content-scale` root var. -->
    <div class="size__preview" aria-hidden="true">
      The quick brown fox jumps over the lazy dog.
    </div>
  </div>
</div>

<style>
  /* Segmented theme picker: one pill split into equal-width options, the active
     one filled with the accent. */
  .seg {
    display: flex;
    gap: 2px;
    padding: 2px;
    margin-top: var(--tl-sp-1);
    border: 1px solid var(--tl-border);
    border-radius: 980px;
    background: var(--tl-surface);
  }
  .seg__btn {
    flex: 1;
    appearance: none;
    border: none;
    border-radius: 980px;
    padding: 5px 0;
    background: transparent;
    color: var(--tl-text);
    font: inherit;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.12s ease;
  }
  .seg__btn:hover:not(:disabled):not(.seg__btn--on) {
    background: var(--tl-surface-hover);
  }
  .seg__btn--on {
    background: var(--tl-accent);
    color: var(--tl-on-accent);
  }
  .seg__btn:disabled {
    cursor: not-allowed;
  }

  /* Checkbox + label row: align the box with the text, normal cursor. */
  .check {
    cursor: pointer;
    gap: var(--tl-sp-2);
  }
  .check input {
    margin: 0;
  }

  /* Text-size slider, flanked by small/large "A" cues. */
  .size {
    margin-top: var(--tl-sp-3);
  }
  .size__head {
    justify-content: space-between;
  }
  .size__value {
    font-size: var(--tl-fs-sm);
    color: var(--tl-text-muted);
    font-variant-numeric: tabular-nums;
  }
  .size__slider {
    margin-top: var(--tl-sp-1);
  }
  /* Custom range so the unfilled track follows the theme (the default native
     track is light in both modes — the white bar in dark mode). WebKit-only is
     fine: macOS/WKWebView is the only target. The filled portion is an accent
     gradient cut at `--fill` (set inline from the live value); the thumb stays
     white to match the native macOS slider in both light and dark. */
  .size__range {
    flex: 1;
    -webkit-appearance: none;
    appearance: none;
    height: 16px; /* generous hit area; the visible track is thinner below */
    background: transparent;
    cursor: pointer;
  }
  .size__range::-webkit-slider-runnable-track {
    height: 4px;
    border-radius: 980px;
    background: linear-gradient(
      to right,
      var(--tl-accent) 0 var(--fill, 0%),
      var(--tl-border) var(--fill, 0%) 100%
    );
  }
  .size__range::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 16px;
    height: 16px;
    margin-top: -6px; /* centre the 16px thumb on the 4px track */
    border-radius: 50%;
    background: #fff;
    border: 1px solid var(--tl-border);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.25);
  }
  .size__tick {
    color: var(--tl-text-muted);
    line-height: 1;
    /* Fixed (not scaled) so the cues stay put while the slider scales the panel. */
    font-size: 12px;
  }
  .size__tick--lg {
    font-size: 18px;
  }

  /* Sample text, styled like a read-only text box so it previews the translate
     view's input/output at the chosen size. */
  .size__preview {
    margin-top: var(--tl-sp-2);
    padding: var(--tl-sp-2) var(--tl-sp-3);
    border: 1px solid var(--tl-border);
    border-radius: var(--tl-radius);
    background: var(--tl-surface);
    color: var(--tl-text);
    font-size: calc(var(--tl-fs-base) * var(--tl-content-scale));
    line-height: 1.4;
  }
</style>
