<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { isTauri } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { info } from "@tauri-apps/plugin-log";
  import {
    appVersion,
    getSettings,
    type Language,
    type UpdateStatus,
  } from "./lib/tauri";
  import Settings from "./Settings.svelte";
  import Translate from "./Translate.svelte";
  import Notifications from "./Notifications.svelte";

  // Esc backs out of an open overlay (Settings/Notifications) first; from the
  // translate view it dismisses the whole panel (like clicking outside / blur,
  // whose handler also remembers the window position).
  function onKeydown(e: KeyboardEvent) {
    if (e.key !== "Escape" || !isTauri()) return;
    if (view !== "translate") {
      goTo("translate");
    } else {
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
  // who already responded.
  let startupPrompted = $state(true);

  // The update found by the most recent check (P2-007), shared so both the bell
  // badge and the Notifications pane reflect it. Null = no update pending.
  let pendingUpdate = $state<UpdateStatus | null>(null);

  // Bell badge: pending launch-at-login consent + an available update.
  const notificationCount = $derived(
    (startupPrompted ? 0 : 1) + (pendingUpdate?.available ? 1 : 0),
  );

  // Settings/Notifications render as an overlay pane over the translate base
  // (P2-012), rather than replacing it — so the panel doesn't look sparse when
  // resized large.
  const overlayOpen = $derived(view !== "translate");

  // macOS panel translucency (P2-012). The window has a vibrancy layer applied
  // in Rust; flipping this class makes the panel background transparent so it
  // shows through. Initialized from settings on mount; toggled from Appearance.
  let translucent = $state(true);
  $effect(() => {
    document.body.classList.toggle("translucent", translucent);
  });
  function setTranslucent(enabled: boolean) {
    translucent = enabled;
  }

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
    // Also pick up the saved translucency preference for the body class.
    try {
      const settings = await getSettings();
      startupPrompted = settings.startup.prompted;
      translucent = settings.ui.translucent;
    } catch {
      /* leave assumed-answered (no badge) if settings can't load */
    }

    // The tray "Settings…" item asks the panel to switch views.
    await listen<View>("navigate", (event) => {
      view = event.payload;
    });

    // The background auto-update poll (P2-013) found a newer version; light the
    // bell + surface it in Notifications/Updates. Same shape as a manual check.
    await listen<UpdateStatus>("update-available", (event) => {
      pendingUpdate = event.payload;
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
  <!-- Frameless window: this slim bar is the drag handle. Left: product name +
       version. Right: bell + gear (on translate) or a back arrow (elsewhere). -->
  <header class="titlebar" data-tauri-drag-region>
    <span class="titlebar-title">
      TLiquid <span class="titlebar-version">v{version}</span>
    </span>
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
          width="16"
          height="16"
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
        <svg
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          aria-hidden="true"
        >
          <circle cx="12" cy="12" r="3"></circle>
          <path
            d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"
          ></path>
        </svg>
      </button>
    {:else}
      <button
        class="icon-btn"
        title="Back"
        aria-label="Back to translate"
        onclick={() => goTo("translate")}
      >
        <svg
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          aria-hidden="true"
        >
          <line x1="19" y1="12" x2="5" y2="12"></line>
          <polyline points="12 19 5 12 12 5"></polyline>
        </svg>
      </button>
    {/if}
  </header>

  {#if error}
    <section class="body">
      <p class="error">{error}</p>
    </section>
  {:else}
    <!-- Translate is the always-present base layer (keeps its typed text +
         result). Settings/Notifications slide in as a fixed-width overlay pane
         over part of it (P2-012), so the panel doesn't look sparse when resized
         large. All three stay mounted; the overlay's children toggle by `hidden`.
         `request` carries a selected-text hotkey capture to translate. -->
    <div class="stage">
      <Translate
        active={view === "translate"}
        request={shortcutRequest}
        onOpenSettings={() => goTo("settings")}
      />

      {#if overlayOpen}
        <!-- Dim the uncovered base; click anywhere on it returns to translate. -->
        <button
          type="button"
          class="scrim"
          aria-label="Close"
          onclick={() => goTo("translate")}
        ></button>
      {/if}

      <div class="overlay" class:open={overlayOpen} aria-hidden={!overlayOpen}>
        <Settings
          {version}
          hidden={view !== "settings"}
          update={pendingUpdate}
          onUpdateAvailable={(s) => (pendingUpdate = s)}
          onTranslucencyChange={setTranslucent}
        />
        <Notifications
          hidden={view !== "notifications"}
          {startupPrompted}
          update={pendingUpdate}
          onAnswered={() => (startupPrompted = true)}
        />
      </div>
    </div>
  {/if}
</div>

<style>
  /* Product name + version on the left of the titlebar; pushes the action
     buttons to the right (margin-right: auto). Part of the drag region. */
  .titlebar-title {
    margin-right: auto;
    padding-left: var(--tl-sp-2);
    font-size: var(--tl-fs-sm);
    font-weight: 600;
    color: var(--tl-text);
    white-space: nowrap;
  }
  .titlebar-version {
    font-weight: 400;
    color: var(--tl-text-muted);
  }

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
