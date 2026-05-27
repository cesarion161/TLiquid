<script lang="ts">
  import { onMount } from "svelte";
  import { isTauri } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { info } from "@tauri-apps/plugin-log";
  import { appVersion } from "./lib/tauri";
  import Settings from "./Settings.svelte";
  import Translate from "./Translate.svelte";

  // The whole app is one window. Navigation between the translate view and the
  // Settings view is just a state swap here — no second window. See
  // src-tauri/src/windows.rs for why TLiquid is single-window.
  type View = "translate" | "settings";

  // A selected-text hotkey delivers the captured text (or a capture error) here;
  // we route to the translate view and hand it to <Translate>. A monotonic `id`
  // lets the child process each request once (P0-014/P0-015).
  type ShortcutRequest = {
    action: "primary" | "secondary";
    text: string | null;
    error: string | null;
    id: number;
  };

  let view = $state<View>("translate");
  let version = $state("…");
  let error = $state<string | null>(null);
  let shortcutRequest = $state<ShortcutRequest | null>(null);
  let seq = 0;

  function toggleView() {
    view = view === "settings" ? "translate" : "settings";
    // Manual navigation: drop any pending hotkey request so returning to the
    // translate view doesn't replay an old capture.
    shortcutRequest = null;
  }

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

    // A global shortcut summoned the panel (shortcuts.rs). "open" just shows the
    // translate view; "primary"/"secondary" carry the captured selection.
    await listen<{
      action: "open" | "primary" | "secondary";
      text: string | null;
      error: string | null;
    }>("shortcut", (event) => {
      const p = event.payload;
      view = "translate";
      if (p.action === "open") {
        shortcutRequest = null;
      } else {
        shortcutRequest = { action: p.action, text: p.text, error: p.error, id: ++seq };
      }
    });
  });
</script>

<div class="panel">
  <!-- Frameless window: this slim bar is the drag handle and houses the gear/back. -->
  <header class="titlebar" data-tauri-drag-region>
    <button
      class="icon-btn"
      title={view === "settings" ? "Back" : "Settings"}
      aria-label={view === "settings" ? "Back to translate" : "Open settings"}
      onclick={toggleView}
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
    <!-- Manual translation surface (PRD §10.4/§10.5). Owns its own state and the
         real provider call; remounts on each switch back so it re-reads settings.
         `request` carries a selected-text hotkey capture to translate (P0-014/15). -->
    <Translate
      request={shortcutRequest}
      onOpenSettings={() => {
        view = "settings";
        // Drop the handled request so returning to translate doesn't bounce back.
        shortcutRequest = null;
      }}
    />
  {/if}
</div>
