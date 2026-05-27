<script lang="ts">
  // Notifications view of the panel (reached by the bell in the titlebar). Hosts
  // actionable notices without shifting the translate UI:
  //  - the one-time launch-at-login consent (P1-001, FR-054);
  //  - a "new version available" alert with Download & install (P2-007, FR-061).
  import { setLaunchAtLogin, type UpdateStatus } from "./lib/tauri";
  import { runUpdateInstall, phaseLabel, type InstallPhase } from "./lib/updates";

  let {
    hidden = false,
    startupPrompted = true,
    update = null,
    onAnswered,
  }: {
    hidden?: boolean;
    // False until the user has answered the launch-at-login consent once.
    startupPrompted?: boolean;
    // The available update (P2-007), or null when none is pending.
    update?: UpdateStatus | null;
    // Called after the user answers the consent, so the parent clears the badge.
    onAnswered: () => void;
  } = $props();

  let busy = $state(false);
  let error = $state<string | null>(null);

  // Install state for the in-pane "Download & install" (shares App's pending
  // update; the actual download/relaunch lives in lib/updates).
  let install = $state<InstallPhase>({ kind: "idle" });
  const installing = $derived(
    install.kind === "downloading" || install.kind === "installing",
  );

  // Is there anything to show? Drives the "all caught up" empty state.
  const hasNotices = $derived(!startupPrompted || !!update?.available);

  async function answer(enable: boolean) {
    if (busy) return;
    busy = true;
    error = null;
    try {
      // set_launch_at_login persists the choice (incl. marking it answered) and
      // applies the OS state; the consent is never enabled without this click.
      await setLaunchAtLogin(enable);
      onAnswered();
    } catch (e) {
      error = `Could not update launch-at-login: ${e}`;
    } finally {
      busy = false;
    }
  }

  async function downloadAndInstall() {
    if (installing) return;
    install = { kind: "downloading", downloaded: 0, total: null };
    // On success the app relaunches and this never returns; failure sets `error`.
    await runUpdateInstall((p) => (install = p));
  }
</script>

<section class="body" class:hidden>
  <div class="section">
    <h2 class="section__title">Notifications</h2>

    {#if update?.available}
      <div class="field">
        <span class="label">New version v{update.version} is available!</span>
        {#if update.notes}
          <p class="hint notes">{update.notes}</p>
        {/if}
        <div class="row">
          <button
            class="btn btn--primary"
            disabled={installing}
            onclick={downloadAndInstall}
          >
            Download &amp; install
          </button>
          {#if installing}
            <span class="hint" role="status">{phaseLabel(install)}</span>
          {/if}
        </div>
        {#if install.kind === "error"}
          <p class="error">Update failed: {install.message}</p>
        {/if}
      </div>
    {/if}

    {#if !startupPrompted}
      <div class="field">
        <span class="label">Launch TLiquid at login?</span>
        <p class="hint">
          Recommended — TLiquid starts in the menu bar, ready for the translate
          hotkey. You can change this anytime in Settings → Startup.
        </p>
        <div class="row">
          <button
            class="btn btn--primary"
            disabled={busy}
            onclick={() => answer(true)}>Enable</button
          >
          <button class="btn" disabled={busy} onclick={() => answer(false)}>
            Not now
          </button>
        </div>
        {#if error}
          <p class="error">{error}</p>
        {/if}
      </div>
    {/if}

    {#if !hasNotices}
      <p class="hint">
        You're all caught up — no notifications. New-version alerts appear here,
        and you can check anytime in Settings → Updates.
      </p>
    {/if}
  </div>
</section>

<style>
  /* Release notes can be multi-line; keep them readable but compact. */
  .notes {
    white-space: pre-wrap;
    max-height: 6rem;
    overflow-y: auto;
  }
</style>
