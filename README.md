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

# Official build entry point: builds + signs + notarizes when APPLE_* env vars
# are set, otherwise an unsigned build (see docs/BUILD.md)
scripts/build-macos.sh
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

### Signing & notarization

The build pipeline **supports** signing and notarization (Hardened Runtime +
`Entitlements.plist`, env-driven), but producing a signed artifact requires a
paid Apple Developer account, so default builds are **unsigned** — acceptable for
local/internal use (FR-075), with the Gatekeeper bypass above.

To produce a signed + notarized release, set the `APPLE_*` credentials and run
`scripts/build-macos.sh`; the full process (local and CI) is documented in
**[`docs/BUILD.md`](./docs/BUILD.md)**. A tag-triggered GitHub Actions release
workflow (`.github/workflows/release.yml`) signs/notarizes automatically when the
repository secrets are configured.

> **Heads-up — Accessibility permission resets on each rebuild.** Unsigned (ad-hoc)
> builds get a *new* code identity every time they're built, and macOS ties the
> Accessibility grant to that identity. So after rebuilding, the existing "TLiquid"
> entry in **Privacy & Security → Accessibility** becomes stale and capture re-prompts
> even though it looks enabled — **remove it (select it, press “−”) and grant again**.
> To make the grant persist across rebuilds during development, sign with a *stable*
> identity (e.g. a self-signed code-signing cert created in Keychain Access, then set
> `APPLE_SIGNING_IDENTITY="<cert name>"` before `pnpm tauri build`; see
> [`docs/BUILD.md`](./docs/BUILD.md)). A real Developer ID signature fixes this for releases.

## Using TLiquid

Click the menu-bar icon to open the panel; click the **⚙ gear** (top-right) to
open **Settings**. For first-time setup you'll mainly use three of its sections:

1. **Languages** — your **primary** language (English by default) and an optional
   **secondary**. Add any number of additional target languages; reorder or
   remove them. (No language cap — it's BYOK.)
2. **Providers** — paste an API key for **OpenAI**, **Anthropic**, **Gemini**, or
   **OpenRouter**, then **Save**. Use **Test** to verify the key; the status reads
   *Configured*, *Connection OK*, or *Invalid key* — or shows the provider error
   if the connection fails. **Remove** deletes the key from the Keychain. For
   **Ollama** (local), there's no key — set the **endpoint URL** (default
   `http://localhost:11434`), **Save**, and **Test** that the server is reachable.
3. **Models** — pick the **default provider** (only providers with a saved key —
   or Ollama — are selectable) and a **default model** (fetched live from the
   provider; if the list can't load, or you haven't pulled an Ollama model yet,
   you can type a model id manually).

Then translate. The **Target** selector is a sticky session choice: an explicit
language is always used; **Auto** applies the primary/secondary rules
(non-primary → primary; primary → secondary, or stays primary with no secondary).

- **Manually:** click the menu-bar icon to open the panel, type/paste text, set
  the **Target**, press **Translate** (or Enter). **Copy**, or press **Enter** to
  copy and dismiss the panel.
- **Selected text (⌘⇧T):** select text in any app and press the hotkey — TLiquid
  translates it to the current **Target**, opens prefilled, and shows the result;
  press **Enter** to copy and dismiss. If **nothing is selected, the panel opens
  empty** so you can type/paste and translate manually. If Accessibility permission
  isn't granted, it opens with guidance to enable it.
- **Translate to secondary (⌘⇧⌥T):** translates the selection to your secondary
  language (and sets the Target to it). If no secondary is configured, the panel
  opens to **Settings** (Languages is at the top).

**Launch at login** (P1-001): on first run a dot appears on the **🔔 bell**
(top-left of the panel) — open it to find the offer to start TLiquid at login
(recommended; nothing is enabled without your click). Change it anytime in
**Settings → Startup** — it starts straight into the menu bar. The bell is also
where **new-version alerts** appear.

