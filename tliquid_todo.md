# TLiquid TODO Tracker

**Document type:** Agent-oriented TODO / Epic tracker  
**Version:** 0.1  
**Date:** 2026-05-26  
**Source document:** TLiquid PRD v0.3  
**Phase 0 scope:** macOS only  
**Execution model:** AI agents take one epic at a time and update task status fields directly.

---

## 1. Status workflow

Each task must always have a status.

Allowed statuses:

```text
not-started
in-progress
blocked
in-review
done
cancelled
```

When an agent starts a task, it must update:

```text
Status: in-progress
Agent ID: <agent-id>
Datetime started: <ISO-8601 datetime>
```

When an agent finishes a task, it must update:

```text
Status: done
Datetime finished: <ISO-8601 datetime>
```

If the task cannot be completed, it must update:

```text
Status: blocked
Notes: reason, missing dependency, failed assumption, or required human decision
```

---

## 2. Task table schema

Every task table uses this schema:

| Field | Meaning |
|---|---|
| Task ID | Stable identifier for the epic/task |
| Name | Human-readable task name |
| Phase | Product phase |
| PRD FRs | Functional requirements covered from PRD v0.3 |
| Status | Required status field |
| Agent ID | ID/name of the agent currently or previously responsible |
| Datetime started | ISO-8601 datetime when work started |
| Datetime finished | ISO-8601 datetime when work finished |
| Acceptance criteria | Concrete criteria for completion |
| Notes | Implementation notes, blockers, links, PRs, or decisions |

---

## 3. Phase 0 — macOS MVP

Goal: after all Phase 0 tasks are complete, TLiquid must be a working installable macOS app with menu-bar behavior, settings, provider API key configuration, model selection, language configuration, manual translation, selected-text translation via hotkey, real LLM calls, secure key storage, no telemetry, and basic documentation.

### Phase 0 task list

