<script lang="ts">
  // Manual translation view (P0-011, PRD §10.4). Owns the input, target
  // selector, the real provider call, and clipboard copy. Renders Result for
  // the output. Mounts fresh each time the panel switches to this view, so it
  // re-reads the latest settings (default provider/model + languages).
  import { onMount, onDestroy } from "svelte";
  import { isTauri } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { writeText } from "@tauri-apps/plugin-clipboard-manager";
  import {
    getSettings,
    translate as runTranslate,
    openAccessibilitySettings,
    type Settings,
    type Language,
    type RoutingMode,
  } from "./lib/tauri";
  import Result from "./Result.svelte";

  // A selected-text hotkey capture handed down from App (P0-014/P0-015). `id`
  // lets us process each request once; `text` is the captured selection (or
  // `error` if capture failed). `action` selects the routing mode.
  type ShortcutRequest = {
    action: "primary" | "secondary";
    text: string | null;
    error: string | null;
    id: number;
  };
  let {
    request = null,
    onOpenSettings,
  }: {
    request?: ShortcutRequest | null;
    // Called when the secondary hotkey fires but no secondary language is
    // configured — App switches to the Settings view (P0-015, PRD §10.3).
    onOpenSettings?: () => void;
  } = $props();

  let settings = $state<Settings | null>(null);
  let settingsPromise: Promise<Settings> | null = null;
  let sourceText = $state("");
  let targetValue = $state("auto"); // "auto" (primary routing) or a language code
  let translating = $state(false);
  let output = $state<string | null>(null);
  let error = $state<string | null>(null);
  let copied = $state(false);
  // True when the current error came from selected-text capture, so the result
  // pane can offer the Accessibility-permission shortcut (P0-016).
  let permissionHelp = $state(false);
  let sourceEl = $state<HTMLTextAreaElement | null>(null);

  // A default model must be configured (in Settings → Models) to translate.
  const ready = $derived(!!settings?.defaultModel);

  // Target options: Auto, then each configured language as an explicit target.
  // Each carries the actual configured Language so an explicit translation uses
  // the user's (possibly hand-edited) name, and disabled additional languages
  // are excluded.
  type Target = { value: string; label: string; lang: Language | null };
  const targets = $derived.by<Target[]>(() => {
    const opts: Target[] = [{ value: "auto", label: "Auto (detect & route)", lang: null }];
    if (!settings) return opts;
    const seen = new Set<string>();
    const add = (l: Language) => {
      if (!seen.has(l.code)) {
        seen.add(l.code);
        opts.push({ value: l.code, label: `→ ${l.name}`, lang: { code: l.code, name: l.name } });
      }
    };
    add(settings.languages.primary);
    if (settings.languages.secondary) add(settings.languages.secondary);
    for (const a of settings.languages.additional) if (a.enabled) add(a);
    return opts;
  });

  onMount(async () => {
    await ensureSettings();
    window.addEventListener("keydown", onWindowKeydown);
  });

  onDestroy(() => window.removeEventListener("keydown", onWindowKeydown));

  // Load settings once (deduped). Awaited by runTranslation so a hotkey request
  // that arrives before onMount finishes still translates once settings load.
  async function ensureSettings(): Promise<Settings | null> {
    if (settings) return settings;
    if (!isTauri()) return null;
    if (!settingsPromise) settingsPromise = getSettings();
    try {
      settings = await settingsPromise;
    } catch (e) {
      error = `Could not load settings: ${e}`;
    }
    return settings;
  }

  // The single translation path used by the manual button and the hotkey flow.
  async function runTranslation(
    text: string,
    mode: RoutingMode,
    explicit: Language | null,
  ) {
    if (translating || !text.trim()) return;
    const s = await ensureSettings();
    if (!s?.defaultModel) return; // not ready; the source is prefilled, hint shows
    error = null;
    copied = false;
    output = null;
    permissionHelp = false; // a provider error is not a permission problem
    translating = true;
    try {
      const resp = await runTranslate({
        sourceText: text,
        routingMode: mode,
        explicitTargetLanguage: explicit,
        provider: s.defaultProvider,
        model: s.defaultModel,
        preserveFormatting: true,
      });
      output = resp.translatedText;
      // Move focus off the input so Enter copies the result (see onWindowKeydown).
      sourceEl?.blur();
    } catch (e) {
      error = String(e);
    } finally {
      translating = false;
    }
  }

  function doTranslate() {
    const explicit = targets.find((t) => t.value === targetValue)?.lang ?? null;
    const mode: RoutingMode = targetValue === "auto" ? "primary" : "explicit";
    runTranslation(sourceText, mode, explicit);
  }

  // A selected-text hotkey capture arrived. `handledId` (a plain, untracked
  // local) ensures each request runs once.
  let handledId: number | undefined;
  $effect(() => {
    const req = request;
    if (!req || req.id === handledId) return;
    handledId = req.id;
    handleRequest(req);
  });

  // Prefill the source and translate per the action's routing rules. For the
  // secondary hotkey with no secondary language configured, redirect to Settings
  // instead of erroring (PRD §10.3).
  async function handleRequest(req: ShortcutRequest) {
    if (req.error) {
      // Capture failed: show the reason (with the Accessibility shortcut) and
      // clear any prior result/source so stale text doesn't linger.
      error = req.error;
      permissionHelp = true;
      sourceText = "";
      output = null;
      copied = false;
      return;
    }
    if (req.text == null) return;

    if (req.action === "secondary") {
      const s = await ensureSettings();
      if (s && !s.languages.secondary) {
        onOpenSettings?.();
        return;
      }
    }

    sourceText = req.text;
    output = null;
    copied = false;
    error = null;
    const mode: RoutingMode = req.action === "secondary" ? "secondary" : "primary";
    runTranslation(req.text, mode, null);
  }

  async function copy() {
    if (!output) return;
    // Reset first so `copied` reflects only this attempt (copyAndDismiss gates
    // on it) and a stale copy error doesn't linger over a successful re-copy.
    copied = false;
    error = null;
    try {
      await writeText(output);
      copied = true;
    } catch (e) {
      error = `Could not copy: ${e}`;
    }
  }

  // Enter after a translation copies and dismisses the panel (PRD §10.4 step 8).
  // The Copy button only copies (no dismiss).
  async function copyAndDismiss() {
    if (!output) return;
    await copy();
    if (copied) {
      try {
        await getCurrentWindow().hide();
      } catch {
        /* dismiss is best-effort; the copy already succeeded. */
      }
    }
  }

  function onSourceKeydown(e: KeyboardEvent) {
    // Enter translates; Shift+Enter inserts a newline (multi-line input).
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      doTranslate();
    }
  }

  // After a translation, Enter (when not editing the input) copies the result
  // and dismisses the panel.
  function onWindowKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey && output && e.target !== sourceEl) {
      e.preventDefault();
      copyAndDismiss();
    }
  }
</script>

<section class="body">
  <div class="field">
    <label class="label" for="source-text">Text</label>
    <textarea
      id="source-text"
      class="textarea"
      placeholder="Type or paste text to translate…  (Enter to translate, Shift+Enter for a new line)"
      bind:this={sourceEl}
      bind:value={sourceText}
      onkeydown={onSourceKeydown}
      oninput={() => (copied = false)}
    ></textarea>
  </div>

  <div class="row">
    <div class="field grow">
      <label class="label" for="target-lang">Target</label>
      <select id="target-lang" class="select" bind:value={targetValue}>
        {#each targets as t (t.value)}
          <option value={t.value}>{t.label}</option>
        {/each}
      </select>
    </div>
    <button
      class="btn btn--primary"
      onclick={doTranslate}
      disabled={!ready || !sourceText.trim() || translating}
    >
      {translating ? "Translating…" : "Translate"}
    </button>
  </div>

  <Result
    {output}
    {error}
    busy={translating}
    {copied}
    showPermissionHelp={permissionHelp}
    onCopy={copy}
    onOpenAccessibility={openAccessibilitySettings}
  />
</section>
