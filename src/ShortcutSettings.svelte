<script lang="ts">
  // Shortcuts section of Settings (P0-007 + P1-002, PRD §10.6.2).
  //
  // Lets the user record custom global shortcuts for "translate selection",
  // "translate to secondary", and each additional language (FR-032); a master
  // toggle disables them all (FR-034). Recording pauses the live shortcuts so
  // the combo reaches the webview (macOS consumes registered global hotkeys),
  // validates it (invalid combos are rejected), persists, and re-registers —
  // surfacing OS-registration failures and same-combo conflicts (FR-033).
  import { onMount, onDestroy } from "svelte";
  import { isTauri } from "@tauri-apps/api/core";
  import {
    applyShortcuts,
    pauseShortcuts,
    shortcutErrors,
    validateShortcut,
    type Settings,
  } from "./lib/tauri";

  let { settings, onChange }: { settings: Settings; onChange: () => void } =
    $props();

  // Mirror config::Settings::default() (src-tauri/src/config.rs) for reset.
  const DEFAULT_PRIMARY = "Cmd+Shift+T";
  const DEFAULT_SECONDARY = "Cmd+Shift+Option+T";

  let errors = $state<string[]>([]);
  // The slot currently being recorded ("primary" | "secondary" | language code),
  // or null. While set, global shortcuts are paused and keydowns are captured.
  let recording = $state<string | null>(null);
  let recordError = $state("");

  const GLYPH: Record<string, string> = {
    Cmd: "⌘", Command: "⌘", Super: "⌘",
    Shift: "⇧", Option: "⌥", Alt: "⌥", Ctrl: "⌃", Control: "⌃",
  };
  const pretty = (accel: string) =>
    accel ? accel.split("+").map((p) => GLYPH[p] ?? p).join("") : "";

  onMount(() => {
    // Capture phase so a recorded Esc/combo is intercepted before App's global
    // Esc-to-dismiss handler (and before text inputs) — only while recording.
    window.addEventListener("keydown", onRecordKeydown, true);
    window.addEventListener("blur", cancelRecording);
    if (isTauri())
      shortcutErrors()
        .then((e) => (errors = e))
        .catch(() => {});
  });
  onDestroy(() => {
    window.removeEventListener("keydown", onRecordKeydown, true);
    window.removeEventListener("blur", cancelRecording);
    // Never leave shortcuts paused if we unmount mid-recording.
    if (recording !== null && isTauri()) void applyShortcuts().catch(() => {});
  });

  async function reapply() {
    if (!isTauri()) return;
    try {
      errors = await applyShortcuts();
    } catch (e) {
      errors = [String(e)];
    }
  }

  async function startRecording(slot: string) {
    recordError = "";
    if (isTauri()) {
      try {
        await pauseShortcuts();
      } catch {
        /* recording still works for unregistered combos */
      }
    }
    recording = slot;
  }

  function cancelRecording() {
    if (recording === null) return;
    recording = null;
    recordError = "";
    void reapply(); // restore the live shortcuts
  }

  // Map a KeyboardEvent to a Tauri accelerator key token (KeyT→T, Digit1→1, …).
  // Unmapped keys return null and are rejected with a hint; the backend
  // validator is the final authority on what the plugin accepts.
  function keyToken(e: KeyboardEvent): string | null {
    const code = e.code;
    if (/^Key[A-Z]$/.test(code)) return code.slice(3);
    if (/^Digit[0-9]$/.test(code)) return code.slice(5);
    if (/^F\d{1,2}$/.test(code)) return code;
    const map: Record<string, string> = {
      Space: "Space", Enter: "Enter", Tab: "Tab",
      Backspace: "Backspace", Delete: "Delete",
      ArrowUp: "Up", ArrowDown: "Down", ArrowLeft: "Left", ArrowRight: "Right",
      Minus: "Minus", Equal: "Equal", Comma: "Comma", Period: "Period",
      Slash: "Slash", Backslash: "Backslash", Semicolon: "Semicolon",
      Quote: "Quote", BracketLeft: "BracketLeft", BracketRight: "BracketRight",
      Backquote: "Backquote",
    };
    return map[code] ?? null;
  }

  async function onRecordKeydown(e: KeyboardEvent) {
    if (recording === null) return;
    e.preventDefault();
    e.stopPropagation();
    if (e.key === "Escape") {
      cancelRecording();
      return;
    }
    // Wait for a non-modifier key.
    if (["Shift", "Meta", "Alt", "Control", "CapsLock", "Dead"].includes(e.key))
      return;
    const mods: string[] = [];
    if (e.metaKey) mods.push("Cmd");
    if (e.ctrlKey) mods.push("Control");
    if (e.altKey) mods.push("Option");
    if (e.shiftKey) mods.push("Shift");
    if (mods.length === 0) {
      recordError = "Use at least one modifier (⌘ ⌃ ⌥ ⇧) with a key.";
      return;
    }
    const key = keyToken(e);
    if (!key) {
      recordError = "Unsupported key — try a letter, digit, or F-key.";
      return;
    }
    await assign(recording, [...mods, key].join("+"));
  }

  async function assign(slot: string, accel: string) {
    if (isTauri() && !(await validateShortcut(accel))) {
      recordError = `“${pretty(accel)}” isn't a valid shortcut.`;
      return; // stay recording so the user can try another combo
    }
    if (slot === "primary") settings.shortcuts.translatePrimary = accel;
    else if (slot === "secondary") settings.shortcuts.translateSecondary = accel;
    else {
      const lang = settings.languages.additional.find((l) => l.code === slot);
      if (lang) lang.shortcut = accel;
    }
    recording = null;
    recordError = "";
    onChange();
    await reapply(); // re-register, surfacing conflicts/failures
  }

  async function clearShortcut(code: string) {
    const lang = settings.languages.additional.find((l) => l.code === code);
    if (lang?.shortcut) {
      lang.shortcut = null;
      onChange();
      await reapply();
    }
  }

  async function resetDefaults() {
    cancelRecording();
    settings.shortcuts.translatePrimary = DEFAULT_PRIMARY;
    settings.shortcuts.translateSecondary = DEFAULT_SECONDARY;
    for (const l of settings.languages.additional) l.shortcut = null;
    recordError = "";
    onChange();
    await reapply();
  }

  async function toggleEnabled(e: Event) {
    settings.shortcuts.enabled = (e.currentTarget as HTMLInputElement).checked;
    onChange();
    await reapply();
  }
