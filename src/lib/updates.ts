// Shared update-install flow (P2-007), used by both the Settings → Updates
// section and the Notifications pane so the channel/progress wiring lives in one
// place. Lighting the bell and the "Check for updates" call stay in the UI; this
// module owns only the download → verify → install → relaunch action.
import {
  Channel,
  downloadAndInstallUpdate,
  type DownloadProgress,
} from "./tauri";

/** Phase of an in-progress install, surfaced to the UI for a progress display. */
export type InstallPhase =
  | { kind: "idle" }
  | { kind: "downloading"; downloaded: number; total: number | null }
  | { kind: "installing" }
  | { kind: "error"; message: string };

/**
 * Download, verify, and install the pending update, relaunching on success.
 * Reports progress via `onPhase`. On success the app restarts (the underlying
 * promise never resolves); on failure it reports an `error` phase and resolves
 * so the caller can re-enable the button.
 */
export async function runUpdateInstall(
  onPhase: (phase: InstallPhase) => void,
): Promise<void> {
  const channel = new Channel<DownloadProgress>();
  channel.onmessage = (p) =>
    onPhase({ kind: "downloading", downloaded: p.downloaded, total: p.total });
  try {
    await downloadAndInstallUpdate(channel);
    // Reached only if the relaunch hasn't torn down the webview yet.
    onPhase({ kind: "installing" });
  } catch (e) {
    onPhase({ kind: "error", message: String(e) });
  }
}

/** Human-readable, compact progress label for an install phase. */
export function phaseLabel(phase: InstallPhase): string {
  switch (phase.kind) {
    case "downloading": {
      if (phase.total && phase.total > 0) {
        const pct = Math.min(
          100,
          Math.round((phase.downloaded / phase.total) * 100),
        );
        return `Downloading… ${pct}%`;
      }
      return "Downloading…";
    }
    case "installing":
      return "Installing…";
    case "error":
      return phase.message;
    case "idle":
      return "";
  }
}
