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
    translateStream,
    listProviders,
    openAccessibilitySettings,
    Channel,
    type Settings,
    type Language,
    type RoutingMode,
    type ProviderMeta,
    type TranslationDelta,
  } from "./lib/tauri";
  import Result from "./Result.svelte";

  // A selected-text hotkey capture handed down from App (P0-014/P0-015). `id`
  // lets us process each request once; `text` is the captured selection (or
  // `error` if capture failed). `action` selects the routing mode.
  type ShortcutRequest = {
    action: "primary" | "secondary" | "explicit";
    text: string | null;
    error: string | null;
    // For an additional-language shortcut (P1-002): the explicit target language.
    target: Language | null;
    id: number;
  };
  let {
    request = null,
    active = true,
    hidden = false,
    onOpenSettings,
  }: {
    request?: ShortcutRequest | null;
    // Whether this is the visible view. The component stays mounted while hidden
    // (preserving the typed text + result); it re-reads settings when it becomes
    // active again so newly-saved providers/models/languages are picked up.
    active?: boolean;
    hidden?: boolean;
    // Called when the secondary hotkey fires but no secondary language is
    // configured — App switches to the Settings view (P0-015, PRD §10.3).
    onOpenSettings?: () => void;
  } = $props();

  let settings = $state<Settings | null>(null);
  let settingsPromise: Promise<Settings> | null = null;
  // Provider metadata (static), fetched once, used to decide whether to stream.
  let providerMeta: ProviderMeta[] | null = null;
  let providerMetaPromise: Promise<ProviderMeta[]> | null = null;
  // Monotonic id per translation run; the streaming channel handler checks it so
  // late deltas from a superseded run can never write into a newer result.
  let runId = 0;
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

  onMount(() => {
    window.addEventListener("keydown", onWindowKeydown);
    void ensureProviderMeta(); // warm the streaming-capability check
  });

  onDestroy(() => window.removeEventListener("keydown", onWindowKeydown));

  // Re-read settings whenever the view becomes active (on mount, and on every
  // return from Settings), so a just-saved provider/model/language is reflected.
  $effect(() => {
    if (active) void reloadSettings();
  });

  // Load settings once (deduped). Awaited by runTranslation so a hotkey request
  // that arrives before settings load still translates once they do.
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

  // Force a fresh settings read (used when the view (re)activates).
  async function reloadSettings() {
    if (!isTauri()) return;
    try {
      settingsPromise = getSettings();
      settings = await settingsPromise;
    } catch (e) {
      error = `Could not load settings: ${e}`;
    }
  }

  // Load provider metadata once (deduped). Best-effort: if it fails we just fall
  // back to the non-streaming path.
  async function ensureProviderMeta(): Promise<ProviderMeta[] | null> {
    if (providerMeta) return providerMeta;
    if (!isTauri()) return null;
    if (!providerMetaPromise) providerMetaPromise = listProviders();
    try {
      providerMeta = await providerMetaPromise;
    } catch {
      /* leave null → non-streaming fallback */
    }
    return providerMeta;
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
    const model = s.defaultModel;
    error = null;
    copied = false;
    output = null;
    permissionHelp = false; // a provider error is not a permission problem
    translating = true;
    const myRun = ++runId;
    const req = {
      sourceText: text,
      routingMode: mode,
      explicitTargetLanguage: explicit,
      provider: s.defaultProvider,
      model,
      preserveFormatting: true,
    };
    try {
      const meta = (await ensureProviderMeta())?.find(
        (p) => p.id === s.defaultProvider,
      );
      if (meta?.supportsStreaming) {
        // Stream: append deltas as they arrive, then settle on the trimmed final.
        output = "";
        const channel = new Channel<TranslationDelta>();
        channel.onmessage = (d) => {
          if (myRun !== runId) return; // ignore deltas from a superseded run
          output = (output ?? "") + d.text;
        };
        const resp = await translateStream(req, channel);
        output = resp.translatedText;
      } else {
        const resp = await runTranslate(req);
        output = resp.translatedText;
      }
      // Move focus off the input so Enter copies the result (see onWindowKeydown).
      sourceEl?.blur();
    } catch (e) {
      error = String(e);
      output = null; // discard any partial stream on failure
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

  // Prefill the source from a selected-text hotkey and translate to the current
  // Target. The Target dropdown is the sticky session choice: an explicit
  // language is always honored; only "Auto" applies the primary/secondary rules.
  // The primary hotkey keeps whatever Target is selected; the secondary hotkey
  // switches the Target to the secondary language (redirecting to Settings if
  // none is configured, PRD §10.3).
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

    const s = await ensureSettings(); // load settings so `targets` is complete

    if (req.action === "secondary") {
      if (s && !s.languages.secondary) {
        onOpenSettings?.();
        return;
      }
      if (s?.languages.secondary) targetValue = s.languages.secondary.code;
    }

    // An additional-language shortcut forces its explicit target, regardless of
    // the sticky Target selection (P1-002). Reflect it in the dropdown too.
    if (req.action === "explicit" && req.target) {
      targetValue = req.target.code;
      sourceText = req.text;
      output = null;
      copied = false;
      error = null;
      permissionHelp = false;
      runTranslation(req.text, "explicit", req.target);
      return;
    }

    sourceText = req.text;
    output = null;
    copied = false;
    error = null;
    permissionHelp = false;
    doTranslate(); // translate to the current Target (Auto → routing rules)
  }

  async function copy() {
    if (!output || translating) return; // don't copy a partial stream
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
  // and dismisses the panel. Ignored while the Settings view is active so it
  // can't fire on a stale result from behind Settings.
  function onWindowKeydown(e: KeyboardEvent) {
    if (
      active &&
      e.key === "Enter" &&
      !e.shiftKey &&
      output &&
      !translating && // wait for the stream to finish before Enter copies
      e.target !== sourceEl
    ) {
      e.preventDefault();
      copyAndDismiss();
    }
  }
</script>

<section class="body" class:hidden>
  <div class="field grow">
    <textarea
      class="textarea"
      aria-label="Text to translate"
      placeholder="Type or paste text to translate…  (Enter to translate, Shift+Enter for a new line)"
      bind:this={sourceEl}
      bind:value={sourceText}
      onkeydown={onSourceKeydown}
      oninput={() => (copied = false)}
    ></textarea>
  </div>

  <div class="row">
    <label class="label" for="target-lang">Target</label>
    <select id="target-lang" class="select grow" bind:value={targetValue}>
      {#each targets as t (t.value)}
        <option value={t.value}>{t.label}</option>
      {/each}
    </select>
    <button
      class="btn btn--primary btn--translate"
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
