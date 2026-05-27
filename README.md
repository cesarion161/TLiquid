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
| UI             | Svelte 5 + Vite + TypeScript (single panel)       |
| Secrets        | macOS Keychain (`keyring` crate)                  |
| HTTP           | `reqwest` (system TLS — no OpenSSL)               |
| Package manager| pnpm                                              |

TLiquid is **one window** — a frameless menu-bar panel that drops down under
the tray icon, in the spirit of [Raycast](https://raycast.com),
[Docker Desktop](https://docker.com)'s tray panel, and JetBrains Toolbox. The
translate input/result and Settings are *views inside that one panel*, not
separate windows. The panel is created once at startup (hidden), shown/hidden
on tray click, and floats above other apps — including fullscreen Spaces — so
it can be summoned from anywhere.

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

On launch the app installs a **menu-bar icon** and stays out of the Dock. The
panel is created hidden up front so opening it is instant. In dev builds it is
shown automatically; in release it stays hidden until you click the tray icon
(or trigger a hotkey).

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

## Install (macOS)

`pnpm tauri build` produces a self-contained, installable app bundle under
`src-tauri/target/release/bundle/`:

```text
src-tauri/target/release/bundle/
├─ macos/TLiquid.app                 # the installable app bundle (~4.7 MB)
└─ dmg/TLiquid_0.1.0_<arch>.dmg      # drag-to-install disk image (GUI Macs)
```

**TLiquid.app** is the installable artifact: open the `.dmg` and drag it to
`/Applications`, or just double-click the `.app`. Launch it; it appears as a
menu-bar icon.

> **Headless/CI note:** the styled `.dmg` step (`bundle_dmg.sh`) drives Finder via
> AppleScript to lay out the disk-image window, so it needs a GUI session and can
> fail in headless/CI environments (which is why CI runs `--no-bundle`). The
> `.app` is always produced regardless. To package a disk image without Finder:
>
> ```bash
> hdiutil create -volname TLiquid -ov -format UDZO \
>   -srcfolder src-tauri/target/release/bundle/macos/TLiquid.app \
>   TLiquid_0.1.0_aarch64.dmg
> ```

### Unsigned build — bypassing Gatekeeper

Phase 0 ships **unsigned and un-notarized** (see below), so on first launch macOS
Gatekeeper will say *"TLiquid can't be opened because Apple cannot check it for
malicious software."* This is expected for a local/MVP build. To open it:

- **Right-click** (or Control-click) TLiquid.app → **Open** → **Open** in the
  dialog. macOS remembers the choice for future launches; **or**
- clear the quarantine attribute:
  ```bash
  xattr -dr com.apple.quarantine /Applications/TLiquid.app
  ```

### First-run permission

Selected-text capture simulates ⌘C, which requires **Accessibility** permission.
macOS prompts the first time you use a translation hotkey; grant TLiquid access in
**System Settings → Privacy & Security → Accessibility**. (The app also offers a
one-click shortcut to that pane when a capture fails.) Manual translation in the
panel needs no special permission.

### Signing & notarization (deferred)

Code signing and notarization are **deferred in Phase 0** (FR-075): they require a
paid Apple Developer account, which this MVP doesn't assume. An unsigned local
build is acceptable for internal testing — use the Gatekeeper bypass above.
Producing a signed, notarized release is tracked as **P1-008**.

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
├─ index.html                                  # the one window entry
├─ src/                                        # Svelte 5 frontend
│  ├─ main.ts                                  # mounts the panel
│  ├─ App.svelte         # the panel: titlebar + view switch (translate/settings)
│  ├─ Settings.svelte / Result.svelte          # views inside the panel, not windows
│  └─ lib/
│     ├─ tauri.ts                              # typed IPC wrappers
│     └─ styles.css
├─ src-tauri/                                  # Rust backend
│  ├─ src/
│  │  ├─ lib.rs            # builder: plugins, panel, tray, macOS accessory mode
│  │  ├─ tray.rs          # menu-bar shell; left-click toggles the panel
│  │  ├─ windows.rs       # the single panel: create-hidden, show/hide, tray-anchored
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
