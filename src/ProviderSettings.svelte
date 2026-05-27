<script lang="ts">
  // Providers + Models sections of Settings (P0-009, PRD §10.6.3/§10.6.4).
  //
  // Keys are entered here, saved to the Keychain via the backend (never kept in
  // this file's state after Save), tested, and removed. The Models section lets
  // the user pick the default provider/model used for translation; providers
  // without a saved key can't be chosen and their models are disabled.
  import { onMount } from "svelte";
  import { isTauri } from "@tauri-apps/api/core";
  import {
    listProviders,
    hasProviderKey,
    setProviderKey,
    deleteProviderKey,
    testProviderKey,
    testProviderConnection,
    listProviderModels,
    type Settings,
    type ProviderId,
    type ProviderMeta,
  } from "./lib/tauri";

  let { settings, onChange }: { settings: Settings; onChange: () => void } =
    $props();

  // A sensible, fast default model per provider so a freshly-keyed provider can
  // translate immediately (overridable later in Models). If one is ever rejected
  // by a provider, the user just picks another from the model list. Ollama has
  // no entry — its models are whatever the user has pulled locally.
  const DEFAULT_MODELS: Partial<Record<ProviderId, string>> = {
    openai: "gpt-5-mini",
    anthropic: "claude-haiku-4-5",
    gemini: "gemini-2.5-flash",
    openrouter: "openai/gpt-5-mini",
  };

  // Ollama is local + keyless (P1-004): it's configured by endpoint URL, not an
  // API key, and is always "ready" to select (it has a default endpoint).
  const DEFAULT_OLLAMA_ENDPOINT = "http://localhost:11434";
  const isOllama = (id: ProviderId) => id === "ollama";
  // A provider is usable when it has a credential: a saved key (cloud) or — for
  // Ollama — always (the endpoint defaults).
  const ready = (id: ProviderId) => isOllama(id) || !!keyPresence[id];

  type Status =
    | { kind: "none" }
    | { kind: "saving" }
    | { kind: "configured" }
    | { kind: "testing" }
    | { kind: "valid" }
    | { kind: "invalid" }
    | { kind: "failed"; message: string };

  // All selectable providers — the four cloud ones plus Ollama (P1-004).
  let providers = $state<ProviderMeta[]>([]);
  // Backend-derived: does each cloud provider have a saved key? (Not in `settings`.)
  let keyPresence = $state<Record<string, boolean>>({});
  let keyInput = $state<Record<string, string>>({});
  // Ollama's endpoint URL (config, not Keychain). Editable in its row.
  let endpointInput = $state<string>(DEFAULT_OLLAMA_ENDPOINT);
  let status = $state<Record<string, Status>>({});

  let models = $state<string[]>([]);
  let modelsState = $state<"idle" | "loading" | "error">("idle");
  let modelsError = $state("");
  // When the model-list API fails, fall back to manual model entry (PRD §10.6.4).
  let manualModel = $state(false);
  // Surfaced if the initial provider/key load fails.
  let loadError = $state<string | null>(null);

  const STATUS_LABEL: Record<string, string> = {
    none: "Not configured",
    saving: "Saving…",
    configured: "Configured",
    testing: "Testing…",
    valid: "Connection OK",
    invalid: "Invalid key",
  };

  onMount(async () => {
    if (!isTauri()) return;
    try {
      providers = (await listProviders()).filter((p) => p.available);
      endpointInput =
        settings.providers.ollama?.endpoint ?? DEFAULT_OLLAMA_ENDPOINT;
      for (const p of providers) {
        if (isOllama(p.id)) {
          // Keyless: configured by endpoint, which always has a value.
          status[p.id] = { kind: "configured" };
          continue;
        }
        const present = await hasProviderKey(p.id);
        keyPresence[p.id] = present;
        status[p.id] = present ? { kind: "configured" } : { kind: "none" };
        keyInput[p.id] = "";
      }
      await loadModels();
    } catch (e) {
      loadError = `Could not load provider settings: ${e}`;
    }
  });

  async function save(id: ProviderId) {
    const key = keyInput[id]?.trim();
    if (!key) return;
    status[id] = { kind: "saving" };
    try {
      await setProviderKey(id, key);
      // Adopt this provider as the default if the current default isn't usable
      // yet (covers the first key saved). Checked BEFORE recording presence;
      // `ready` treats Ollama as usable so a configured Ollama keeps default.
      const adoptAsDefault = !ready(settings.defaultProvider);

      keyPresence[id] = true;
      settings.providers[id].enabled = true;
      // Seed a ready-to-use model so the user can translate without extra steps.
      if (!settings.providers[id].defaultModel) {
        settings.providers[id].defaultModel = DEFAULT_MODELS[id] ?? null;
      }
      keyInput[id] = ""; // don't retain the key in the UI
      status[id] = { kind: "configured" };

      if (adoptAsDefault) {
        settings.defaultProvider = id;
        settings.defaultModel = settings.providers[id].defaultModel ?? null;
      }
      onChange();
      if (id === settings.defaultProvider) await loadModels();
    } catch (e) {
      status[id] = { kind: "failed", message: String(e) };
    }
  }

  async function remove(id: ProviderId) {
    try {
      await deleteProviderKey(id);
      keyPresence[id] = false;
      settings.providers[id].enabled = false;
      keyInput[id] = "";
      status[id] = { kind: "none" };
      if (id === settings.defaultProvider) {
        settings.defaultModel = null;
        await loadModels(); // resets models/state for the now-keyless provider
      }
      onChange();
    } catch (e) {
      status[id] = { kind: "failed", message: String(e) };
    }
  }

  async function test(id: ProviderId) {
    const typed = keyInput[id]?.trim();
    if (!typed && !keyPresence[id]) return;
    status[id] = { kind: "testing" };
    try {
      // Test the just-typed key if present, else the saved one.
      const ok = typed
        ? await testProviderKey(id, typed)
        : await testProviderConnection(id);
      status[id] = ok ? { kind: "valid" } : { kind: "invalid" };
    } catch (e) {
      status[id] = { kind: "failed", message: String(e) };
    }
  }

  // ── Ollama (endpoint-based, keyless) ──────────────────────────────────────
  function ollamaEndpoint(): string {
    return endpointInput.trim() || DEFAULT_OLLAMA_ENDPOINT;
  }

  async function saveEndpoint() {
    const ep = ollamaEndpoint();
    status.ollama = { kind: "saving" };
    try {
      const adoptAsDefault = !ready(settings.defaultProvider);
      settings.providers.ollama.endpoint = ep;
      settings.providers.ollama.enabled = true;
      endpointInput = ep;
      status.ollama = { kind: "configured" };
      if (adoptAsDefault) {
        settings.defaultProvider = "ollama";
        settings.defaultModel = settings.providers.ollama.defaultModel ?? null;
      }
      onChange();
      if (settings.defaultProvider === "ollama") await loadModels();
    } catch (e) {
      status.ollama = { kind: "failed", message: String(e) };
    }
  }

  async function testEndpoint() {
    status.ollama = { kind: "testing" };
    try {
      // Reachable server → ok; unreachable → throws a clear network error.
      const ok = await testProviderKey("ollama", ollamaEndpoint());
      status.ollama = ok ? { kind: "valid" } : { kind: "invalid" };
    } catch (e) {
      status.ollama = { kind: "failed", message: String(e) };
    }
  }

  async function setDefaultProvider(id: ProviderId) {
    settings.defaultProvider = id;
    // Use this provider's remembered/hardcoded default model so it's immediately
    // usable, rather than clearing it. (Ollama has no hardcoded default — the
    // user picks from their pulled models, loaded below.)
    settings.defaultModel =
      settings.providers[id].defaultModel ?? DEFAULT_MODELS[id] ?? null;
    onChange();
    await loadModels();
  }

  function setDefaultModel(model: string) {
    settings.defaultModel = model || null;
    onChange();
  }

  async function loadModels() {
    const p = settings.defaultProvider;
    manualModel = false;
    modelsError = "";
    if (!ready(p)) {
      models = [];
      modelsState = "idle";
      return;
    }
    modelsState = "loading";
    try {
      models = await listProviderModels(p);
      modelsState = "idle";
      // An empty list (e.g. Ollama with no models pulled yet) → let the user
      // type a model id, with a neutral hint rather than an error.
      if (models.length === 0) manualModel = true;
    } catch (e) {
      // Fall back to manual model entry if the list API is unavailable
      // (cloud list endpoint down, or Ollama not running).
      models = [];
      modelsState = "error";
      modelsError = String(e);
      manualModel = true;
    }
  }

  // Model options including the current default even if it's not in the list.
  const modelOptions = $derived(
    settings.defaultModel && !models.includes(settings.defaultModel)
      ? [settings.defaultModel, ...models]
      : models,
  );
