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

---

# TLiquid — Phase 1 QA / Release Candidate report (P1-010)

Date: 2026-05-27 · Branch: `main` · Version: 0.1.0 (macOS, Apple Silicon)

Phase 1 end-to-end QA pass. Scope (per `tliquid_todo.md`): **P1-001** launch-at-login,
**P1-002** configurable shortcuts, **P1-004** Ollama/local models, **P1-007** diagnostics
export, **P1-008** packaging/signing, **P1-009** streaming. (P1-003 output behavior,
P1-005 right-click, and P1-006 capture reliability are **dismissed** — out of scope.)

## 1. Automated gate (matches CI, `.github/workflows/ci.yml`) — all green

| Step | Result |
|---|---|
| `pnpm check` (svelte-check) | ✅ 0 errors, 0 warnings (123 files) |
| `cargo fmt --check` | ✅ clean |
| `cargo clippy --all-targets -- -D warnings` | ✅ clean |
| `cargo test` | ✅ 78 passed, 0 failed (was 58 in Phase 0; +20 new) |
| `pnpm tauri build --no-bundle` | ✅ release builds |
| `pnpm tauri build` (full) | ✅ **`.app` + styled `.dmg`** both bundled |

New Phase 1 test coverage: SSE line parser + per-provider stream-delta extractors
(`providers::{http,openai,openrouter,anthropic,gemini}`), Ollama tag/chat/NDJSON
parsing + endpoint trim + 200-error surfacing (`providers::ollama`), Ollama endpoint
config fallback (`config`), shortcut conflict resolution + validity + entry-building
(`shortcuts`), and the diagnostics error-level tally (`diagnostics`). The Phase 0
privacy guards (host allowlist, `reqwest` confinement, no-key-in-error, diagnostics
no-secret-fields) still pass.

## 2. IPC surface — fully wired (23 commands)

All 23 `#[tauri::command]` functions are defined, registered in `invoke_handler!`
(`lib.rs`), and exposed via a typed wrapper in `src/lib/tauri.ts` — cross-checked,
**zero mismatches**. New since Phase 0: `translate_stream` (P1-009), `pause_shortcuts`
+ `validate_shortcut` (P1-002), `set_launch_at_login` + `is_launch_at_login` (P1-001),
`export_diagnostics` (P1-007).

## 3. Build / boot

- `pnpm tauri build` produces **`TLiquid.app`** (`com.tliquid.app`, v0.1.0, arm64
  Mach-O, 4.7 MB) and **`TLiquid_0.1.0_aarch64.dmg`** (2.4 MB — under the §13.1 30 MB
  target). Both verified on disk this pass.