**Updates** (P2-007): open **Settings → Updates** and click **Check for updates**.
If a newer version exists, a **Download & install** button appears (and the 🔔
bell lights up with the same offer) — clicking it downloads a signed bundle,
verifies it, installs it in place, and relaunches TLiquid. No manual reinstall,
and updates are never silent: you always click to install.

The panel drops down under the menu-bar icon, **remembers where you drag it**, and
**auto-hides when it loses focus** (click outside, switch apps, or press **Esc**) —
re-open it from the tray icon (or the hotkey, with text selected). Shortcuts can be
toggled off in **Settings → Shortcuts**, where you can also **record custom
shortcuts** for translate-selection, translate-to-secondary, and any additional
language (click the shortcut, press your combo; conflicts and invalid combos are
flagged; **Reset to defaults** restores ⌘⇧T / ⌘⇧⌥T).

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
│  ├─ Entitlements.plist  # Hardened-Runtime entitlements for signed builds
│  └─ tauri.conf.json
├─ scripts/build-macos.sh                      # official build/sign/notarize entry point
├─ docs/BUILD.md                               # macOS build, signing & notarization guide
└─ .github/workflows/
   ├─ ci.yml                                   # lint/test/build on push + PR
   └─ release.yml                              # tag-triggered signed release build
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
- **Streaming output** (P1-009): translations stream in incrementally (cloud
  providers and Ollama); press **Enter** once it finishes to copy the complete text.
- **Local models (Ollama)** (P1-004): run a local Ollama server, set its endpoint
  in **Settings → Providers**, and translate with no cloud key. Requires Ollama
  installed and a model pulled (`ollama pull <model>`); errors are shown if the
  server is unreachable.
- The result's target-language label is best-effort in Auto/primary mode (the
  model picks the real target from the detected source).

## Troubleshooting

- **The hotkey opens the panel but doesn't translate:** if nothing is selected the
  panel opens empty (by design) — type/paste to translate. If text *is* selected but
  capture fails, grant **Accessibility** permission (System Settings → Privacy &
  Security → Accessibility; the error offers a one-click button). Make sure the app
  supports Copy; use the manual panel as a fallback.
- **It keeps asking for Accessibility even though TLiquid is enabled in the list:**
  the entry is stale from a previous build — **remove it (select it, press “−”) and
  re-grant**. Unsigned builds get a new identity each rebuild (see "Signing" above).
- **"Invalid key" or a connection error:** re-check the key in Settings → Providers
  and **Test** it; confirm network access and that the provider/model is available.
- **No models to choose / list won't load:** the provider's model API may be
  unavailable — type the model id manually in Settings → Models, or **Retry**.
- **App won't open ("Apple cannot check it…"):** right-click → Open, or
  `xattr -dr com.apple.quarantine /Applications/TLiquid.app` (unsigned build).
- **"Not running inside the TLiquid app":** you opened the dev URL in a browser —
  use the window `pnpm tauri dev` opens instead.
- **Bug reports:** Settings → Privacy → **Copy diagnostics** (clipboard) or
  **Save to file…** exports a local, non-sensitive bundle — app/OS info, your
  settings shape, a recent-error summary, and a tail of the local log (no keys,
  text, or provider responses). Never uploaded.

## Status

Phase 0 (macOS MVP) is complete. **Phase 1** adds: streaming translation output
(P1-009), **Ollama / local models** (P1-004), **configurable shortcuts** with
per-language hotkeys + conflict detection (P1-002), **launch-at-login** with a
first-run consent (P1-001), and a richer **diagnostics bundle** with export
(P1-007). Packaging supports signing + notarization (P1-008), but official signed
builds need a paid Apple Developer account, so default builds remain unsigned. The
automated gate is green (78 tests) and the full `.app` + `.dmg` build; remaining
sign-off is the on-device checklist in [`QA.md`](./QA.md) and signing. Epic-by-epic
status is tracked in [`tliquid_todo.md`](./tliquid_todo.md).

## License

[MIT](./LICENSE) © TLiquid contributors
