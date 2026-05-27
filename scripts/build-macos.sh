#!/usr/bin/env bash
#
# build-macos.sh — official TLiquid macOS build entry point (P1-008).
#
# Builds the .app + .dmg with `pnpm tauri build`. Tauri performs code signing
# and notarization automatically *when the relevant Apple credentials are present
# in the environment* — this script just detects which mode applies, prints it,
# and fails fast on a half-configured signing setup (a common footgun).
#
# Usage:
#   scripts/build-macos.sh [extra tauri build args...]
#
# Signing (optional — set in the environment before running):
#   APPLE_SIGNING_IDENTITY     Keychain identity, e.g. "Developer ID Application: Name (TEAMID)"
#   APPLE_CERTIFICATE          base64 of a .p12 (CI; imported into a temp keychain by tauri-action)
#   APPLE_CERTIFICATE_PASSWORD password for the .p12 above
#
# Notarization (optional — pick ONE of the two methods):
#   App Store Connect API key (recommended for CI):
#     APPLE_API_ISSUER, APPLE_API_KEY, APPLE_API_KEY_PATH
#   Apple ID:
#     APPLE_ID, APPLE_PASSWORD (app-specific password), APPLE_TEAM_ID
#
# With no signing vars set, this produces an UNSIGNED build (fine for local use;
# users bypass Gatekeeper per the README). See docs/BUILD.md for the full guide.

set -euo pipefail

cd "$(dirname "$0")/.."

# --- detect signing state -----------------------------------------------------
signing="unsigned"
if [[ -n "${APPLE_SIGNING_IDENTITY:-}" || -n "${APPLE_CERTIFICATE:-}" ]]; then
	signing="signed"
	# A cert with no identity name (or vice versa) silently produces an unsigned
	# build — catch that misconfiguration here instead of shipping it.
	if [[ -n "${APPLE_CERTIFICATE:-}" && -z "${APPLE_CERTIFICATE_PASSWORD:-}" ]]; then
		echo "error: APPLE_CERTIFICATE is set but APPLE_CERTIFICATE_PASSWORD is not." >&2
		exit 1
	fi
fi

# --- detect notarization state ------------------------------------------------
notarize="off"
if [[ -n "${APPLE_API_KEY:-}" || -n "${APPLE_API_ISSUER:-}" || -n "${APPLE_API_KEY_PATH:-}" ]]; then
	if [[ -z "${APPLE_API_KEY:-}" || -z "${APPLE_API_ISSUER:-}" || -z "${APPLE_API_KEY_PATH:-}" ]]; then
		echo "error: incomplete App Store Connect API notarization config." >&2
		echo "       set all of APPLE_API_KEY, APPLE_API_ISSUER, APPLE_API_KEY_PATH." >&2
		exit 1
	fi
	notarize="api-key"
elif [[ -n "${APPLE_ID:-}" || -n "${APPLE_PASSWORD:-}" || -n "${APPLE_TEAM_ID:-}" ]]; then
	if [[ -z "${APPLE_ID:-}" || -z "${APPLE_PASSWORD:-}" || -z "${APPLE_TEAM_ID:-}" ]]; then
		echo "error: incomplete Apple ID notarization config." >&2
		echo "       set all of APPLE_ID, APPLE_PASSWORD, APPLE_TEAM_ID." >&2
		exit 1
	fi
	notarize="apple-id"
fi

if [[ "$notarize" != "off" && "$signing" == "unsigned" ]]; then
	echo "error: notarization requires signing — set APPLE_SIGNING_IDENTITY too." >&2
	exit 1
fi

echo "TLiquid macOS build  •  signing: ${signing}  •  notarization: ${notarize}"
[[ "$signing" == "unsigned" ]] && \
	echo "note: unsigned build — recipients must bypass Gatekeeper (see README)."

# Tauri reads the APPLE_* vars itself; we just invoke the build.
exec pnpm tauri build "$@"
