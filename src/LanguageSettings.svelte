<script lang="ts">
  // Languages section of Settings (P0-006, PRD §10.6.1).
  //
  // Mutates the shared `settings` object (a Svelte $state proxy owned by
  // Settings.svelte) and calls `onChange` to persist. Rules enforced here keep
  // the model consistent: primary is mandatory, a language can occupy only one
  // slot (primary / secondary / additional), and primary ≠ secondary.
  import type { Settings, Language } from "./lib/tauri";
  import { COMMON_LANGUAGES, languageByCode } from "./lib/languages";

  let { settings, onChange }: { settings: Settings; onChange: () => void } =
    $props();

  // Code chosen in the "add language" picker before pressing Add.
  let addCode = $state("");

  const codesInUse = $derived(
    new Set<string>([
      settings.languages.primary.code,
      ...(settings.languages.secondary ? [settings.languages.secondary.code] : []),
      ...settings.languages.additional.map((l) => l.code),
    ]),
  );

  // Languages still free to add to the additional list.
  const addable = $derived(
    COMMON_LANGUAGES.filter((l) => !codesInUse.has(l.code)),
  );

  // Primary options: the full list, plus the current primary if a hand-edited
  // config used a code outside COMMON_LANGUAGES (FR-047).
  const primaryOptions = $derived(
    COMMON_LANGUAGES.some((l) => l.code === settings.languages.primary.code)
      ? [...COMMON_LANGUAGES]
      : [settings.languages.primary, ...COMMON_LANGUAGES],
  );

  // Secondary options: every language except the primary, plus the current
  // secondary if it's outside COMMON_LANGUAGES.
  const secondaryOptions = $derived.by((): Language[] => {
    const primaryCode = settings.languages.primary.code;
    const sec = settings.languages.secondary;
    const base = COMMON_LANGUAGES.filter((l) => l.code !== primaryCode);
    if (sec && sec.code !== primaryCode && !base.some((l) => l.code === sec.code)) {
      return [sec, ...base];
    }
    return base;
  });

  function setPrimary(code: string) {
    settings.languages.primary = languageByCode(code);
    // A language can't be both primary and secondary/additional.
    if (settings.languages.secondary?.code === code) {
      settings.languages.secondary = null;
    }
    settings.languages.additional = settings.languages.additional.filter(
      (l) => l.code !== code,
    );
    onChange();
  }

  function setSecondary(code: string) {
    if (code === "") {
      settings.languages.secondary = null;
    } else {
      settings.languages.secondary = languageByCode(code);
      settings.languages.additional = settings.languages.additional.filter(
        (l) => l.code !== code,
      );
    }
    onChange();
  }

  function addLanguage() {
    if (!addCode || codesInUse.has(addCode)) return;
    settings.languages.additional = [
      ...settings.languages.additional,
      { ...languageByCode(addCode), enabled: true },
    ];
    addCode = "";
    onChange();
  }

  function removeAdditional(code: string) {
    settings.languages.additional = settings.languages.additional.filter(
      (l) => l.code !== code,
    );
    onChange();
  }

  function move(index: number, delta: number) {
    const list = settings.languages.additional;
    const target = index + delta;
    if (target < 0 || target >= list.length) return;
    const next = [...list];
    [next[index], next[target]] = [next[target], next[index]];
    settings.languages.additional = next;
    onChange();
  }
</script>

<div class="section">
  <h2 class="section__title">Languages</h2>

  <div class="field">
    <label class="label" for="primary-lang">Primary (required)</label>
    <select
      id="primary-lang"
      class="select"
      value={settings.languages.primary.code}
      onchange={(e) => setPrimary(e.currentTarget.value)}
    >
      {#each primaryOptions as lang (lang.code)}
        <option value={lang.code}>{lang.name}</option>
      {/each}
    </select>
  </div>

  <div class="field">
    <label class="label" for="secondary-lang">Secondary (optional)</label>
    <select
      id="secondary-lang"
      class="select"
      value={settings.languages.secondary?.code ?? ""}
      onchange={(e) => setSecondary(e.currentTarget.value)}
    >
      <option value="">None</option>
      {#each secondaryOptions as lang (lang.code)}
        <option value={lang.code}>{lang.name}</option>
      {/each}
    </select>
  </div>

  <div class="field">
    <span class="label">Additional</span>
    {#if settings.languages.additional.length === 0}
      <p class="hint">No additional languages yet.</p>
    {:else}
      {#each settings.languages.additional as lang, i (lang.code)}
        <div class="row">
          <span class="grow">{lang.name}</span>
          <button
            class="icon-btn"
            onclick={() => move(i, -1)}
            disabled={i === 0}
            aria-label="Move {lang.name} up"
            title="Move up">↑</button
          >
          <button
            class="icon-btn"
            onclick={() => move(i, 1)}
            disabled={i === settings.languages.additional.length - 1}
            aria-label="Move {lang.name} down"
            title="Move down">↓</button
          >
          <button
            class="icon-btn"
            onclick={() => removeAdditional(lang.code)}
            aria-label="Remove {lang.name}"
            title="Remove">✕</button
          >
        </div>
      {/each}
    {/if}

    <div class="row">
      <select
        class="select grow"
        bind:value={addCode}
        disabled={addable.length === 0}
        aria-label="Add an additional language"
      >
        <option value="">Add a language…</option>
        {#each addable as lang (lang.code)}
          <option value={lang.code}>{lang.name}</option>
        {/each}
      </select>
      <button
        class="btn"
        onclick={addLanguage}
        disabled={!addCode || codesInUse.has(addCode)}>Add</button
      >
    </div>
  </div>
</div>
