<script lang="ts">
  // Notifications view of the panel (reached by the bell in the titlebar). Hosts
  // actionable notices without shifting the translate UI. Today that's the
  // one-time launch-at-login consent (P1-001, FR-054); new-version alerts are a
  // placeholder until the update check ships in Phase 2 (P2-007 — Phase 0/1 make
  // no automatic update checks, FR-056).
  import { setLaunchAtLogin } from "./lib/tauri";

  let {
    hidden = false,
    startupPrompted = true,
    onAnswered,
  }: {
    hidden?: boolean;
    // False until the user has answered the launch-at-login consent once.
    startupPrompted?: boolean;
    // Called after the user answers, so the parent clears the bell badge.
    onAnswered: () => void;
  } = $props();

  let busy = $state(false);
  let error = $state<string | null>(null);

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
</script>

<section class="body" class:hidden>
  <div class="section">
    <h2 class="section__title">Notifications</h2>

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
    {:else}
      <p class="hint">
        You're all caught up — no notifications. New-version alerts will appear
        here once update checks ship (a later version).
      </p>
    {/if}
  </div>
</section>
