# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

TLiquid is a macOS-first, BYOK (bring-your-own-key) LLM translator that lives in the
menu bar. **Tauri v2** shell, **Rust** core (`src-tauri/`), **Svelte 5 + Vite + TypeScript**
frontend (`src/`). Package manager is **pnpm**. Currently **Phase 0** (macOS-only MVP);
Windows/Linux are not verified targets. The full spec is `llm_translator_prd.md`; work is
tracked as `P0-xxx` tasks in `tliquid_todo.md`, and module/command doc-comments cite the
task IDs they implement.

## Commands

```bash
pnpm install                 # JS deps (use --frozen-lockfile in CI)
pnpm tauri dev               # run the app (hot-reload UI + Rust rebuilds)
pnpm build                   # frontend only → ./dist
pnpm tauri build --no-bundle # full app build, no signing (what CI runs)
pnpm tauri build             # full .app + .dmg → src-tauri/target/release/bundle

pnpm check                   # frontend type-check (svelte-check)

# Rust (from repo root; note the manifest path)
cargo fmt    --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test   --manifest-path src-tauri/Cargo.toml
cargo test   --manifest-path src-tauri/Cargo.toml <name>   # single test by name
```

CI (`.github/workflows/ci.yml`, macOS only) runs, in order: `pnpm check`, `cargo fmt --check`,
`cargo clippy -D warnings`, `cargo test`, `pnpm tauri build --no-bundle`. Match these before pushing.

**Do not open `http://localhost:1420` in a browser** to test the UI — the Tauri `invoke`
runtime only exists inside the app window, so the frontend guards on `isTauri()` and shows
an error otherwise. Always interact through the window `pnpm tauri dev` opens.

## Architecture

### Single-window panel (one window, internal navigation)
TLiquid is **one window** — a frameless menu-bar panel anchored under the tray icon, in the
spirit of Raycast, Docker Desktop's tray panel, and JetBrains Toolbox. There is no separate
settings or result window: those are **views inside the one panel**, switched by a `$state`
variable in `App.svelte` (`view: "translate" | "settings" | "notifications"`), not by routing or
by opening new windows. So there is a single `index.html` → `src/main.ts` → `App.svelte`;
`Settings.svelte` and `Result.svelte` are child components of `App`, not window entry points.
Adding a "screen" means adding a component and a branch to `App.svelte`'s view switch — **not** a
new HTML entry or window. The **translate view is the always-present base layer**; Settings and
Notifications render as a **fixed-width overlay pane** (`.overlay`, right-aligned, ~340px) sliding
over part of it with a click-to-close scrim (P2-012), so the panel doesn't look sparse when resized
large — they are not full-window replacements.

The window itself (`src-tauri/src/windows.rs`, label `"main"`) is:
- **Created once at startup, hidden** (`create_panel`), so summoning it is an instant `show`/`hide`, never a fresh webview load. Dev builds auto-show it; release stays hidden until summoned. Default size 360×270; **resizable but never smaller** (`min_inner_size` = the default), with the inner layout growing proportionally (P2-012). A user resize is remembered across restarts alongside the position (`window.json`; `USER_SIZED`/`USER_POSITIONED`, persisted independently).
- **Frameless + transparent + `always_on_top` + `visible_on_all_workspaces`**, which — combined with the macOS Accessory activation policy set in `lib.rs` — lets it float over other apps including fullscreen Spaces. **Transparent** (needs `app.macOSPrivateApi` + the `macos-private-api` tauri feature) so the optional **translucency** is plain *see-through* window transparency — **not** vibrancy/`NSVisualEffectView` (vibrancy frosts/blurs what's behind; we want to actually see the window underneath). It's driven entirely in CSS: the frontend flips a `body.translucent` class (from `config.ui.translucent`, default on) that swaps the opaque `.panel`/surface backgrounds for low-alpha "glass" + a shared window-wide colour wash (`styles.css` `--tl-paint*`/`--tl-glass*`). `windows::apply_translucency` just *clears* any stray vibrancy; `set_translucency` persists the toggle. With translucency off, the opaque CSS background fills the transparent window so it looks solid. Reduce-Transparency is honored via a CSS `prefers-reduced-transparency` fallback (Safari 17+). The slim titlebar (drag handle + bell/gear/back, no title text) is drawn in the UI (`.titlebar` with `data-tauri-drag-region`); dragging needs the `core:window:allow-start-dragging` capability.
- **Summon + auto-hide (Spotlight-style)**: `show_panel` is the single entry point (there is no toggle) — `tray.rs` left-click, the tray menu's "Open"/"Settings…", and the translate hotkey (via `on_trigger`, for any outcome — captured text, a capture error, or no selection) all call it. Until the user drags the window, each summon anchors it under the tray icon via `position_under_tray` (reads the tray icon's screen `rect()` + the pure, unit-tested `panel_origin` clamping). The panel **auto-hides on blur** (`Focused(false)` → `hide`) and on **Esc** (App.svelte) — re-summon via the tray or the hotkey. A `CloseRequested` also hides rather than destroys (keeps the menu-bar app alive).
- **Draggable + position-remembering** (Raycast-style): the user can drag the panel by the titlebar; once dragged it stops re-anchoring and the position is remembered across restarts (`window.json` beside `settings.json`, validated against connected monitors). Only genuine user drags are saved — moves while the window is hidden (our anchoring/restoring) are ignored (`USER_POSITIONED`). Tray right-click opens the menu; "Settings…" also emits a `navigate` event the frontend listens for to switch views.