- Boot smoke test: the debug binary launches with **no panic** and no compile/runtime
  error. **The WKWebView could not render in this environment** (no attached display /
  screen-recording session — `screencapture` returns "could not create image from
  display", and the process exits without mounting the webview), so the panel UI could
  not be visually inspected or screenshotted here. The frontend builds clean
  (svelte-check 0, vite 132 modules) and the IPC round-trip was confirmed in Phase 0.
  **Interactive UI verification + screenshots require the owner's interactive Mac
  session** (see §6).

## 4. Per-task QA coverage

| Task | Verified automatically / by build | Requires on-device (real key/OS) |
|---|---|---|
| **P1-009 streaming** | SSE reader + delta extractors unit-tested; `supports_streaming()` true for the 4 cloud providers; non-streaming fallback path intact; release build OK | Live streamed deltas appearing incrementally in the panel; Enter-to-copy on completed text |
| **P1-004 Ollama** | Adapter parse/endpoint/error tests; keyless endpoint via config; provider abstraction unchanged; UI type-checks | Live translate against a running `ollama serve` + a pulled model; "server unreachable" error |
| **P1-002 shortcuts** | Conflict/validity/entry-building tests; recorder lifecycle reviewed (3 rounds); IPC wired | Recording a combo; conflict/invalid rejection live; per-language hotkey translates; re-register across apps |
| **P1-001 launch-at-login** | `tauri-plugin-autostart` wired; reconcile at boot; consent flow server-authoritative (no clobber, reviewed); tests | Toggle registers a LaunchAgent for the installed `.app`; app launches to the menu bar at login |
| **P1-007 diagnostics** | Bundle = metadata + error summary + log tail; no-secret guard; logging-discipline audit; export writes a file | Copy/Save-to-file from the panel; reveal in Finder |
| **P1-008 packaging** | `.app` + `.dmg` build; env-driven signing/notarization config + CI release workflow + `docs/BUILD.md` | **Signed + notarized artifact (needs a paid Apple Developer account)** — see §6 |

## 5. Privacy / no-telemetry (re-confirmed for Phase 1)

- Streaming reuses the same host set (no new hosts; Gemini `:streamGenerateContent?alt=sse`
  is the same host); the SSE/NDJSON readers normalize non-2xx exactly like `send_json`
  (401/403 never echo the body). Host-allowlist + `reqwest`-confinement tests still pass.
- Ollama is local/keyless (endpoint config, default `http://localhost:11434`; `localhost`/
  `127.0.0.1` already allow-listed); no key involved.
- The diagnostics bundle adds a log tail, safe because the logging discipline never writes
  keys/text/prompts/responses (audited: the only log calls are 4 generic `warn!` sites);
  the metadata section is still guarded structurally. No upload anywhere.
- No telemetry/updater/analytics dependencies were added.

## 6. Blockers / items requiring owner action

- **P1-008 signing/notarization is `in-progress`, not done.** The pipeline is fully wired
  (Hardened-Runtime config, entitlements, env-driven signing, tag-triggered CI release,
  `docs/BUILD.md`), but producing a *signed + notarized* artifact requires a **paid Apple
  Developer account** + the certificate/API-key secrets. Until then releases are unsigned
  (acceptable for local/MVP use per FR-075; Gatekeeper bypass documented). This is the one
  Phase 1 task that can't be completed without owner credentials.
- **Interactive UI / live behavior could not be exercised in this environment** (no display
  for the webview, no real provider keys, no Ollama server, no login session). The §7
  checklist needs the owner's Mac.

## 7. Manual QA checklist (Phase 1 — needs a Mac, real keys, Ollama, a login)

- [ ] **Streaming:** translate with a cloud provider; the result streams in word-by-word;
      press Enter after it finishes → copies the complete text.
- [ ] **Ollama:** `ollama serve` + `ollama pull <model>`; Settings → Providers → set the
      endpoint, **Test** = OK; Models lists the pulled model; translate with no cloud key;
      stop Ollama → a clear "unreachable" error.
- [ ] **Shortcuts:** Settings → Shortcuts → record a new combo for primary/secondary and an
      additional language; assigning the same combo twice flags a conflict; an invalid combo
      is rejected; **Reset to defaults** restores ⌘⇧T / ⌘⇧⌥T; the additional-language hotkey
      translates a selection into that language.
- [ ] **Launch-at-login:** first run shows the consent banner (Enable/Not now); enabling
      registers a LaunchAgent; after logout/login the app starts in the menu bar; the
      Settings → Startup toggle and the consent choice agree and persist.
- [ ] **Diagnostics:** Settings → Privacy → **Copy diagnostics** and **Save to file…** both
      produce a bundle with metadata + a log tail and **no** keys/text; Save reveals the file.
- [ ] **Signing (when credentials exist):** `scripts/build-macos.sh` with the `APPLE_*` vars
      produces a signed/notarized `.app` that passes `spctl -a -vvv -t install` and
      `xcrun stapler validate`.

## 8. Verdict

Phase 1 is a **release candidate**: every automated gate is green (78 tests), the IPC
surface is fully aligned (23 commands), and a complete installable `.app` + `.dmg` build.
Five of the six in-scope tasks (P1-001, P1-002, P1-004, P1-007, P1-009) are **done** and
peer-reviewed; **P1-008 stays in-progress** pending an Apple Developer account. Sign-off is
pending the §7 on-device checklist (and signing) — the interactive parts an automated,
display-less environment can't exercise.
