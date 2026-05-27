<script lang="ts">
  // Shortcuts section of Settings (P0-007, PRD §10.6.2).
  //
  // Phase 0 shows the default global shortcuts (custom remapping is Phase 1) and
  // offers a master enable/disable toggle. Toggling re-registers the shortcuts
  // and surfaces any registration failures (e.g. an accelerator already taken).
  import { onMount } from "svelte";
  import { isTauri } from "@tauri-apps/api/core";
  import { applyShortcuts, shortcutErrors, type Settings } from "./lib/tauri";

  let { settings, onChange }: { settings: Settings; onChange: () => void } =
    $props();

  let errors = $state<string[]>([]);

  const rows = $derived([
    { label: "Translate selection", accel: settings.shortcuts.translatePrimary },
    { label: "Translate to secondary", accel: settings.shortcuts.translateSecondary },
  ]);

  // Render an accelerator like "Cmd+Shift+T" with macOS modifier glyphs.
  const GLYPH: Record<string, string> = {
    Cmd: "⌘",
    Command: "⌘",
    Shift: "⇧",
    Option: "⌥",
    Alt: "⌥",
    Ctrl: "⌃",
    Control: "⌃",
  };
  const pretty = (accel: string) =>
    accel.split("+").map((part) => GLYPH[part] ?? part).join("");

  onMount(async () => {
    if (!isTauri()) return;
    try {
      errors = await shortcutErrors();
    } catch {
      errors = [];
    }
  });

  async function toggleEnabled(e: Event) {
    settings.shortcuts.enabled = (e.currentTarget as HTMLInputElement).checked;
    onChange();
    try {
      errors = await applyShortcuts();
    } catch (err) {
      errors = [String(err)];
    }
  }
</script>

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

  {#each rows as row (row.label)}
    <div class="row">
      <span class="grow" class:hint={!settings.shortcuts.enabled}>{row.label}</span>
      <kbd class="accel" class:hint={!settings.shortcuts.enabled}>{pretty(row.accel)}</kbd>
    </div>
  {/each}

  {#each errors as message, i (i)}
    <p class="error">{message}</p>
  {/each}

  <p class="hint">Custom shortcuts are coming in a later version.</p>
</div>

<style>
  .accel {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: var(--tl-fs-sm);
    padding: 2px 6px;
    border: 1px solid var(--tl-border);
    border-radius: var(--tl-radius-sm);
    background: var(--tl-bg);
    white-space: nowrap;
  }
</style>