### The IPC boundary (keep these in sync)
`src/lib/tauri.ts` is the **only** place the frontend calls `invoke` — Svelte components import
typed wrappers from it, never `@tauri-apps/api` directly. Each wrapper maps 1:1 to a `#[tauri::command]`
in `src-tauri/src/commands.rs`, and those commands are the **only** entry points into the Rust core.
Three things must stay aligned when you touch a command:
1. The `#[tauri::command]` fn in `commands.rs`.
2. Its registration in the `invoke_handler!` macro in `src-tauri/src/lib.rs` (unregistered = not callable).
3. The typed wrapper + interface in `src/lib/tauri.ts`.

Rust structs use `#[serde(rename_all = "camelCase")]`, so a Rust field `source_text` is `sourceText`
on the TS side. Mismatches surface as runtime deserialization errors, not type errors.

### Rust module map (`src-tauri/src/`)
- `lib.rs` — app builder: plugin registration, panel creation, tray setup, macOS accessory mode (no Dock), `invoke_handler`. `main.rs` just calls `run()`.
- `tray.rs` — menu-bar shell; the tray is the app's primary surface. Left-click summons the panel; right-click opens a menu (Open / Settings… / Quit).
- `windows.rs` — the single panel window: create-hidden-at-startup, summon (`show_panel`)/hide, tray-anchored-then-draggable positioning + position persistence, auto-hide on blur (above).
- `shortcuts.rs` — global hotkeys (P0-007, P1-002). Default ⌘⇧T translate-selection and ⌘⇧⌥T translate-to-secondary, **both user-configurable**, plus an optional per-additional-language shortcut (`Action::Explicit(Language)`) that translates the selection into that language. Each captures the selection first (`capture`), then summons the panel: with text it translates, on a capture failure it shows the error, and with **no selection it opens the panel empty** for manual typing (no auto-translate). `apply()` re-registers from settings, deduping same-combo conflicts (pure `resolve_conflicts`) and collecting OS-registration failures for Settings; `is_valid()` (plugin parse) rejects invalid combos; `pause()` unregisters all while the UI records a new combo (so the keypress reaches the webview).
- `capture.rs` — macOS selected-text capture (P0-013): probe-the-clipboard + simulated ⌘C + poll. Returns a 3-way `Capture` (`Text` / `NoSelection` / `Failed`); `Enigo::new` returning `NoPermission` is how "no Accessibility permission" is told apart from "no selection".
- `commands.rs` — the IPC surface (above).
- `config.rs` — non-secret settings, persisted as `settings.json` in the app config dir. Corrupt files are renamed to `.json.bak` and defaults used, never silently discarded. `Settings::default()` is the source of truth for defaults.
- `secrets.rs` — API keys in the **macOS Keychain** via the `keyring` crate (service `com.tliquid.app`, account = provider id). Keys never go in `settings.json` or logs.
- `languages.rs` — routing engine. `resolve()` returns a `Resolution`. Primary-mode routing is **not** resolved to one target up front — source detection happens inside the LLM, so the routing rules are encoded into the prompt (`PrimaryRouted`).
- `translation.rs` — pure, provider-neutral prompt builders. No translation text is persisted anywhere.
- `providers/` — provider abstraction (see below).
- `startup.rs` — launch-at-login (P1-001) via `tauri-plugin-autostart` (macOS LaunchAgent). The user's intent lives in `config.startup.enabled`; `reconcile()` applies it to the OS at startup, and `set_launch_at_login`/`is_launch_at_login` commands drive/read it. The app always boots into Accessory/menu-bar mode, so a login launch is silent.
- `diagnostics.rs` — local diagnostics bundle (P0-016 + P1-007): non-secret settings metadata + a recent-error summary + a tail of the persisted log file (`tliquid.log` in the app log dir, written by the `tauri-plugin-log` `LogDir` target). Copied or saved to a file (`diagnostics`/`export_diagnostics` commands), never uploaded. Safe to include logs because the logging discipline (P0-017 audit) never writes keys/text/prompts/responses.
- `updater.rs` — in-app updates (P2-007 manual). Wraps the Tauri updater plugin against `latest.json` on GitHub Releases (endpoint + minisign **public** key in `tauri.conf.json`; the **private** key is gitignored in `.tauri/` and lives in the `TAURI_SIGNING_PRIVATE_KEY` repo secret — see `docs/BUILD.md`). `check()` reports whether a newer version exists and stashes the `Update` in `PendingUpdate` managed state; `download_and_install()` installs the stashed bundle (minisign-verified, full-bundle replace) streaming progress over a `Channel`, then `commands::download_and_install_update` calls `app.restart()`. Updates are never silent/forced — the user always clicks to install (FR-063). `start_auto_check()` (P2-013, spawned in `lib.rs`) reuses `check()` on a background timer — on startup + every 3h when `config.updates.auto_check` is on (default; re-read each tick so toggling off in Settings stops it) — and emits `update-available` to the panel so the bell lights up. It is **check-only** (never downloads/installs); this is the disclosed exception to the Phase-0/1 "no automatic update checks" promise (FR-056), opt-out via Settings → Updates.
- `error.rs` — `AppError` serializes to its message string only. Messages must never embed API keys, prompts, translation text, or provider responses.

