<script lang="ts">
  import { onMount } from "svelte";
  import { isTauri } from "@tauri-apps/api/core";
  import { revealItemInDir } from "@tauri-apps/plugin-opener";
  import {
    getSettings,
    saveSettings,
    settingsPath,
    applyShortcuts,
    type Settings,
    type UpdateStatus,
  } from "./lib/tauri";
  import { isRecordingShortcut } from "./lib/recording";
  import LanguageSettings from "./LanguageSettings.svelte";
  import ShortcutSettings from "./ShortcutSettings.svelte";
  import StartupSettings from "./StartupSettings.svelte";
  import ProviderSettings from "./ProviderSettings.svelte";
  import PrivacySettings from "./PrivacySettings.svelte";
  import AppearanceSettings from "./AppearanceSettings.svelte";
  import AboutSettings from "./AboutSettings.svelte";

  // Settings view of the panel (not a separate window). The version is passed
  // down from App so this view doesn't re-fetch it.
  //
  // This component owns the loaded `settings` object and a `persist()` that
  // saves it; section components (Languages, and later Shortcuts/Providers/…)
  // mutate `settings` and call `persist`. P0-003 laid out the section shells;
  // P0-006 fills Languages. Shortcuts (P0-007), Providers & Models (P0-009),
  // Output/Privacy (P0-017), and Updates/About (P0-018) follow.
  // `hidden` keeps this view mounted but off-screen while the translate view is
  // active (so neither view loses its state on a switch).
  let {
    version = "—",
    hidden = false,
    // The pending update (from a manual check or the background poll), so the
    // Updates section shows the install affordance even if found in the background.
    update = null,
    // Bubbles an update found by the Updates section up to App (lights the bell).
    onUpdateAvailable = () => {},
    // Bubbles a translucency toggle up to App (flips the body `.translucent` class).
    onTranslucencyChange = () => {},
    // Bubbles a text-size change up to App (sets the `--tl-fs-scale` root var live).
    onFontScaleChange = () => {},
  }: {
    version?: string;
    hidden?: boolean;
    update?: UpdateStatus | null;
    onUpdateAvailable?: (status: UpdateStatus | null) => void;
    onTranslucencyChange?: (enabled: boolean) => void;
    onFontScaleChange?: (scale: number) => void;
  } = $props();

  let configPath = $state<string | null>(null);
  let settings = $state<Settings | null>(null);
  let saveError = $state<string | null>(null);

  onMount(async () => {
    if (!isTauri()) return;
    try {
      settings = await getSettings();
    } catch (e) {
      saveError = `Could not load settings: ${e}`;
    }
    try {
      configPath = await settingsPath();
    } catch {
      configPath = null;
    }
  });

  async function persist() {
    if (!settings) return;
    try {
      await saveSettings(settings);
      saveError = null;
      // Keep the registered global shortcuts in sync with the saved config —
      // e.g. removing an additional language must drop its hotkey (P1-002). The
      // save is awaited first so the re-register reads the new config (no race).
      // Skip while a shortcut is being recorded: recording deliberately pauses
      // the live shortcuts, and a save from another section must not un-pause
      // them mid-recording (ShortcutSettings re-applies when recording ends).
      if (!isRecordingShortcut()) await applyShortcuts();
    } catch (e) {
      saveError = `Could not save settings: ${e}`;
    }
  }

  async function reveal() {
    if (!configPath) return;
    // The file should exist (written at startup), but guard the rare first-run
    // failure so a rejected reveal doesn't surface as an unhandled rejection.
    try {
      await revealItemInDir(configPath);
    } catch {
      /* nothing actionable for the user; the path is shown above to copy. */
    }
  }
</script>

<section class="body" class:hidden>
  {#if saveError}
    <p class="error">{saveError}</p>
  {/if}

  {#if settings}
    <LanguageSettings {settings} onChange={persist} />
  {:else}
    <div class="section">
      <h2 class="section__title">Languages</h2>
      <p class="hint">Loading…</p>
    </div>
  {/if}

  {#if settings}
    <ShortcutSettings {settings} onChange={persist} {hidden} />
    <ProviderSettings {settings} onChange={persist} />
    <StartupSettings {settings} />
    <AppearanceSettings
      {settings}
      onChange={onTranslucencyChange}
      {onFontScaleChange}
      onFontScalePersist={persist}
    />
  {/if}

  <div class="section">
    <h2 class="section__title">Output</h2>
    <p class="hint">What happens with a translation result.</p>
  </div>

  <PrivacySettings />

  <div class="section">
    <h2 class="section__title">Settings file</h2>
    <p class="hint">
      Advanced non-secret settings can be edited here. API keys are stored
      separately in the macOS Keychain, never in this file.
    </p>
    {#if configPath}
      <code class="path">{configPath}</code>
      <div class="row">
        <button class="btn" onclick={reveal}>Reveal in Finder</button>
      </div>
    {/if}
  </div>

  <AboutSettings {version} {update} {settings} {onUpdateAvailable} onChange={persist} />
</section>

<style>
  .path {
    display: block;
    padding: var(--tl-sp-2) var(--tl-sp-3);
    border: 1px solid var(--tl-border);
    border-radius: var(--tl-radius-sm);
    background: var(--tl-bg);
    color: var(--tl-text-muted);
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: var(--tl-fs-sm);
    user-select: text;
    -webkit-user-select: text;
    word-break: break-all;
  }
</style>
