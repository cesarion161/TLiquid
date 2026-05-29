<script lang="ts">
  // Updates + About sections of Settings (P0-018 → P2-007 manual → P2-013 auto).
  //
  // Updates: a "Check for updates" button (always works), a state line, and —
  // when an update exists — "Download & install" (fetches a minisign-verified
  // bundle, installs in place, relaunches; FR-060–063). A toggle controls the
  // background auto-check (P2-013, default ON; check-only). The available-update
  // UI is driven by the `update` prop so it shows whether the update was found by
  // the manual check or the background poll (single source of truth in App).
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { checkForUpdate, type Settings, type UpdateStatus } from "./lib/tauri";
  import { runUpdateInstall, phaseLabel, type InstallPhase } from "./lib/updates";

  let {
    version = "—",
    // The pending update (from a manual check or the background poll), or null.
    update = null,
    // Loaded settings (null until loaded); needed for the auto-check toggle.
    settings = null,
    // Bubble a found/cleared update up to App so the bell badge reflects it.
    onUpdateAvailable = () => {},
    // Persist settings after the toggle changes.
    onChange = () => {},
  }: {
    version?: string;
    update?: UpdateStatus | null;
    settings?: Settings | null;
    onUpdateAvailable?: (status: UpdateStatus | null) => void;
    onChange?: () => void;
  } = $props();

  const REPO_URL = "https://github.com/cesarion161/TLiquid";
  const RELEASES_URL = `${REPO_URL}/releases`;

  let checking = $state(false);
  let checkError = $state<string | null>(null);
  // True after a manual check that found nothing — drives the "Up to date" line.
  let checkedClean = $state(false);
  let install = $state<InstallPhase>({ kind: "idle" });

  const installing = $derived(
    install.kind === "downloading" || install.kind === "installing",
  );

  async function check() {
    if (checking || installing) return;
    checking = true;
    checkError = null;
    checkedClean = false;
    install = { kind: "idle" };
    try {
      const status = await checkForUpdate();
      // Funnel the result into App's pending-update state (lights/clears bell);
      // it flows back via the `update` prop and drives the install affordance.
      onUpdateAvailable(status.available ? status : null);
      checkedClean = !status.available;
    } catch (e) {
      checkError = String(e);
    } finally {
      checking = false;
    }
  }

  async function downloadAndInstall() {
    if (installing) return;
    install = { kind: "downloading", downloaded: 0, total: null };
    // On success the app relaunches and this never returns; failure sets `error`.
    await runUpdateInstall((p) => (install = p));
  }

  function toggleAutoCheck(enabled: boolean) {
    if (!settings) return;
    settings.updates.autoCheck = enabled;
    onChange();
  }

  function openExternal(url: string) {
    openUrl(url).catch(() => {
      /* best-effort; the URL is also shown for manual navigation. */
    });
  }
</script>

<div class="section">
  <h2 class="section__title">Updates</h2>

  <div class="row">
    <button class="btn" onclick={check} disabled={checking || installing}>
      {checking ? "Checking…" : "Check for updates"}
    </button>
    {#if checkedClean && !update?.available && !installing}
      <span class="hint" role="status">Up to date (v{version})</span>
    {/if}
  </div>

  {#if checkError}
    <p class="error">Could not check for updates: {checkError}</p>
  {/if}

  {#if update?.available}
    <div class="field">
      <span class="label">Update available: v{update.version}</span>
      {#if update.notes}
        <p class="hint notes">{update.notes}</p>
      {/if}
      <div class="row">
        <button
          class="btn btn--primary"
          onclick={downloadAndInstall}
          disabled={installing}
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

  {#if settings}
    <label class="row check">
      <input
        type="checkbox"
        checked={settings.updates.autoCheck}
        onchange={(e) => toggleAutoCheck(e.currentTarget.checked)}
      />
      <span>Automatically check for new updates</span>
    </label>
  {/if}

  <p class="hint">
    When on, T·Liquid checks GitHub for a newer version on startup and every few
    hours — it only notifies you (via the 🔔 bell); you always click to install.
    Updates are signed and installed in place — no manual reinstall.
    <button class="linklike" onclick={() => openExternal(RELEASES_URL)}>
      View all releases
    </button>
  </p>
</div>

<div class="section">
  <h2 class="section__title">About</h2>
  <p class="hint">T·Liquid v{version}</p>
  <p class="hint">
    An open-source (MIT), macOS-first, bring-your-own-key LLM translator that
    lives in your menu bar.
  </p>
  <div class="row">
    <button class="btn" onclick={() => openExternal(REPO_URL)}>Source on GitHub</button>
  </div>
</div>

<style>
  /* Release notes can be multi-line; keep them readable but compact. */
  .notes {
    white-space: pre-wrap;
    max-height: 6rem;
    overflow-y: auto;
  }
  /* Checkbox + label row: align the box with the text, allow a normal cursor. */
  .check {
    cursor: pointer;
    gap: var(--tl-sp-2);
  }
  .check input {
    margin: 0;
  }
  /* An inline text button that reads like a link (for "View all releases"). */
  .linklike {
    background: none;
    border: none;
    padding: 0;
    color: var(--tl-accent);
    font: inherit;
    cursor: pointer;
    text-decoration: underline;
  }
</style>
