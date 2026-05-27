# TLiquid — Phase 0 QA / Release Candidate report (P0-021)

Date: 2026-05-27 · Branch: `main` · Version: 0.1.0 (macOS, Apple Silicon)

This is the Phase 0 end-to-end QA pass. It records what was verified automatically
and via a boot smoke test, and lists the interactive checks that require a human
on a Mac with a real provider API key (the parts an automated/CI environment
can't exercise).

## 1. Automated gate (matches CI, `.github/workflows/ci.yml`)

| Step | Result |
|---|---|
| `pnpm check` (svelte-check) | ✅ 0 errors, 0 warnings (121 files) |
| `cargo fmt --check` | ✅ clean |
| `cargo clippy --all-targets -- -D warnings` | ✅ clean |
| `cargo test` | ✅ 58 passed, 0 failed |
| `pnpm tauri build --no-bundle` | ✅ release builds, "Built application" |

Test coverage spans the privacy-sensitive/logic-heavy areas per the agent
guidance: language routing + orchestrator (`languages`, `translation`), config
load/save/corrupt-backup (`config`), Keychain mapping + no-key-in-error
(`secrets`), provider response parsing + error normalization + the privacy
host-allowlist/`reqwest`-confinement guards (`providers`), capture decision logic
(`capture`), and the diagnostics no-secret-fields guard (`diagnostics`).

## 2. IPC surface — fully wired

All 16 `#[tauri::command]` functions are defined, registered in the
`invoke_handler!` macro (`lib.rs`), and exposed via a typed wrapper in
`src/lib/tauri.ts` — cross-checked, zero mismatches:

`app_version, list_providers, get_settings, save_settings, settings_path,
set_provider_key, delete_provider_key, has_provider_key, test_provider_key,
test_provider_connection, list_provider_models, translate, apply_shortcuts,
shortcut_errors, open_accessibility_settings, diagnostics`

## 3. Boot smoke test (runtime)

`pnpm tauri dev` launched the app: `applicationDidFinishLaunching` fired, the
panel window was created, and the webview logged **`TLiquid panel ready
(v0.1.0)`** — confirming the frontend mounted and the `app_version` IPC round-trip
works at runtime. No panic/error. (The single "web content process terminated →
reload" line is Vite's normal dependency-optimization reload, not a crash.)

## 4. Privacy / no-telemetry (FR-019/020/051/052/056/064/067)

- The only `http(s)` hosts referenced anywhere in `src-tauri/src` are the four
  provider APIs (`api.openai.com`, `api.anthropic.com`,
  `generativelanguage.googleapis.com`, `openrouter.ai`). No TLiquid/telemetry host.
- `reqwest` is confined to `src/providers/` (enforced by a unit test); the frontend
  makes no `fetch`/XHR/WebSocket calls (IPC-only).
- No updater/telemetry/analytics dependency.
- API keys live only in the macOS Keychain; tests confirm they never enter error
  messages, and the diagnostics export carries only allowlisted non-secret fields.
- All of the above are enforced by tests (`providers::privacy_tests`,
  `secrets`, `diagnostics`), so regressions fail CI.

## 5. Installable artifact (FR-073/074/075)

`pnpm tauri build` compiles the release (~2m) and produces a valid, persisted
`TLiquid.app` at `src-tauri/target/release/bundle/macos/` — verified on disk:
`com.tliquid.app`, v0.1.0, arm64 Mach-O (~4.6 MB), intact `Info.plist`. (P0-019
also confirmed a headless `hdiutil` `.dmg` at 2.7 MB — under the §13.1 30 MB
target.) Install/run instructions, the Gatekeeper bypass for the unsigned build,
and the signing/notarization deferral (FR-075 → P1-008) are documented in the
README. The styled `.dmg` step needs a Finder/GUI session (fails headless); a
headless `.dmg` is buildable via `hdiutil` (documented).

## 6. Definition-of-Done scenario (`tliquid_todo.md` §8) — coverage map

| # | DoD step | Implemented by | Verified |
|---|---|---|---|
| 1–3 | Install on macOS; menu-bar icon; no Dock while idle | P0-002 (tray, Accessory policy) | build + boot ✓; tray/Dock = manual |
| 4 | Open settings | P0-002/003 (tray menu, gear) | boot ✓; click = manual |
| 5–7 | Configure primary / secondary / additional languages | P0-006 | auto ✓ (logic); UI = manual |
| 8–9 | Enter provider key; stored in Keychain | P0-005/009 | auto ✓ (Keychain tests); entry = manual |
| 10 | Select default model | P0-009 | UI logic ✓; live list = manual w/ key |
| 11 | Open translation panel | P0-002/011 | boot ✓ |
| 12 | Translate typed text with a real LLM | P0-008/010/011 | **manual w/ real key** |
| 13–16 | Select text in another app → primary hotkey → capture → routed translate | P0-007/013/014 | logic/wiring ✓; **manual w/ key + Accessibility** |
| 17–18 | Result in panel; Enter copies | P0-011/012 | logic ✓; **manual** |
| 19 | Secondary hotkey repeat | P0-007/013/015 | logic ✓; **manual** |
| 20 | Useful errors on provider/network/permission/capture failure | P0-016/018 (+ adapters) | logic ✓; **manual** |
| 21–24 | No telemetry / no update calls / no text to TLiquid / no keys in logs-config | P0-017 | auto ✓ (tests + audit) |
| 25 | Basic documentation | P0-020 | ✓ (README) |
| 26 | macOS installable artifact | P0-019 | ✓ (`.app` built + verified) |

## 7. Manual QA checklist (requires a Mac + a real provider API key)

These need an interactive macOS session with a valid key and can't be exercised
in an automated/headless environment:

- [ ] Install the `.app` (Gatekeeper right-click→Open) and confirm the menu-bar
      icon appears with no Dock icon.
- [ ] Tray left-click toggles the panel under the icon; right-click → Open /
      Settings… / Quit.
- [ ] Settings: set primary/secondary/additional languages; add a provider key,
      **Test** shows *Connection OK*; pick a default model; values persist after
      relaunch; "Reveal in Finder" opens the config file.
- [ ] Manual translation with a real provider returns a translation; **Copy** and
      **Enter** (copy + dismiss) both work.
- [ ] Select text in Safari/Chrome, a text editor, and Terminal; press ⌘⇧T —
      capture works (grant Accessibility when prompted), panel opens prefilled and
      translated; ⌘⇧⌥T translates to secondary; with no secondary, it opens
      Settings.
- [ ] Failure paths show compact, actionable errors (bad key, no network, no
      selection / Accessibility off → the "Open Accessibility Settings" button).
- [ ] Confirm via Activity Monitor / a proxy that no traffic goes anywhere except
      the configured provider's host.

## 8. Known issues / notes

- **Bundle identifier warning:** `com.tliquid.app` ends in `.app`, which Tauri
  warns can be confused with the bundle extension. It's cosmetic (the build and
  Keychain service work); changing it would rename the Keychain service, so it's
  left for a deliberate future decision.
- **`.dmg` bundling** requires Finder/GUI (headless fallback documented).
- Phase 0 is **macOS only**; Windows/Linux are not verified (PRD §3.1).

## Verdict

Phase 0 is a **release candidate**: all automated gates are green, the app boots
and the IPC works, the privacy invariants are enforced by tests, and a valid
installable artifact is produced. Sign-off is pending the §7 manual checklist on a
Mac with a real API key, and signing/notarization (P1-008).
