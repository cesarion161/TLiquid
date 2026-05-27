<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { isTauri } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { info } from "@tauri-apps/plugin-log";
  import { appVersion, type Language } from "./lib/tauri";
  import Settings from "./Settings.svelte";
  import Translate from "./Translate.svelte";

  // Esc dismisses the panel (same as clicking outside / losing focus), from any
  // view. Hiding triggers the window's blur handler, which remembers position.
  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && isTauri()) {
      getCurrentWindow()
        .hide()
        .catch(() => {});
    }
  }

  // The whole app is one window. Navigation between the translate view and the
  // Settings view is just a state swap here — no second window. See
  // src-tauri/src/windows.rs for why TLiquid is single-window.
  type View = "translate" | "settings";

  // A selected-text hotkey delivers the captured text (or a capture error) here;
  // we route to the translate view and hand it to <Translate>. A monotonic `id`
  // lets the child process each request once (P0-014/P0-015). An additional-
  // language shortcut (P1-002) carries its explicit `target` language.
  type ShortcutRequest = {
    action: "primary" | "secondary" | "explicit";
    text: string | null;
    error: string | null;
    target: Language | null;
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
    window.addEventListener("keydown", onKeydown);

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

    // A selected-text hotkey summoned the panel (shortcuts.rs) with the captured
    // selection (or a capture error). It only fires when there's something to act
    // on — a no-selection press is a silent no-op and never reaches here.
    await listen<{
      action: "primary" | "secondary" | "explicit";
      text: string | null;
      error: string | null;
      target: Language | null;
    }>("shortcut", (event) => {
      const p = event.payload;
      view = "translate";
      shortcutRequest = {
        action: p.action,
        text: p.text,
        error: p.error,
        target: p.target ?? null,
        id: ++seq,
      };
    });
  });

  onDestroy(() => window.removeEventListener("keydown", onKeydown));
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
  {:else}
    <!-- Both views stay mounted (hidden, not unmounted) so the translate view
         keeps its typed text and result while you visit Settings. Translate
         re-reads settings whenever it becomes active again, picking up changes.
         `request` carries a selected-text hotkey capture to translate. -->
    <Translate
      hidden={view !== "translate"}
      active={view === "translate"}
      request={shortcutRequest}
      onOpenSettings={() => {
        view = "settings";
        // Drop the handled request so returning to translate doesn't bounce back.
        shortcutRequest = null;
      }}
    />
    <Settings {version} hidden={view !== "settings"} />
  {/if}
</div>
