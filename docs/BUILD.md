# Building, signing & notarizing TLiquid (macOS)

This is the official macOS build process for TLiquid (Phase 1, **P1-008**). It
covers the unsigned local build, the signed + notarized release build, and the
GitHub Actions release pipeline.

> **TL;DR**
> - **Local / unsigned:** `pnpm tauri build` (or `scripts/build-macos.sh`). Works
>   with no Apple account; recipients bypass Gatekeeper once (see README).
> - **Release / signed + notarized:** set the `APPLE_*` env vars below, then run
>   `scripts/build-macos.sh`. Requires a paid Apple Developer account.

---

## 1. Prerequisites

- macOS 12+ with **Xcode Command Line Tools** (`xcode-select --install`).
- **Rust** (stable), **Node 20+**, **pnpm 10+** — see the README.
- For a signed release: a paid **Apple Developer Program** membership and a
  **Developer ID Application** certificate (this is what allows direct
  distribution outside the App Store).

## 2. Build outputs

`pnpm tauri build` produces, under `src-tauri/target/release/bundle/`:

```text
macos/TLiquid.app                 # the app bundle
dmg/TLiquid_<version>_<arch>.dmg  # drag-to-install disk image
```

The DMG window layout (size + icon positions) is configured in
`tauri.conf.json` under `bundle.macOS.dmg`, so the installer shows the app icon
next to an Applications-folder shortcut.

> **Headless/CI note:** the styled `.dmg` step drives Finder via AppleScript and
> needs a GUI session; it can fail headless (CI uses `--no-bundle`). The `.app`
> is always produced. To package a DMG without Finder, use the `hdiutil` fallback
> in the README.

## 3. Unsigned local build

```bash
scripts/build-macos.sh           # wraps `pnpm tauri build`, prints signing mode
# or directly:
pnpm tauri build
```

With no `APPLE_*` variables set this is **unsigned**. It runs fine locally; the
first launch needs a one-time Gatekeeper bypass (right-click → Open, or
`xattr -dr com.apple.quarantine /Applications/TLiquid.app`).