</script>

{#snippet recorder(slot: string, accel: string)}
  <button
    class="accel-btn"
    class:recording={recording === slot}
    class:set={!!accel}
    onclick={() =>
      recording === slot ? cancelRecording() : startRecording(slot)}
    aria-label={recording === slot
      ? "Recording shortcut; press a combination or Escape to cancel"
      : "Set shortcut"}
  >
    {#if recording === slot}
      Press keys… (Esc)
    {:else if accel}
      {pretty(accel)}
    {:else}
      Set shortcut
    {/if}
  </button>
{/snippet}

<div class="section">
  <h2 class="section__title">Shortcuts</h2>

  <label class="row" style="cursor: pointer;">
    <input
      type="checkbox"
      checked={settings.shortcuts.enabled}
      onchange={toggleEnabled}
    />
    <span class="grow">Enable global keyboard shortcuts</span>
  </label>

  <div class="row">
    <span class="grow" class:hint={!settings.shortcuts.enabled}>Translate selection</span>
    {@render recorder("primary", settings.shortcuts.translatePrimary)}
  </div>
  <div class="row">
    <span class="grow" class:hint={!settings.shortcuts.enabled}>Translate to secondary</span>
    {@render recorder("secondary", settings.shortcuts.translateSecondary)}
  </div>

  {#each settings.languages.additional as lang (lang.code)}
    <div class="row">
      <span class="grow" class:hint={!settings.shortcuts.enabled}>Translate to {lang.name}</span>
      {@render recorder(lang.code, lang.shortcut ?? "")}
      <button
        class="icon-btn"
        onclick={() => clearShortcut(lang.code)}
        disabled={!lang.shortcut}
        aria-label="Clear {lang.name} shortcut"
        title="Clear">✕</button
      >
    </div>
  {/each}

  {#if recordError}
    <p class="error">{recordError}</p>
  {/if}
  {#each errors as message, i (i)}
    <p class="error">{message}</p>
  {/each}

  <div class="row">
    <button class="btn" onclick={resetDefaults}>Reset to defaults</button>
  </div>
</div>

<style>
  .accel-btn {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: var(--tl-fs-sm);
    padding: 2px 8px;
    min-width: 84px;
    border: 1px solid var(--tl-border);
    border-radius: var(--tl-radius-sm);
    background: var(--tl-bg);
    color: var(--tl-text-muted);
    cursor: pointer;
    white-space: nowrap;
  }
  .accel-btn.set {
    color: var(--tl-text);
  }
  .accel-btn.recording {
    border-color: var(--tl-accent);
    color: var(--tl-accent);
  }
</style>
