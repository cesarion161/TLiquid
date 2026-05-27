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
variable in `App.svelte` (`view: "translate" | "settings"`), not by routing or by opening
new windows. So there is a single `index.html` → `src/main.ts` → `App.svelte`; `Settings.svelte`
and `Result.svelte` are child components of `App`, not window entry points. Adding a "screen"
means adding a component and a branch to `App.svelte`'s view switch — **not** a new HTML
entry or window.

The window itself (`src-tauri/src/windows.rs`, label `"main"`) is:
- **Created once at startup, hidden** (`create_panel`), so summoning it is an instant `show`/`hide`, never a fresh webview load. Dev builds auto-show it; release stays hidden until summoned.
- **Frameless + `always_on_top` + `visible_on_all_workspaces`**, which — combined with the macOS Accessory activation policy set in `lib.rs` — lets it float over other apps including fullscreen Spaces. The titlebar is drawn in the UI (`.titlebar` with `data-tauri-drag-region`).
- **Anchored under the tray icon**: `tray.rs` left-click calls `toggle_panel` with the click position; `windows::position_under` centers the panel beneath it, clamped to the monitor. Right-click opens the tray menu; "Settings…" shows the panel and emits a `navigate` event the frontend listens for to switch views.

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
- `tray.rs` — menu-bar shell; the tray is the app's primary surface. Left-click toggles the panel anchored under the icon; right-click opens a menu (Open / Settings… / Quit).
- `windows.rs` — the single panel window: create-hidden-at-startup, show/hide/toggle, tray-anchored positioning (above).
- `commands.rs` — the IPC surface (above).
- `config.rs` — non-secret settings, persisted as `settings.json` in the app config dir. Corrupt files are renamed to `.json.bak` and defaults used, never silently discarded. `Settings::default()` is the source of truth for defaults.
- `secrets.rs` — API keys in the **macOS Keychain** via the `keyring` crate (service `com.tliquid.app`, account = provider id). Keys never go in `settings.json` or logs.
- `languages.rs` — routing engine. `resolve()` returns a `Resolution`. Primary-mode routing is **not** resolved to one target up front — source detection happens inside the LLM, so the routing rules are encoded into the prompt (`PrimaryRouted`).
- `translation.rs` — pure, provider-neutral prompt builders. No translation text is persisted anywhere.
- `providers/` — provider abstraction (see below).
- `error.rs` — `AppError` serializes to its message string only. Messages must never embed API keys, prompts, translation text, or provider responses.

### Provider abstraction (`src-tauri/src/providers/`)
`mod.rs` defines the `Provider` async trait, the `Prompt` (system + user) the trait's `translate` consumes,
and the shared `TranslationRequest`/`TranslationResponse`/`ProviderMeta` types. `adapter(ProviderId)` is the
factory; `all()` builds the metadata list for the settings UI. The four cloud adapters (`openai.rs`,
`anthropic.rs`, `gemini.rs`, `openrouter.rs`) make **real** direct BYOK HTTP calls (P0-008) through the shared
`http.rs` helpers — one pooled `reqwest` client + status→message normalization that never leaks the key (it
strips the URL from transport errors and ignores auth-error bodies). `ollama.rs` stays a Phase 1 stub (P1-004).
`Provider::translate` returns just the completion **text**; the orchestrator (`commands::translate` →
`translation::build_prompt`) resolves routing, builds the `Prompt`, times the call, and assembles the
`TranslationResponse` — so adapters stay response-agnostic and error/latency logic lives in one place.
Adding/changing a provider touches several places: the `ProviderId` enum + its `as_str()`, a module + an
`adapter()` match arm + the `all()` list, a field on `config::Providers`, and the `ProviderId` union in `src/lib/tauri.ts`.

**Transport decisions (PRD §15.5):** adapters use raw `reqwest` (system TLS, no OpenSSL) — **not provider
SDKs** (none are first-party for Rust; the trait is already the abstraction). **Phase 0 is non-streaming**:
the orchestrator awaits the full completion and returns one `TranslationResponse`, and `supports_streaming()` is
`false` everywhere. Streaming (SSE for cloud providers / NDJSON for Ollama → a Tauri channel) is a Phase 1
task (P1-009), so don't reach for `reqwest`'s `stream` feature in Phase 0.