> **Dev tip — stable identity:** unsigned builds get a *new* code identity each
> rebuild, so macOS forgets the Accessibility grant every time (capture
> re-prompts). To keep the grant during development, create a **self-signed
> code-signing certificate** in Keychain Access ("Certificate Assistant → Create
> a Certificate", type *Code Signing*) and build with
> `APPLE_SIGNING_IDENTITY="<that cert's name>" scripts/build-macos.sh`. This is
> *ad-hoc-ish* local signing — it does **not** notarize and won't pass Gatekeeper
> on other machines, but it stabilizes the Accessibility permission for you.

## 4. Signed + notarized release build

Tauri reads the Apple credentials from the environment during `pnpm tauri build`
and signs (with the Hardened Runtime + `Entitlements.plist`) and notarizes
automatically. `scripts/build-macos.sh` validates that the variables are
internally consistent before building.

### 4.1 Code signing

Set the signing identity from a **Developer ID Application** cert installed in
your login keychain:

```bash
export APPLE_SIGNING_IDENTITY="Developer ID Application: Your Name (TEAMID)"
# (find the exact string with: security find-identity -v -p codesigning)
```

The bundle is signed with the Hardened Runtime enabled
(`bundle.macOS.hardenedRuntime: true`) and the entitlements in
`src-tauri/Entitlements.plist` (JIT for the webview; **no** App Sandbox, **no**
AppleEvents — selected-text capture uses CGEvent synthesis, gated by
Accessibility/TCC at runtime).

### 4.2 Notarization

Pick **one** method. App Store Connect API key is recommended (no 2FA prompts,
ideal for CI):

```bash
# Method A — App Store Connect API key
export APPLE_API_ISSUER="<issuer-uuid>"
export APPLE_API_KEY="<key-id>"
export APPLE_API_KEY_PATH="/absolute/path/AuthKey_<key-id>.p8"

# Method B — Apple ID + app-specific password
export APPLE_ID="you@example.com"
export APPLE_PASSWORD="<app-specific-password>"   # appleid.apple.com → App-Specific Passwords
export APPLE_TEAM_ID="TEAMID"
```

Then build:

```bash
scripts/build-macos.sh
```

Tauri submits the signed bundle to Apple, waits for the result, and **staples**
the notarization ticket to the `.app` and `.dmg`. Verify afterward:

```bash
spctl -a -vvv -t install src-tauri/target/release/bundle/macos/TLiquid.app
# expected: "source=Notarized Developer ID"
xcrun stapler validate src-tauri/target/release/bundle/macos/TLiquid.app
```

## 5. CI release pipeline

`.github/workflows/release.yml` builds the macOS bundle on tag pushes
(`v*`). It signs + notarizes **only when the corresponding repository secrets are
present**, and otherwise produces an unsigned build — so the workflow works in a
fork without any Apple account.

Configure these repository **secrets** to enable signing/notarization. The
workflow derives the signing identity *from the certificate* (via
`security find-identity`), so there is no separate `APPLE_SIGNING_IDENTITY`
secret to forget:

| Secret | Purpose |
|---|---|
| `APPLE_CERTIFICATE` | base64 of your Developer ID `.p12` |
| `APPLE_CERTIFICATE_PASSWORD` | password for that `.p12` |
| `KEYCHAIN_PASSWORD` | any string; password for the temp CI keychain |
| `APPLE_API_ISSUER` / `APPLE_API_KEY` / `APPLE_API_KEY_PATH_B64` | App Store Connect API key for notarization (`_B64` is the base64 of the `.p8`) |

Signing requires `APPLE_CERTIFICATE` + `APPLE_CERTIFICATE_PASSWORD` +
`KEYCHAIN_PASSWORD`; notarization additionally needs the three `APPLE_API_*`
secrets. With none set, the workflow builds unsigned. To export the `.p12` to
base64 for the secret:

```bash
base64 -i certificate.p12 | pbcopy
```

## 6. In-app update signing (P2-007)

TLiquid's in-app updater (Settings → Updates) downloads a **full app bundle**
(`.app.tar.gz`) from GitHub Releases and verifies it against a **minisign**
public key embedded in `tauri.conf.json` (`plugins.updater.pubkey`). This is a
**separate key from the Apple Developer cert** above — update verification does
not depend on Apple signing, so auto-updates work even while the app is unsigned.

### 6.1 The signing keypair

A keypair was generated with `pnpm tauri signer generate`. The **public** key is
committed in `tauri.conf.json`. The **private** key lives at
`.tauri/tliquid_updater.key` (gitignored — **never commit it**) and must be
mirrored to a repository secret:

| Secret | Purpose |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | the contents of `.tauri/tliquid_updater.key` |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | the key's password (empty for this key) |

```bash
# add the private key as a secret (requires the gh CLI, authenticated)
gh secret set TAURI_SIGNING_PRIVATE_KEY < .tauri/tliquid_updater.key
gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD --body ''
```

> **Lost key = no more updates.** If the private key is lost, you cannot sign
> updates that existing installs will accept; users would have to reinstall
> manually. Back it up securely. To rotate, generate a new keypair, replace the
> `pubkey` in `tauri.conf.json`, and ship it in a release signed with the **old**
> key (so current installs accept the update that introduces the new key).

### 6.2 How a release produces updates

`createUpdaterArtifacts` is deliberately **kept out of the base
`tauri.conf.json`**: with a `pubkey` configured, the bundler hard-requires
`TAURI_SIGNING_PRIVATE_KEY` whenever that flag is on, which would break the
unsigned local build (§3) and fork CI. It is instead enabled by a partial-config
override, `src-tauri/tauri.updater.conf.json`, applied **only when the signing
key is present**:

- **In CI** (`release.yml`): the "Enable signed updater artifacts" step adds
  `--config src-tauri/tauri.updater.conf.json` to the build only when
  `TAURI_SIGNING_PRIVATE_KEY` is set. No key → no override → unsigned build with
  no updater artifacts (still succeeds, so forks work).
- **Locally** (`scripts/build-macos.sh`): the same override is added
  automatically when `TAURI_SIGNING_PRIVATE_KEY` (or `_PATH`) is exported, e.g.:

  ```bash
  export TAURI_SIGNING_PRIVATE_KEY="$(cat .tauri/tliquid_updater.key)"
  export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=''
  scripts/build-macos.sh           # prints "updater: artifacts"
  ```

When enabled, the build:

1. builds the `.app.tar.gz` updater bundle and signs it (`.sig`);
2. generates `latest.json` (version + per-target download URL + signature);
3. (in CI) uploads both to the GitHub Release.

The app's updater endpoint is
`https://github.com/cesarion161/TLiquid/releases/latest/download/latest.json`.

> **Publish the draft.** The workflow creates a **draft** release
> (`releaseDraft: true`). GitHub's `/releases/latest/` only serves the newest
> **published** (non-draft, non-prerelease) release, so `latest.json` is not
> reachable — and clients see no update — until you **publish** the draft. Review
> the artifacts, then publish.

## 7. Status

- **Implemented:** Hardened-Runtime + entitlements config, env-driven signing &
  notarization, the DMG installer layout, the `build-macos.sh` helper, the
  tag-triggered release workflow, and this document.
- **Requires owner action:** producing an actually signed + notarized artifact
  needs a paid Apple Developer account and the certificate/API-key secrets above.
  Until those exist, releases remain **unsigned** (acceptable per FR-075).
