#!/usr/bin/env bash
#
# release.sh — cut a TLiquid release for the in-app updater (P2-007).
#
# Bumps the version in the three manifests + Cargo.lock, commits, tags `vX.Y.Z`,
# and pushes. The pushed tag triggers .github/workflows/release.yml, which
# builds, minisign-signs the update bundle, generates latest.json, and creates a
# DRAFT GitHub release. You then review and PUBLISH the draft — the updater only
# sees published releases:
#
#   gh release edit vX.Y.Z --draft=false --latest
#
# Usage:
#   scripts/release.sh 0.1.2
#
# Preconditions (enforced): a clean working tree, on `main`, the vX.Y.Z tag must
# not already exist, and the version must be a higher semver than what users have
# (else installed apps just report "Up to date"). The one-time setup — the
# TAURI_SIGNING_PRIVATE_KEY secret and read-write Actions permissions — is already
# done (see docs/BUILD.md §6).

set -euo pipefail
cd "$(dirname "$0")/.."

# --- args ---------------------------------------------------------------------
VERSION="${1:-}"
if [[ -z "$VERSION" ]]; then
	echo "usage: scripts/release.sh <version>   e.g. scripts/release.sh 0.1.2" >&2
	exit 1
fi
if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
	echo "error: version must be semver MAJOR.MINOR.PATCH (e.g. 0.1.2), got '$VERSION'." >&2
	exit 1
fi
TAG="v$VERSION"

# --- preconditions ------------------------------------------------------------
BRANCH=$(git rev-parse --abbrev-ref HEAD)
if [[ "$BRANCH" != "main" ]]; then
	echo "error: releases are cut from 'main' (you are on '$BRANCH')." >&2
	exit 1
fi
if [[ -n "$(git status --porcelain)" ]]; then
	echo "error: working tree is not clean — commit or stash changes first." >&2
	exit 1
fi
if git rev-parse -q --verify "refs/tags/$TAG" >/dev/null 2>&1 ||
	git ls-remote --exit-code --tags origin "$TAG" >/dev/null 2>&1; then
	echo "error: tag $TAG already exists (locally or on origin)." >&2
	exit 1
fi

# --- bump version (minimal, targeted edits; perl for macOS/Linux parity) ------
# JSON manifests: the single top-level `  "version": "…"` line.
for f in src-tauri/tauri.conf.json package.json; do
	VER="$VERSION" perl -i -pe 's/^(  "version": ")[^"]*(")/${1}$ENV{VER}${2}/' "$f"
done
# Cargo.toml: the [package] version (line-anchored, so dependency `version = …`
# entries — which are inline inside `{ … }` — are untouched).
VER="$VERSION" perl -i -pe 's/^(version = ")[^"]*(")/${1}$ENV{VER}${2}/' src-tauri/Cargo.toml
# Cargo.lock: only the `version` line immediately after `name = "tliquid"`.
VER="$VERSION" perl -i -pe 'if ($s) { s/^version = "[^"]*"/version = "$ENV{VER}"/; $s = 0 } $s = 1 if /^name = "tliquid"$/' src-tauri/Cargo.lock

# Verify every file actually changed, so a reformat that breaks a pattern fails
# loudly instead of silently shipping the old version.
grep -q "^  \"version\": \"$VERSION\"" src-tauri/tauri.conf.json || { echo "error: tauri.conf.json not bumped." >&2; exit 1; }
grep -q "^  \"version\": \"$VERSION\"" package.json || { echo "error: package.json not bumped." >&2; exit 1; }
grep -q "^version = \"$VERSION\"" src-tauri/Cargo.toml || { echo "error: Cargo.toml not bumped." >&2; exit 1; }
grep -A1 '^name = "tliquid"$' src-tauri/Cargo.lock | grep -q "^version = \"$VERSION\"" || { echo "error: Cargo.lock not bumped." >&2; exit 1; }

echo "Bumped to $VERSION:"
git --no-pager diff --stat

# --- confirm, then commit / tag / push ----------------------------------------
read -r -p "Commit, tag $TAG, and push (this triggers the release build)? [y/N] " ans
if [[ ! "$ans" =~ ^[Yy]$ ]]; then
	echo "Aborted — reverting version edits."
	git checkout -- src-tauri/tauri.conf.json package.json src-tauri/Cargo.toml src-tauri/Cargo.lock
	exit 1
fi

git commit -aqm "chore(release): $VERSION"
git tag "$TAG"
git push origin "$BRANCH"
git push origin "$TAG"

cat <<EOF

✅ Pushed $TAG. release.yml is now building + signing the update.

Next:
  1. Watch the build:
       gh run watch \$(gh run list --workflow Release -L1 --json databaseId --jq '.[0].databaseId') --exit-status
  2. When the draft release appears, review and PUBLISH it:
       gh release edit $TAG --draft=false --latest
  3. Confirm the endpoint serves it:
       curl -sL https://github.com/cesarion161/TLiquid/releases/latest/download/latest.json

Installed apps older than $VERSION will then be offered the update.
EOF
