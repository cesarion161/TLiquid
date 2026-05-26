# TLiquid

> A tiny, open-source, **macOS-first** bring-your-own-key (BYOK) LLM translator
> that lives in your menu bar and translates selected text instantly.

```text
Select text → press global hotkey → translation appears → press Enter to copy
```

TLiquid is a small system utility, not a chat app. It runs in the background,
exposes a persistent menu-bar icon, and routes translation requests directly
from your machine to the LLM provider you configure (OpenAI, Anthropic, Gemini,
or OpenRouter). No translation text is ever sent to TLiquid servers, and there
is no telemetry.

This repository is the **Phase 0 (macOS MVP)** foundation. See
[`llm_translator_prd.md`](./llm_translator_prd.md) for the full product spec and
[`tliquid_todo.md`](./tliquid_todo.md) for the epic tracker.

> **Scope note:** Phase 0 is macOS only. Windows and Linux are *not* verified
> targets yet (the architecture is kept portable via Tauri, but only macOS
> behavior is accepted). See PRD §3.1.

## Tech stack

| Layer          | Choice                                            |
| -------------- | ------------------------------------------------- |
| Desktop shell  | [Tauri v2](https://tauri.app)                     |
| Core language  | Rust                                              |
| UI             | Svelte 5 + Vite + TypeScript (multi-window SPA)   |
| Secrets        | macOS Keychain (`keyring` crate)                  |
| HTTP           | `reqwest` (system TLS — no OpenSSL)               |
| Package manager| pnpm                                              |

Each native window (manual popup, settings, result popup) is its own HTML entry
point built by Vite and opened on demand by the Rust side.

## Prerequisites

- **macOS 12+** on Apple Silicon or Intel
- **Xcode Command Line Tools** (`xcode-select --install`)
- **Rust** (stable) via [rustup](https://rustup.rs)
- **Node.js 20+** and **pnpm 10+** (`npm i -g pnpm`)

## Getting started

```bash
# Install JS dependencies
pnpm install

# Run the app in development (hot-reloading UI + Rust rebuilds)
pnpm tauri dev
```

On launch the app installs a **menu-bar icon** and stays out of the Dock. In
dev builds the main window also opens automatically; in release it stays
menu-bar-only (open it from the tray).

> **Don't open `http://localhost:1420` in a browser.** The Tauri IPC runtime
> only exists inside the app's own window — a browser has no `invoke`, so the UI
> will show an error. Always interact through the window `pnpm tauri dev` opens.

## Build commands

```bash
# Frontend only (outputs to ./dist)
pnpm build

# Full app build without bundling/signing (fast; used in CI)
pnpm tauri build --no-bundle

# Full macOS bundle (.app + .dmg in src-tauri/target/release/bundle)
pnpm tauri build
```

> Producing a signed/notarized installable artifact is tracked separately in
> task **P0-019**; an unsigned local build is fine for MVP testing.

## Lint, format & test

```bash
# Frontend type-check
pnpm check

# Rust format / lint / tests
cargo fmt   --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test  --manifest-path src-tauri/Cargo.toml
```

A starter GitHub Actions workflow lives in
[`.github/workflows/ci.yml`](./.github/workflows/ci.yml).

## Project structure

```text
.
├─ index.html / settings.html / result.html   # one entry per native window
├─ src/                                        # Svelte 5 frontend
│  ├─ main.ts / settings.ts / result.ts        # window mount points
│  ├─ App.svelte / Settings.svelte / Result.svelte
│  └─ lib/
│     ├─ tauri.ts                              # typed IPC wrappers
│     └─ styles.css
├─ src-tauri/                                  # Rust backend
│  ├─ src/
│  │  ├─ lib.rs            # builder: plugins, tray, macOS accessory mode
│  │  ├─ tray.rs          # menu-bar shell
│  │  ├─ windows.rs       # on-demand window creation
│  │  ├─ commands.rs      # Tauri commands exposed to the UI
│  │  ├─ config.rs        # non-secret settings (PRD §16)
│  │  ├─ secrets.rs       # macOS Keychain storage
│  │  ├─ languages.rs     # primary/secondary routing engine
│  │  ├─ translation.rs   # prompt templates + orchestrator
│  │  ├─ error.rs
│  │  └─ providers/       # OpenAI / Anthropic / Gemini / OpenRouter / Ollama
│  ├─ capabilities/       # window permissions
│  └─ tauri.conf.json
└─ .github/workflows/ci.yml
```

## Privacy

- **BYOK, local-first:** translation requests go directly from your machine to
  the provider you configure. (PRD FR-020, FR-044)
- **No telemetry** and **no automatic update checks** in Phase 0.
- **API keys** are stored in the **macOS Keychain**, never in the settings file
  or logs. (FR-050–FR-052)
- **No translation history** is stored by default.

Translated text *is* sent to your chosen provider — that is inherent to LLM
translation and is disclosed here intentionally.

## Status

Phase 0 foundation (**task P0-001**) is in place: the app builds and runs as a
macOS menu-bar utility with the full dependency set, module skeleton, and IPC
surface wired. Feature epics (settings UI, providers, selected-text capture,
hotkeys) follow per [`tliquid_todo.md`](./tliquid_todo.md).

## License

[MIT](./LICENSE) © TLiquid contributors
