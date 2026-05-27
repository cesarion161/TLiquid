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

## Using TLiquid

Click the menu-bar icon to open the panel; click the **⚙ gear** (top-right) to
open **Settings**. For first-time setup you'll mainly use three of its sections:

1. **Languages** — your **primary** language (English by default) and an optional
   **secondary**. Add any number of additional target languages; reorder or
   remove them. (No language cap — it's BYOK.)
2. **Providers** — paste an API key for **OpenAI**, **Anthropic**, **Gemini**, or
   **OpenRouter**, then **Save**. Use **Test** to verify the key; the status reads
   *Configured*, *Connection OK*, or *Invalid key* — or shows the provider error
   if the connection fails. **Remove** deletes the key from the Keychain.
3. **Models** — pick the **default provider** (only providers with a saved key are
   selectable) and a **default model** (fetched live from the provider; if the
   list can't load you can type a model id manually).

Then translate. The **Target** selector is a sticky session choice: an explicit
language is always used; **Auto** applies the primary/secondary rules
(non-primary → primary; primary → secondary, or stays primary with no secondary).

- **Manually:** click the menu-bar icon to open the panel, type/paste text, set
  the **Target**, press **Translate** (or Enter). **Copy**, or press **Enter** to
  copy and dismiss the panel.
- **Selected text (⌘⇧T):** select text in any app and press the hotkey — TLiquid
  translates it to the current **Target**, opens prefilled, and shows the result;
  press **Enter** to copy and dismiss. If **nothing is selected, it does nothing**
  (no panel). If Accessibility permission isn't granted, it opens with guidance to
  enable it.
- **Translate to secondary (⌘⇧⌥T):** translates the selection to your secondary
  language (and sets the Target to it). If no secondary is configured, the panel
  opens to **Settings** (Languages is at the top).

The panel drops down under the menu-bar icon, **remembers where you drag it**, and
**auto-hides when it loses focus** (click outside, switch apps, or press **Esc**) —
re-open it from the tray icon (or the hotkey, with text selected). Shortcuts can be
toggled off in **Settings → Shortcuts** (custom remapping is a later phase).

### How your API keys are stored

Keys are stored in the **macOS Keychain** (service `com.tliquid.app`, one entry
per provider) — never in the settings file, logs, error messages, or the
diagnostics export. (FR-050–FR-052) The non-secret settings file lives in the app
config dir; its exact path and a **Reveal in Finder** button are in
**Settings → Settings file**.

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
│  ├─ App.svelte          # the panel: titlebar + view switch + hotkey routing
│  ├─ Translate.svelte    # manual + selected-text translate view
│  ├─ Result.svelte       # output pane (copy / Enter-to-copy / errors)
│  ├─ Settings.svelte     # loads settings; hosts the section components
│  ├─ LanguageSettings.svelte / ShortcutSettings.svelte
│  ├─ ProviderSettings.svelte / PrivacySettings.svelte / AboutSettings.svelte
│  └─ lib/
│     ├─ tauri.ts                              # typed IPC wrappers (the only invoke site)
│     ├─ languages.ts                          # selectable language list
│     └─ styles.css                            # design system
├─ src-tauri/                                  # Rust backend
│  ├─ src/
│  │  ├─ lib.rs            # builder: plugins, panel, tray, macOS accessory mode
│  │  ├─ tray.rs          # menu-bar shell; left-click toggles the panel
│  │  ├─ windows.rs       # the single panel: create-hidden, show/hide, tray-anchored
│  │  ├─ commands.rs      # Tauri commands exposed to the UI
│  │  ├─ config.rs        # non-secret settings (PRD §16)
│  │  ├─ secrets.rs       # macOS Keychain storage
│  │  ├─ shortcuts.rs     # global hotkey registration
│  │  ├─ capture.rs       # macOS selected-text capture (simulated ⌘C)
│  │  ├─ languages.rs     # primary/secondary routing engine
│  │  ├─ translation.rs   # prompt templates + orchestrator
│  │  ├─ diagnostics.rs   # local diagnostics export (no upload)
│  │  ├─ error.rs
│  │  └─ providers/       # http core + OpenAI / Anthropic / Gemini / OpenRouter / Ollama
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

## Known limitations (Phase 0)

- **macOS only.** Windows and Linux are **not verified** targets. The code is
  kept portable via Tauri, but only macOS behavior is accepted (PRD §3.1).
- **Selected-text capture** simulates ⌘C, so it requires Accessibility permission
  and works only in apps that respond to Copy. It briefly uses the clipboard and
  restores the previous **text**; non-text clipboard contents (e.g. an image)
  can't be preserved. A selection identical to the current clipboard reads as
  "no selection."
- **Unsigned build** — see the Gatekeeper bypass above.
- **Non-streaming.** Translations appear all at once when the provider responds
  (streaming output is a Phase 1 goal).
- **Local models (Ollama)** are not available yet (Phase 1).
- The result's target-language label is best-effort in Auto/primary mode (the
  model picks the real target from the detected source).

## Troubleshooting

- **Nothing happens on the hotkey / "No text was captured":** grant **Accessibility**
  permission (System Settings → Privacy & Security → Accessibility); the app's
  error offers a one-click button to that pane. Make sure text is actually
  selected and the app supports Copy. Use the manual panel as a fallback.
- **"Invalid key" or a connection error:** re-check the key in Settings → Providers
  and **Test** it; confirm network access and that the provider/model is available.
- **No models to choose / list won't load:** the provider's model API may be
  unavailable — type the model id manually in Settings → Models, or **Retry**.
- **App won't open ("Apple cannot check it…"):** right-click → Open, or
  `xattr -dr com.apple.quarantine /Applications/TLiquid.app` (unsigned build).
- **"Not running inside the TLiquid app":** you opened the dev URL in a browser —
  use the window `pnpm tauri dev` opens instead.
- **Bug reports:** Settings → Privacy → **Copy diagnostics** copies a local,
  non-sensitive summary (no keys or text) to paste into an issue.

## Status

Phase 0 (macOS MVP) is feature-complete: menu-bar shell, settings (languages,
shortcuts, providers/models, privacy/about), secure key storage, manual and
selected-text translation against real BYOK providers, copy/Enter behavior, and
an installable `.app`. Remaining work is end-to-end QA (**P0-021**) and signing
(**P1-008**). Epic-by-epic status is tracked in [`tliquid_todo.md`](./tliquid_todo.md).

## License

[MIT](./LICENSE) © TLiquid contributors
