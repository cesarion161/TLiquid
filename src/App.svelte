<script lang="ts">
  import { onMount } from "svelte";
  import { isTauri } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { info } from "@tauri-apps/plugin-log";
  import { appVersion } from "./lib/tauri";
  import Settings from "./Settings.svelte";
  import Result from "./Result.svelte";

  // The whole app is one window. Navigation between the translate view and the
  // Settings view is just a state swap here — no second window. See
  // src-tauri/src/windows.rs for why TLiquid is single-window.
  type View = "translate" | "settings";

  let view = $state<View>("translate");
  let version = $state("…");
  let error = $state<string | null>(null);

  onMount(async () => {
    // The Tauri runtime is only present inside the app's own webview. Opening
    // this page in a regular browser (e.g. http://localhost:1420) has no IPC,
    // so guard rather than throwing a raw TypeError.
    if (!isTauri()) {
      version = "—";
      error =
        "Not running inside the TLiquid app. Launch it with `pnpm tauri dev` and open it from the menu-bar icon — don't open the dev URL in a browser.";
      return;
    }

    try {
      version = await appVersion();
      await info(`TLiquid panel ready (v${version})`);
    } catch (e) {
      error = String(e);
    }

    // The tray "Settings…" item asks the panel to switch views.
    await listen<View>("navigate", (event) => {
      view = event.payload;
    });
  });
</script>

<div class="panel">
  <!-- Frameless window: this bar is the drag handle and houses the gear/back. -->
  <header class="titlebar" data-tauri-drag-region>
    <span class="title">TLiquid</span>
    <button
      class="icon-btn"
      title={view === "settings" ? "Back" : "Settings"}
      aria-label={view === "settings" ? "Back to translate" : "Open settings"}
      onclick={() => (view = view === "settings" ? "translate" : "settings")}
    >
      {view === "settings" ? "←" : "⚙"}
    </button>
  </header>

  {#if error}
    <section class="body">
      <p class="error">{error}</p>
    </section>
  {:else if view === "settings"}
    <Settings {version} />
  {:else}
    <!-- Manual translation surface (PRD §10.5). This view's layout is the UI
         foundation (P0-003); the controls are inert here and get wired to a
         real provider call in P0-011 (translate) / P0-012 (result + copy). -->
    <section class="body">
      <div class="field">
        <label class="label" for="source-text">Text</label>
        <textarea
          id="source-text"
          class="textarea"
          placeholder="Type or paste text to translate…"
          disabled
        ></textarea>
      </div>

      <div class="row">
        <div class="field grow">
          <label class="label" for="target-lang">Target</label>
          <select id="target-lang" class="select" disabled>
            <option>Auto (primary / secondary)</option>
          </select>
        </div>
        <button class="btn btn--primary" disabled>Translate</button>
      </div>

      <Result />
    </section>
  {/if}
</div>