### Provider abstraction (`src-tauri/src/providers/`)
`mod.rs` defines the `Provider` async trait, the `Prompt` (system + user) the trait's `translate` consumes,
and the shared `TranslationRequest`/`TranslationResponse`/`ProviderMeta` types. `adapter(ProviderId)` is the
factory; `all()` builds the metadata list for the settings UI. The four cloud adapters (`openai.rs`,
`anthropic.rs`, `gemini.rs`, `openrouter.rs`) make **real** direct BYOK HTTP calls (P0-008) through the shared
`http.rs` helpers — one pooled `reqwest` client + status→message normalization that never leaks the key (it
strips the URL from transport errors and ignores auth-error bodies). `ollama.rs` is a **real local adapter**
(P1-004): it talks to a local Ollama server (`/api/tags` for models, `/api/chat` for translation, NDJSON
streaming via `http::stream_ndjson`), is `available()` + `supports_streaming()`, and is **keyless** — it's
addressed by an endpoint URL (`config::Providers::ollama_endpoint`, default `http://localhost:11434`) which the
orchestrator passes through the same `api_key` slot the cloud adapters use for their key (`commands::provider_credential`
resolves Keychain-key vs endpoint), so the `Provider` trait stays unchanged.
`Provider::translate` returns just the completion **text**; the orchestrator (`commands::translate` →
`translation::build_prompt`) resolves routing, builds the `Prompt`, times the call, and assembles the
`TranslationResponse` — so adapters stay response-agnostic and error/latency logic lives in one place.
Adding/changing a provider touches several places: the `ProviderId` enum + its `as_str()`, a module + an
`adapter()` match arm + the `all()` list, a field on `config::Providers`, and the `ProviderId` union in `src/lib/tauri.ts`.

**Transport decisions (PRD §15.5):** adapters use raw `reqwest` (system TLS, no OpenSSL) — **not provider
SDKs** (none are first-party for Rust; the trait is already the abstraction).

**Streaming (P1-009):** the four cloud adapters now stream — they parse the provider's SSE response (via the
shared `http::stream_sse`) and override `Provider::translate_stream`, returning `supports_streaming() == true`.
`translate_stream` invokes an `on_delta: &(dyn Fn(String) + …)` sink per chunk **and** returns the full text, so
response assembly (latency/trim/`TranslationResponse`) stays in one place and Enter-to-copy works on the finished
result. The orchestration lives in `commands::translate_stream` (shared `prepare`/`finish` helpers with the
non-streaming `commands::translate`): it wraps a Tauri `ipc::Channel<TranslationDelta>` in the sink, and the
frontend (`Translate.svelte`) creates the `Channel`, appends `{text}` deltas as they arrive, then settles on the
returned trimmed text. **The non-streaming `translate` command/path remains the fallback** — the frontend uses it
for providers whose `supportsStreaming` is `false`, and the trait's default `translate_stream` also degrades to one
`translate` call. Ollama also streams (NDJSON via `http::stream_ndjson`, P1-004).
