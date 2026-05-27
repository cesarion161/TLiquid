<script lang="ts">
  // Updates + About sections of Settings (P0-018 → P2-007, PRD §10.6.7/§10.6.8).
  //
  // P2-007 makes Updates real: a "Check for updates" button checks GitHub
  // Releases (never in the background — that's the P2-013 toggle), shows the
  // state, and when an update exists offers "Download & install", which fetches
  // a minisign-verified bundle, installs it in place, and relaunches (FR-060–063).
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { checkForUpdate, type UpdateStatus } from "./lib/tauri";
  import { runUpdateInstall, phaseLabel, type InstallPhase } from "./lib/updates";

  let {
    version = "—",
    // Bubble a found/cleared update up to App so the notification bell reflects it.
    onUpdateAvailable = () => {},
  }: {
    version?: string;
    onUpdateAvailable?: (status: UpdateStatus | null) => void;
  } = $props();

  const REPO_URL = "https://github.com/cesarion161/TLiquid";
  const RELEASES_URL = `${REPO_URL}/releases`;

  // Manual check state. `status` is null until the user checks at least once.
  let checking = $state(false);
  let status = $state<UpdateStatus | null>(null);
  let checkError = $state<string | null>(null);
  let install = $state<InstallPhase>({ kind: "idle" });

  const installing = $derived(
    install.kind === "downloading" || install.kind === "installing",
  );

  async function check() {
    if (checking || installing) return;
    checking = true;
    checkError = null;
    install = { kind: "idle" };
    try {
      status = await checkForUpdate();
      // Light (or clear) the bell badge based on the result.
      onUpdateAvailable(status.available ? status : null);
    } catch (e) {
      status = null;
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
    {#if status && !status.available && !installing}
      <span class="hint" role="status">Up to date (v{status.currentVersion})</span>
    {/if}
  </div>

  {#if checkError}
    <p class="error">Could not check for updates: {checkError}</p>
  {/if}

  {#if status?.available}
    <div class="field">
      <span class="label">Update available: v{status.version}</span>
      {#if status.notes}
        <p class="hint notes">{status.notes}</p>
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

  <p class="hint">
    TLiquid updates are signed and installed in place — no manual reinstall.
    <button class="linklike" onclick={() => openExternal(RELEASES_URL)}>
      View all releases
    </button>
  </p>
</div>

<div class="section">
  <h2 class="section__title">About</h2>
  <p class="hint">TLiquid v{version}</p>
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