</script>

<div class="section">
  <h2 class="section__title">Providers</h2>
  {#if loadError}
    <p class="error">{loadError}</p>
  {/if}
  {#each providers as p (p.id)}
    <div class="field">
      <div class="row">
        <span class="label grow">{p.displayName}</span>
        <span class="hint" class:error={status[p.id]?.kind === "failed"}>
          {#if status[p.id]?.kind === "failed"}
            {(status[p.id] as { message: string }).message}
          {:else}
            {STATUS_LABEL[status[p.id]?.kind ?? "none"]}
          {/if}
        </span>
      </div>
      {#if isOllama(p.id)}
        <!-- Ollama is local + keyless: an endpoint URL, not an API key. -->
        <div class="row">
          <input
            class="input grow"
            type="text"
            autocomplete="off"
            spellcheck="false"
            placeholder={DEFAULT_OLLAMA_ENDPOINT}
            bind:value={endpointInput}
            aria-label="Ollama server endpoint URL"
          />
          <button class="btn" onclick={saveEndpoint}>Save</button>
          <button class="btn" onclick={testEndpoint}>Test</button>
        </div>
        <p class="hint">Local server — no API key needed. Run Ollama and pull a model.</p>
      {:else}
        <div class="row">
          <input
            class="input grow"
            type="password"
            autocomplete="off"
            spellcheck="false"
            placeholder={keyPresence[p.id] ? "Key saved — type to replace" : "API key"}
            bind:value={keyInput[p.id]}
            aria-label="{p.displayName} API key"
          />
          <button class="btn" onclick={() => save(p.id)} disabled={!keyInput[p.id]?.trim()}>
            Save
          </button>
          <button
            class="btn"
            onclick={() => test(p.id)}
            disabled={!keyInput[p.id]?.trim() && !keyPresence[p.id]}
          >
            Test
          </button>
          <button class="btn" onclick={() => remove(p.id)} disabled={!keyPresence[p.id]}>
            Remove
          </button>
        </div>
      {/if}
    </div>
  {/each}
</div>

<div class="section">
  <h2 class="section__title">Models</h2>

  <div class="field">
    <label class="label" for="default-provider">Default provider</label>
    <select
      id="default-provider"
      class="select"
      value={settings.defaultProvider}
      onchange={(e) => setDefaultProvider(e.currentTarget.value as ProviderId)}
    >
      {#each providers as p (p.id)}
        <option value={p.id} disabled={!ready(p.id)}>
          {p.displayName}{ready(p.id) ? "" : " (no key)"}
        </option>
      {/each}
    </select>
  </div>

  <div class="field">
    <label class="label" for="default-model">Default model</label>
    {#if !ready(settings.defaultProvider)}
      <p class="hint">Add an API key for this provider to choose a model.</p>
    {:else if modelsState === "loading"}
      <p class="hint">Loading models…</p>
    {:else if manualModel}
      <input
        id="default-model"
        class="input"
        placeholder="Enter a model id (e.g. gpt-4o)"
        value={settings.defaultModel ?? ""}
        onchange={(e) => setDefaultModel(e.currentTarget.value.trim())}
      />
      {#if modelsError}
        <p class="hint error">Couldn't load the model list: {modelsError}</p>
      {:else}
        <p class="hint">No models found — type a model id, or pull one (e.g. <code>ollama pull llama3</code>).</p>
      {/if}
      <div class="row">
        <button class="btn" onclick={loadModels}>Retry</button>
      </div>
    {:else}
      <select
        id="default-model"
        class="select"
        value={settings.defaultModel ?? ""}
        onchange={(e) => setDefaultModel(e.currentTarget.value)}
      >
        <option value="" disabled>Choose a model…</option>
        {#each modelOptions as model (model)}
          <option value={model}>{model}</option>
        {/each}
      </select>
    {/if}
  </div>
</div>
