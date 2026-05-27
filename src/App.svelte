<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { isTauri } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { info } from "@tauri-apps/plugin-log";
  import { appVersion, getSettings, type Language } from "./lib/tauri";
  import Settings from "./Settings.svelte";
  import Translate from "./Translate.svelte";
  import Notifications from "./Notifications.svelte";

  // Esc dismisses the panel (same as clicking outside / losing focus), from any
  // view. Hiding triggers the window's blur handler, which remembers position.
  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && isTauri()) {
      getCurrentWindow()
        .hide()
        .catch(() => {});
    }
  }

  // The whole app is one window. Navigation between the translate view, the
  // Settings view, and the Notifications view is just a state swap here — no
  // second window. See src-tauri/src/windows.rs for why TLiquid is single-window.
  type View = "translate" | "settings" | "notifications";

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

  // Whether the one-time launch-at-login consent has been answered (P1-001).
  // Until it has, the bell shows a badge and the Notifications view offers it.
  // Assume answered until settings load so the badge doesn't flash for users
  // who already responded. Future: also surface new-version alerts here (P2-007).
  let startupPrompted = $state(true);
  const notificationCount = $derived(startupPrompted ? 0 : 1);

  // Switch views; manual navigation drops any pending hotkey request so returning
  // to translate doesn't replay an old capture.
  function goTo(next: View) {
    view = next;
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

    // Has the launch-at-login consent been answered? Drives the bell badge.
    try {
      const settings = await getSettings();
      startupPrompted = settings.startup.prompted;
    } catch {
      /* leave assumed-answered (no badge) if settings can't load */
    }

    // The tray "Settings…" item asks the panel to switch views.
    await listen<View>("navigate", (event) => {
      view = event.payload;
    });

    // A selected-text hotkey summoned the panel (shortcuts.rs) with the captured
    // selection, a capture error, or nothing selected (text/error both null →
    // the panel just opens empty for manual typing). Always routes to translate.
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
  <!-- Frameless window: this slim bar is the drag handle and houses the bell +
       gear (on translate) or a back arrow (on settings/notifications). -->
  <header class="titlebar" data-tauri-drag-region>
    {#if view === "translate"}
      <button
        class="icon-btn bell"
        title={notificationCount > 0
          ? `Notifications (${notificationCount} new)`
          : "Notifications"}
        aria-label={notificationCount > 0
          ? `Notifications (${notificationCount} new)`
          : "Notifications"}
        onclick={() => goTo("notifications")}
      >
        <svg
          width="15"
          height="15"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          aria-hidden="true"
        >
          <path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9"></path>
          <path d="M13.73 21a2 2 0 0 1-3.46 0"></path>
        </svg>
        {#if notificationCount > 0}
          <span class="bell-dot" aria-hidden="true"></span>
        {/if}
      </button>
      <button
        class="icon-btn"
        title="Settings"
        aria-label="Open settings"
        onclick={() => goTo("settings")}
      >
        ⚙
      </button>
    {:else}
      <button
        class="icon-btn"
        title="Back"
        aria-label="Back to translate"
        onclick={() => goTo("translate")}
      >
        ←
      </button>
    {/if}
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
      onOpenSettings={() => goTo("settings")}
    />
    <Settings {version} hidden={view !== "settings"} />
    <Notifications
      hidden={view !== "notifications"}
      {startupPrompted}
      onAnswered={() => (startupPrompted = true)}
    />
  {/if}
</div>

<style>
  /* Unread badge on the notification bell. */
  .bell {
    position: relative;
  }
  .bell-dot {
    position: absolute;
    top: 3px;
    right: 3px;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--tl-accent);
    border: 1px solid var(--tl-surface);
  }
</style>