| Task ID | Name | Phase | PRD FRs | Status | Agent ID | Datetime started | Datetime finished | Acceptance criteria | Notes |
|---|---|---|---|---|---|---|---|---|---|
| P0-001 | Create repository foundation and build system | Phase 0 | FR-001, FR-002, FR-003, FR-073, FR-074 | done | claude-opus-4.7 | 2026-05-26T22:48:47Z | 2026-05-26T23:01:00Z | Repo contains Rust + Tauri + Svelte app skeleton; app builds locally on macOS; README includes dev setup and build commands; project license is present; basic CI/lint/test commands documented even if CI is not yet configured. | Scaffolded Tauri v2 + Rust + Svelte 5 (Vite, single-window menu-bar panel — see architecture-decision note below). All Phase 0 deps wired: plugins (global-shortcut, clipboard-manager, opener, log, single-instance) + keyring(apple-native)/reqwest/async-trait. Module skeleton (config, secrets, languages, translation, providers/* adapters, tray, windows) + 10 IPC commands. Verified: `cargo build` ✓, `vite build` ✓, `clippy -D warnings` clean, `cargo fmt --check` clean, 6 unit tests pass, `svelte-check` 0 errors. README + MIT LICENSE + starter CI workflow added. Not committed (left for owner). ARCHITECTURE DECISION (post-scaffold): TLiquid is a single window — a frameless menu-bar panel anchored under the tray icon (Raycast / Docker Desktop tray / JetBrains Toolbox style), created hidden at startup and shown/hidden on tray click. Settings and result are *views inside the panel*, not separate windows. The earlier `settings.html`/`result.html` entries were removed; `index.html` → `App.svelte` switches views via a `$state`. Window flags: frameless + always-on-top + visible-on-all-workspaces + Accessory policy so it floats over fullscreen apps. See PRD §19.2 and `src-tauri/src/windows.rs`. |
| P0-002 | Implement macOS menu-bar app shell | Phase 0 | FR-004, FR-005, FR-006, FR-007, FR-008 | done | claude-opus-4.7 | 2026-05-27T01:14:35Z | 2026-05-27T01:16:55Z | App runs as single instance; appears in macOS menu bar; left-clicking the tray toggles the tray-anchored panel (translate + settings views); can quit from the tray's right-click menu; avoids Dock presence while idle where feasible; app remains alive in background. | Single-instance plugin, tray (left-click toggles / right-click Open·Settings·Quit menu), Accessory activation policy (no Dock), and the hidden-at-startup panel were scaffolded in P0-001. P0-002 completed the shell: (1) a `CloseRequested` guard on the panel that calls `prevent_close()` + `hide()` so a close gesture (Cmd+W) dismisses the panel instead of tearing down the only window and letting the app exit — keeps it alive in the background and reuses the warm webview (FR-005, PRD §13.2); (2) extracted the tray-anchored positioning math from `position_under` into a pure `panel_origin()` fn (added bottom-edge clamping for short monitors) with 6 unit tests covering centering, drop-below, left/right/bottom clamping, and non-zero monitor origins. Verified: clippy -D warnings clean, fmt clean, all tests pass. Tray/Dock/single-instance runtime behavior needs manual on-device verification (covered by P0-021 QA). Do not implement Windows/Linux tray behavior in Phase 0. Auto-hide-on-blur is a follow-up (PRD §19.2 Future). |
| P0-003 | Implement Svelte UI foundation and visual system | Phase 0 | FR-003, FR-046 | done | claude-opus-4.7 | 2026-05-27T01:21:05Z | 2026-05-27T01:23:44Z | Svelte UI shell exists as a single panel with translate and settings views (switched in-app via `$state`, not separate windows); frameless panel has a custom draggable titlebar with a gear/back control; follows light/dark system theme where feasible; uses minimal styling; primary action uses TLiquid accent color; UI is responsive and small. | Rebuilt `styles.css` as a documented design system: tokens (accent/surface/text colors, 4px spacing scale, radii, type scale), light/dark via `prefers-color-scheme`, an accessibility baseline (`:focus-visible` ring + `prefers-reduced-motion` guard), and reusable component primitives (`.field/.label/.input/.select/.textarea`, `.btn`/`.btn--primary` accent action, `.output`, `.section`, `.hint`, `.row`/`.grow`). Refined the titlebar (gear/back with aria-labels). Built the inert visual skeleton of the translate view per PRD §10.5 (labelled source textarea, target selector, primary Translate button) and `Result.svelte` (scrollable output + Copy + "Press Enter to copy"); behavior is wired later in P0-011/P0-012. Laid out `Settings.svelte` as the 8 sections from PRD §10.6 (Languages/Shortcuts/Providers/Models/Output/Privacy/Updates/About), filled by P0-006/007/009/017/018. Verified: `pnpm check` 0 errors, `pnpm build` OK. Visual on-device check folded into P0-021 QA. Kept UI utility-like; one window only. |
| P0-004 | Implement local configuration manager | Phase 0 | FR-046, FR-047, FR-048, FR-049 | done | claude-opus-4.7 | 2026-05-27T01:27:32Z | 2026-05-27T01:30:25Z | Non-secret settings persist across restarts; config file is created in a macOS-appropriate app config directory; settings UI shows config file location; corrupted config is handled safely with defaults/backups. | Scaffold already had `Settings`, `load`/`save`, app-config-dir path, and corrupt→`.bak` fallback. P0-004 made it real + testable: extracted pure `load_from_path`/`save_to_path` from the AppHandle wrappers and added 4 unit tests (missing→defaults w/o creating the file, save→load round-trip incl. parent-dir creation, corrupt→defaults + sibling `.json.bak` preserving the original, camelCase serialization guard); derived `PartialEq` on settings structs for the round-trip assertion; added `ensure_initialized` (writes defaults on first run only) called best-effort at startup so the file always exists for editing; added `settings_path` command (+ TS wrapper, lib.rs registration) exposing the absolute path; Settings view now shows the path in a monospace box with a "Reveal in Finder" button (`opener:allow-reveal-item-in-dir` added to capabilities). Secrets remain Keychain-only. Verified: clippy -D warnings, fmt, 16 tests, `pnpm check`, `pnpm build` all clean. |
| P0-005 | Implement macOS Keychain secret storage | Phase 0 | FR-050, FR-051, FR-052 | done | claude-opus-4.7 | 2026-05-27T01:33:00Z | 2026-05-27T01:36:29Z | Provider API keys are stored in macOS Keychain or equivalent secure storage; keys are not logged; keys are not written to plaintext config; deleting/updating keys works; error states are visible in settings. | Scaffold already used the `keyring` crate (`apple-native` → macOS Keychain, service `com.tliquid.app`, account = provider id) with set/get/delete + the `set/has/delete_provider_key` commands. P0-005 hardened + tested it: extracted the result-mapping into pure `map_get` (missing entry → `Ok(None)`, not an error) and `map_delete` (idempotent — deleting an absent key is a no-op success) helpers, and added 6 unit tests incl. a privacy guard asserting the mapped error text is the Keychain backend's own message, never the key value (FR-051/FR-052). Confirmed by inspection: nothing in `secrets.rs`/`commands.rs` logs, and `config::Settings` has no key fields so keys never hit plaintext config. Keychain mock can't test set→get round-trip (fresh credential per `Entry::new`); real persistence + the in-settings error display (provider UI) are exercised by P0-009 + P0-021 QA. Plaintext fallback intentionally absent. clippy/fmt/tests all clean. |
| P0-006 | Implement language settings and primary/secondary model | Phase 0 | FR-021, FR-022, FR-023, FR-024, FR-025, FR-026, FR-027, FR-068, FR-069 | done | claude-opus-4.7 | 2026-05-27T01:39:54Z | 2026-05-27T01:42:36Z | User has mandatory primary language with English default; user can set optional secondary language; user can add/remove/reorder unlimited additional languages; primary/secondary can be changed; settings persist. | Backend config model (`Languages`: primary/secondary/additional, English default) already existed. P0-006 built the Languages settings UI: new `src/lib/languages.ts` (44-language provider-neutral list + `languageByCode`); new `LanguageSettings.svelte` — primary `<select>` (required), secondary `<select>` (incl. "None"), and an add/remove/reorder (↑/↓) list for unlimited additional languages, with consistency rules (a language occupies only one slot; primary≠secondary; hand-edited out-of-list codes still shown, FR-047). `Settings.svelte` now loads the full `Settings` and owns a `persist()` (saveSettings) that section components call on change — the shared-settings pattern P0-007/P0-009 will reuse; save/load errors surface inline. No language cap. Frontend-only; `pnpm check` 0 errors, `pnpm build` OK. Interactive persistence verified at P0-021 QA. |
| P0-007 | Implement shortcut registration for macOS | Phase 0 | FR-028, FR-029, FR-030, FR-031, FR-033, FR-034 | not-started |  |  |  | App registers default macOS global shortcuts: primary translation, secondary translation, open-panel; settings display shortcuts; registration failure shows actionable error; shortcuts can be disabled if feasible. | Custom remapping is Phase 1, not Phase 0. |
| P0-008 | Implement provider abstraction layer | Phase 0 | FR-035, FR-036, FR-037, FR-038, FR-040, FR-041, FR-042, FR-043, FR-044 | done | claude-opus-4.7 | 2026-05-27T01:51:49Z | 2026-05-27T01:56:09Z | Common provider interface exists; OpenAI, Anthropic, Gemini, and OpenRouter adapters compile; provider keys can be read from secure storage; provider errors are normalized; no provider-specific logic leaks into UI beyond model/provider options. | Replaced the four cloud-adapter stubs with real `reqwest` BYOK calls (endpoints verified against current docs): OpenAI `/v1/chat/completions` + `/v1/models` (Bearer); Anthropic `/v1/messages` + `/v1/models` (`x-api-key` + `anthropic-version: 2023-06-01`, top-level `system`, `max_tokens` 4096); Gemini `:generateContent` + `/v1beta/models` via **`x-goog-api-key` header** (so the key never enters the URL) with `systemInstruction`+`contents`, filtering to `generateContent` models; OpenRouter OpenAI-compatible chat + `/api/v1/key` for validation (models list is public). New shared `providers/http.rs`: one pooled client (60s timeout, system TLS), `send_json`/`validate_status`, and a pure `http_error_message` (status→category; 401/403 never echo the body; transport errors use `without_url()`) — normalizing errors without leaking keys (FR-051). Refactored the trait: added `Prompt {system,user}`; `translate(api_key, model, &Prompt) -> String` returns only completion text; `translation::build_prompt` now yields `Prompt` (text in the user message); `commands::translate` orchestrates resolve→prompt→adapter→`TranslationResponse` (latency-timed, trimmed, no persistence). 20 new unit tests (per-adapter response/model parsing + error normalization); 43 total. clippy/fmt clean. Updated CLAUDE.md provider section. validate_key: Ok(true)/Ok(false on 401-403)/Err. Live HTTP exercised at P0-009/P0-021 QA (needs real keys). |
| P0-009 | Implement provider settings, key validation, and model selection UI | Phase 0 | FR-035, FR-036, FR-037, FR-038, FR-040, FR-041, FR-042 | done | claude-opus-4.7 | 2026-05-27T02:02:31Z | 2026-05-27T02:04:42Z | Settings UI allows entering/updating/removing API keys for OpenAI, Anthropic, Gemini, OpenRouter; user can test provider connection; active providers expose selectable models; models for missing keys are disabled; default provider/model persists. | New `ProviderSettings.svelte` renders the Providers + Models sections. Providers: the 4 cloud providers (from `listProviders()` filtered to `available`, so Ollama is excluded); per provider a password key input (`autocomplete=off`), Save (→ `setProviderKey`, clears the input so the key isn't retained in JS, marks `enabled`), Remove (→ `deleteProviderKey`), Test, and a text status (Not configured / Configured / Testing… / Connection OK / Invalid key / Connection failed: msg — labels not color-only). Test validates the just-typed key (`testProviderKey`) or, if the field is empty, the already-saved key via the new `test_provider_connection` command (reads from Keychain so the frontend never holds a saved key). Models: default-provider `<select>` with keyless providers disabled (FR-041/042); default-model `<select>` populated from `listProviderModels` once a key exists, with a manual model-id input + Retry fallback when the list API fails (PRD §10.6.4); `defaultProvider`/`defaultModel` persist via the shared `persist()`. Backend: `test_provider_connection` (+ lib.rs registration + TS wrapper). Verified: clippy/fmt/43 tests, `pnpm check` 0 errors, `pnpm build` OK. Live key validation exercised at P0-021 QA. |
| P0-010 | Implement translation orchestrator and prompt routing | Phase 0 | FR-014, FR-015, FR-018, FR-019, FR-020, FR-044 | done | claude-opus-4.7 | 2026-05-27T02:09:21Z | 2026-05-27T02:10:58Z | Translation orchestrator accepts source text, routing mode, language settings, provider/model; builds primary-mode and explicit-target prompts; returns translation only; preserves formatting/code instructions; no translation text is stored locally by default; no TLiquid server calls exist. | Formalized the orchestrator after P0-008 wired the flow inline. Extracted `translation::plan(settings, mode, explicit, source_text) -> TranslationPlan{target_language, prompt}` — the pure core (routing resolution via `languages::resolve` + prompt selection + missing-secondary error), leaving `commands::translate` as thin I/O glue (Keychain lookup → adapter call → latency-timed `TranslationResponse`, dropped its now-unused `languages` import). Primary-mode prompt encodes the §9.2 auto-routing rules; explicit/secondary use the fixed-target prompt; all instruct "return only the translation" + preserve formatting/code. Added 5 orchestrator tests (explicit/primary-no-secondary/primary-with-secondary/secondary-missing→err/secondary-targets) → 48 total. No translation text persisted; only direct BYOK provider HTTP (no TLiquid server). Non-streaming (one response). clippy/fmt/tests clean. |
| P0-011 | Implement manual translation in the panel | Phase 0 | FR-011, FR-016, FR-017, FR-018 | done | claude-opus-4.7 | 2026-05-27T02:15:52Z | 2026-05-27T02:17:14Z | The panel's translate view contains input field, target selector, Translate button, output field, Copy button, and Enter-to-copy behavior after translation; real configured LLM provider is called; provider/network errors are shown compactly. | New `Translate.svelte` owns the manual flow: source textarea, a target `<select>` (Auto = primary routing; or each configured language as an explicit target), and a Translate button that calls the real `translate` command with the configured `defaultProvider`/`defaultModel`. Enter translates / Shift+Enter newlines; on success focus leaves the input so a window-level Enter handler copies the result (FR-017), and the Copy button copies via the clipboard plugin (FR-016). `Result.svelte` rewritten as a presentational pane (output / busy / compact error / Copy + "Press Enter to copy"/"Copied!"). Shows a hint to configure a provider when no default model is set. Remounts on each return to the view so it re-reads settings. App.svelte renders `<Translate>` instead of the inert skeleton. Panel-dismiss-on-Enter + scrollable result are P0-012. `pnpm check` 0 errors, `pnpm build` OK; live translation exercised at P0-021 QA. |
| P0-012 | Implement result view and clipboard copy behavior | Phase 0 | FR-016, FR-017, FR-018 | not-started |  |  |  | Result view in the panel displays translated text; Copy button copies result; pressing Enter copies result and dismisses the panel; large outputs are scrollable; errors are visible and do not crash app. | Result is shown inline in the same panel as manual translation (one window). |
| P0-013 | Implement macOS selected-text capture | Phase 0 | FR-012, FR-013, FR-018 | not-started |  |  |  | App can capture currently selected text from common macOS apps using selected implementation path; app handles no-selection and capture-failure cases; clipboard is restored if simulated copy is used; permission failures show guidance. | Test at minimum with browser, text editor, terminal/IDE if possible. |
| P0-014 | Implement primary selected-text translation flow | Phase 0 | FR-012, FR-014, FR-015, FR-016, FR-017, FR-018 | not-started |  |  |  | Selecting text and pressing primary shortcut translates using primary/secondary rules; the panel opens prefilled with the source text and translation; Enter copies result; failures are actionable. | Primary language default is English. |
| P0-015 | Implement secondary selected-text translation flow | Phase 0 | FR-013, FR-014, FR-015, FR-016, FR-017, FR-018 | not-started |  |  |  | If secondary language is configured, selecting text and pressing secondary shortcut translates to secondary and the panel opens prefilled; if secondary is missing, the panel opens to the Settings view (languages section). | Secondary language is optional. |
| P0-016 | Implement macOS permissions onboarding and diagnostics messages | Phase 0 | FR-018, FR-064, FR-065 | not-started |  |  |  | App explains required macOS permissions only when needed; capture failures show likely cause and fix; optional local diagnostics export is available if implemented; no diagnostics are uploaded. | Keep messages short and actionable. |
| P0-017 | Implement privacy and no-telemetry safeguards | Phase 0 | FR-019, FR-020, FR-051, FR-052, FR-056, FR-064, FR-067 | not-started |  |  |  | No TLiquid network calls except direct provider calls initiated for translation/key validation/model lookup; no telemetry upload; no automatic update checks; logs exclude text, clipboard, prompts, provider responses, and API keys. | Add tests or code review checklist for this. |
| P0-018 | Implement updates/about placeholders | Phase 0 | FR-056, FR-057 | not-started |  |  |  | Settings/About shows app version and release link; explicitly indicates that automatic updates are not enabled in Phase 0; no background update network request occurs. | Update system itself is Phase 2. |
| P0-019 | Build macOS installable artifact | Phase 0 | FR-073, FR-074, FR-075 | not-started |  |  |  | Build produces macOS installable artifact suitable for manual local installation; install/run instructions exist; signing/notarization is attempted or documented as deferred if not feasible. | Internal MVP can use unsigned build if documented. |
| P0-020 | Write Phase 0 documentation | Phase 0 | FR-002, FR-020, FR-050, FR-064, FR-073, FR-074 | not-started |  |  |  | README documents product purpose, macOS-only Phase 0 scope, installation, provider setup, API key storage, privacy behavior, no telemetry, permissions, known limitations, and troubleshooting. | Must clearly state Windows/Linux are not verified in Phase 0. |
| P0-021 | Phase 0 end-to-end QA and release candidate | Phase 0 | All Phase 0 FRs | not-started |  |  |  | Fresh install works on macOS; menu-bar icon appears; settings work; provider key can be added; model selected; manual translation works with real LLM; selected-text primary/secondary hotkeys work; copy behavior works; no telemetry/update calls occur; install artifact is available. | This task should be taken only after P0-001 through P0-020 are done or in-review. |

---

## 4. Phase 1 — macOS polish, local models, right-click integration

Goal: improve the macOS product after MVP, add startup behavior, configurable shortcuts, local model support, output modes, and macOS right-click integration.

### Phase 1 task list

| Task ID | Name | Phase | PRD FRs | Status | Agent ID | Datetime started | Datetime finished | Acceptance criteria | Notes |
|---|---|---|---|---|---|---|---|---|---|
| P1-001 | Implement startup-on-login for macOS | Phase 1 | FR-053, FR-054, FR-055 | not-started |  |  |  | User can enable/disable launch at login; default onboarding offers ON behavior with clear consent; setting persists; app launches into menu-bar mode. | Use Tauri/plugin or macOS-native mechanism. |
| P1-002 | Implement configurable shortcuts | Phase 1 | FR-032, FR-033, FR-034 | not-started |  |  |  | User can change primary, secondary, open-panel, and optional additional-language shortcuts; conflicts are detected; invalid shortcuts are rejected; settings persist. | Include reset-to-default action. |
| P1-003 | Implement configurable output behavior | Phase 1 | FR-016, FR-017, FR-018 | not-started |  |  |  | User can choose show panel and/or copy to clipboard automatically; replace-selection option is either safely implemented or explicitly hidden/experimental; defaults remain safe. | Replacing selected text must not be default. |
| P1-004 | Add Ollama/local model support | Phase 1 | FR-039, FR-043 | not-started |  |  |  | User can configure local Ollama endpoint; available models can be listed or manually configured; translation works without cloud provider key; errors are clear when Ollama is unavailable. | Keep provider abstraction unchanged. |
| P1-005 | Implement macOS right-click integration | Phase 1 | Right-click plan, FR-012, FR-013 | not-started |  |  |  | macOS selected text can be sent to TLiquid through Services or equivalent right-click/native mechanism; user can translate through contextual action where macOS supports it; hotkey remains primary fallback. | Do not block core flow on this. |
| P1-006 | Improve macOS selected-text capture reliability | Phase 1 | FR-012, FR-013, FR-018 | not-started |  |  |  | Capture works across a wider set of macOS apps; clipboard restoration is more robust; permission detection is improved; failure diagnostics are clearer. | Add app compatibility notes. |
| P1-007 | Improve local diagnostics export | Phase 1 | FR-065, FR-067 | not-started |  |  |  | User can export local diagnostics bundle that excludes API keys, translations, clipboard contents, prompts, and provider responses; bundle includes version, OS, settings metadata, logs, and recent error categories. | User manually sends/export diagnostics; no automatic upload. |
| P1-008 | Improve macOS packaging, signing, and notarization | Phase 1 | FR-073, FR-074, FR-075 | not-started |  |  |  | Official macOS build process is documented; signing/notarization implemented if credentials exist; installer UX improved. | Can remain partially blocked by Apple developer account. |
| P1-009 | Add streaming translation output | Phase 1 | FR-014, FR-015, FR-018 | not-started |  |  |  | Provider adapters stream partial output (SSE for OpenAI/Anthropic/Gemini/OpenRouter, NDJSON for Ollama); deltas are surfaced incrementally in the panel via a Tauri channel; `supports_streaming()` is true for capable providers; the non-streaming path remains a fallback; Enter-to-copy still works on the completed text. | Deferred from Phase 0 (PRD §24 #1). Builds on P0-008/P0-010. Use the `reqwest` `stream` feature + an SSE parser; still no provider SDKs. |
| P1-010 | Phase 1 QA and release candidate | Phase 1 | All Phase 1 FRs | not-started |  |  |  | Fresh macOS install validates startup, custom shortcuts, Ollama/local models, output behavior, streaming output, right-click integration where available, and improved capture behavior. | Should be done after other Phase 1 tasks. |

---

## 5. Phase 2 — Windows, hosted cloud, updates, paid mode

Goal: add verified Windows support and introduce hosted cloud features, paid mode, updates, optional diagnostics, and Windows right-click integration.

### Phase 2 task list

| Task ID | Name | Phase | PRD FRs | Status | Agent ID | Datetime started | Datetime finished | Acceptance criteria | Notes |
|---|---|---|---|---|---|---|---|---|---|
| P2-001 | Add verified Windows app shell and tray support | Phase 2 | FR-009, FR-076 | not-started |  |  |  | App builds and runs on Windows; tray icon appears; no taskbar item when idle where possible; app can open popup/settings and quit from tray. | Requires Windows test environment. |
| P2-002 | Add Windows global shortcut and selected-text support | Phase 2 | FR-012, FR-013, FR-028, FR-029, FR-030 | not-started |  |  |  | Primary/secondary/manual shortcuts work on Windows; selected-text capture works in common Windows apps; failures are handled clearly. | Use Windows-specific platform adapter. |
| P2-003 | Add Windows packaging | Phase 2 | FR-076 | not-started |  |  |  | Windows installer/package is produced; install/run/uninstall instructions exist; release artifact is available. | Signing may be deferred or documented. |
| P2-004 | Implement hosted LLM proxy backend | Phase 2 | FR-045, FR-070, FR-071, FR-072 | not-started |  |  |  | Backend accepts authenticated translation requests; routes to LLM provider using TLiquid credentials; returns translation; never exposes provider keys; usage is metered. | Requires backend stack decision. |
| P2-005 | Implement account, licensing, and cloud mode in app | Phase 2 | FR-070, FR-071, FR-072 | not-started |  |  |  | User can sign in or enter license; app can choose BYOK or hosted mode; hosted mode uses backend proxy; failures fall back cleanly. | Keep BYOK mode unrestricted. |
| P2-006 | Implement usage metering and billing hooks | Phase 2 | FR-070, FR-071, FR-072 | not-started |  |  |  | Hosted usage records include minimal metadata needed for billing/rate limits; plan limits are enforced; no translation content is stored by default. | Payment provider choice can be separate. |
| P2-007 | Implement auto-update check and update-now flow | Phase 2 | FR-058, FR-059, FR-060, FR-061, FR-062, FR-063 | not-started |  |  |  | App checks for updates on startup and once per day; settings show update state; user can click Update now; update downloads, verifies, installs, and relaunches. | Must not be silent/forced. |
| P2-008 | Implement optional anonymous diagnostics backend and client | Phase 2 | FR-066, FR-067 | not-started |  |  |  | Diagnostics are opt-in and OFF by default; client sends only allowed metadata; backend receives/stores diagnostics; forbidden data is excluded by design and tests. | No translation text, clipboard, keys, prompts, responses. |
| P2-009 | Implement Windows right-click integration | Phase 2 | Right-click plan | not-started |  |  |  | Windows context integration works where technically feasible; selected text can be sent to TLiquid; hotkey remains primary fallback. | May require shell extension. |
| P2-010 | Add optional translation history MVP | Phase 2 | Phase 2 goals | not-started |  |  |  | If enabled, user can view recent translations; history is OFF by default or explicitly consented; storage mode is local-first unless cloud sync is explicitly enabled. | May be deferred to Phase 3 if scope grows. |
| P2-011 | Phase 2 QA and release candidate | Phase 2 | All Phase 2 FRs | not-started |  |  |  | macOS remains working; Windows verified; hosted proxy works; updates work; diagnostics opt-in works; BYOK unrestricted mode remains intact. | Requires macOS and Windows testing. |

---

## 6. Phase 3+ — Linux and advanced features

Goal: add verified Linux support and larger product features such as translation memory, cloud sync, and advanced translation workflows.

### Phase 3+ task list

| Task ID | Name | Phase | PRD FRs | Status | Agent ID | Datetime started | Datetime finished | Acceptance criteria | Notes |
|---|---|---|---|---|---|---|---|---|---|
| P3-001 | Add verified Linux app shell and tray compatibility matrix | Phase 3+ | Phase 3+ goals | not-started |  |  |  | App builds and runs on target Linux environments; tray/status icon behavior documented; compatibility matrix covers at least GNOME/KDE and Wayland/X11 considerations. | Requires Linux test machines/VMs. |
| P3-002 | Add Linux global shortcut and selected-text support | Phase 3+ | Phase 3+ goals | not-started |  |  |  | Global shortcuts and selected-text capture work where technically feasible; limitations are documented per desktop environment/compositor. | Wayland may restrict behavior. |
| P3-003 | Add Linux packaging | Phase 3+ | Phase 3+ goals | not-started |  |  |  | At least one Linux package format is produced; install instructions exist; AppImage/deb/rpm/Flatpak decision documented. | Choose format based on target users. |
| P3-004 | Add Linux right-click best-effort integration | Phase 3+ | Right-click plan | not-started |  |  |  | Context integration is implemented for selected Linux desktop environments where feasible; unsupported environments are documented. | Likely not universal. |
| P3-005 | Implement translation memory | Phase 3+ | Phase 3+ goals | not-started |  |  |  | User can enable local translation memory; repeated phrases can be reused/suggested; feature is privacy-preserving and controllable. | Requires product design. |
| P3-006 | Implement cloud profile/settings sync | Phase 3+ | Phase 3+ goals | not-started |  |  |  | User can sync non-secret settings across devices; secrets are handled securely and not blindly synced unless explicitly designed. | Depends on account backend. |
| P3-007 | Implement advanced translation modes | Phase 3+ | Phase 3+ goals | not-started |  |  |  | User can choose literal/natural/professional/casual/preserve-markdown modes; prompts are tested and provider-neutral. | Avoid turning app into generic chat. |
| P3-008 | Implement team/organization features | Phase 3+ | Phase 3+ goals | not-started |  |  |  | Organization can manage shared provider settings, billing, and policy controls; individual privacy constraints remain explicit. | Enterprise scope; defer unless validated. |
| P3-009 | Phase 3+ QA and release candidate | Phase 3+ | All Phase 3+ goals | not-started |  |  |  | Linux support is verified; advanced features work without regressing macOS/Windows; privacy promises remain intact. | Requires multi-platform QA. |

---

## 7. Phase 0 dependency order

Recommended order for agents:

```text
P0-001 Repository foundation
  ↓
P0-002 macOS menu-bar shell
  ↓
P0-003 Svelte UI foundation
  ↓
P0-004 Config manager
  ↓
P0-005 Keychain secret storage
  ↓
P0-006 Language settings
  ↓
P0-008 Provider abstraction
  ↓
P0-009 Provider/model settings
  ↓
P0-010 Translation orchestrator
  ↓
P0-011 Manual translation in the panel
  ↓
P0-007 Shortcuts
  ↓
P0-013 Selected-text capture
  ↓
P0-012 Result view
  ↓
P0-014 Primary selected-text flow
  ↓
P0-015 Secondary selected-text flow
  ↓
P0-016 Permissions/errors
  ↓
P0-017 Privacy/no telemetry safeguards
  ↓
P0-018 Updates/about placeholders
  ↓
P0-019 macOS installable artifact
  ↓
P0-020 Documentation
  ↓
P0-021 End-to-end QA/release candidate
```

Some tasks can be parallelized after the foundation exists:

```text
P0-006 Language settings
P0-008 Provider abstraction
P0-003 UI foundation
```

But integration tasks should wait for their dependencies.

---

## 8. Phase 0 definition of done

Phase 0 is done only when a fresh macOS install can perform this full scenario:

1. User installs TLiquid on macOS.
2. TLiquid appears in the macOS menu bar.
3. TLiquid does not show unnecessary Dock presence while idle where feasible.
4. User opens settings.
5. User configures primary language.
6. User optionally configures secondary language.
7. User adds any number of additional languages.
8. User enters at least one provider API key.
9. API key is stored securely in macOS Keychain.
10. User selects default model.
11. User opens the translation panel.
12. User translates typed/pasted text with a real LLM provider.
13. User selects text in another macOS app.
14. User presses primary translation hotkey.
15. TLiquid captures selected text.
16. TLiquid translates using primary/secondary rules.
17. Result appears in the panel.
18. User presses Enter and result is copied.
19. User repeats with secondary hotkey if secondary language is configured.
20. App shows useful errors if provider/network/permission/selection capture fails.
21. No telemetry calls are made.
22. No automatic update calls are made.
23. No translation text is sent to TLiquid servers.
24. API keys are not logged or written to plaintext config.
25. Basic documentation exists.
26. A macOS installable artifact exists.

---

## 9. Agent instructions

When an agent takes a task:

1. Update the task row before coding:
   - `Status = in-progress`
   - `Agent ID = <agent-id>`
   - `Datetime started = <ISO-8601 datetime>`
2. Work only within the task scope unless a dependency requires a small supporting change.
3. Keep PRs large enough to finish the epic, but not so broad that unrelated epics are mixed.
4. Update documentation if the task changes user-facing behavior.
5. Add tests where practical, especially for routing, config, provider abstraction, and privacy safeguards.
6. Do not add telemetry, update checks, hosted backend calls, Windows behavior, or Linux behavior in Phase 0 unless explicitly assigned.
7. Do not store API keys in plaintext.
8. Do not log translation text, prompts, provider responses, clipboard contents, or API keys.
9. When finished, update:
   - `Status = done`
   - `Datetime finished = <ISO-8601 datetime>`
   - `Notes = PR link / commit / summary`
10. If blocked, update:
   - `Status = blocked`
   - `Notes = exact blocker and proposed resolution`

---

## 10. Phase 0 explicit exclusions

Agents must not spend Phase 0 implementation time on:

1. Windows tray behavior.
2. Windows selected-text capture.
3. Windows packaging.
4. Linux tray behavior.
5. Linux selected-text capture.
6. Linux packaging.
7. Right-click context menu integration.
8. Hosted TLiquid backend.
9. Paid accounts or licensing.
10. Auto-update checks.
11. Anonymous telemetry upload.
12. Translation history.
13. Translation memory.
14. Cloud sync.
15. Team/organization features.
16. Heavy native Liquid Glass work.

These are future-phase items unless the PRD is updated.

