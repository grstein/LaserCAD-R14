#!/usr/bin/env bash
# scripts/release.sh — create and publish a LaserCAD GitHub release.
#
# Usage: ./scripts/release.sh                 # gate, tag, push, publish
#        ./scripts/release.sh --list-assets   # print the assets it would attach, then exit 0
#
# Prerequisites:
#   - Clean git working tree (no uncommitted changes)
#   - gh CLI authenticated (gh auth status)
#   - dist/lasercad-x86_64.AppImage  (built by scripts/build-appimage.sh)  — required
#   - dist/lasercad_*.deb             (built by scripts/build-deb.sh)       — required
#   - dist/lasercad-<version>-windows-x86_64.zip (scripts/build-zip.ps1)   — optional
#   - dist/lasercad-<version>-macos-aarch64.dmg  (scripts/build-dmg.sh)    — optional
#
# An absent optional artifact is reported as `missing: <file>` and the release
# goes ahead with what exists (LCV-201).
#
# The script runs scripts/gate.sh, creates the annotated tag v<version> (version read
# from Cargo.toml), pushes it, extracts the matching [<version>] CHANGELOG block as
# release notes, and creates the GitHub release with the binary assets attached.

set -euo pipefail

LIST_ONLY=0
case "${1:-}" in
    "") ;;
    --list-assets) LIST_ONLY=1 ;;
    *) echo "usage: $0 [--list-assets]" >&2; exit 2 ;;
esac

VERSION=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -1)
[[ -n "${VERSION}" ]] || { echo "cannot read version from Cargo.toml" >&2; exit 1; }
RELEASE_TAG="v${VERSION}"
RELEASE_TITLE="LaserCAD v${VERSION}"
NOTES_FILE="/tmp/lasercad-release-notes.txt"

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------
info()  { printf '\033[1;34m[INFO]\033[0m  %s\n' "$*"; }
ok()    { printf '\033[1;32m[ OK ]\033[0m  %s\n' "$*"; }
fail()  { printf '\033[1;31m[FAIL]\033[0m  %s\n' "$*" >&2; exit 1; }

# ---------------------------------------------------------------------------
# Release assets — one list shared by --list-assets and the real run.
# ASSETS: files that exist; MISSING_REQUIRED / MISSING_OPTIONAL: absent ones.
# `${A[@]+"${A[@]}"}` expands an empty array safely under `set -u` on bash < 4.4
# (macOS ships bash 3.2, and the macOS CI leg runs --list-assets).
# ---------------------------------------------------------------------------
ASSETS=()
MISSING_REQUIRED=()
MISSING_OPTIONAL=()

APPIMAGE_PATH="dist/lasercad-x86_64.AppImage"
if [[ -f "${APPIMAGE_PATH}" ]]; then ASSETS+=("${APPIMAGE_PATH}"); else MISSING_REQUIRED+=("${APPIMAGE_PATH}"); fi

shopt -s nullglob
DEB_PATHS=(dist/lasercad_*.deb)
shopt -u nullglob
if (( ${#DEB_PATHS[@]} > 0 )); then ASSETS+=("${DEB_PATHS[@]}"); else MISSING_REQUIRED+=("dist/lasercad_*.deb"); fi

for optional in "dist/lasercad-${VERSION}-windows-x86_64.zip" "dist/lasercad-${VERSION}-macos-aarch64.dmg"; do
    if [[ -f "${optional}" ]]; then ASSETS+=("${optional}"); else MISSING_OPTIONAL+=("${optional}"); fi
done

if (( LIST_ONLY )); then
    for a in ${ASSETS[@]+"${ASSETS[@]}"}; do printf '%s\n' "${a}"; done
    for m in ${MISSING_REQUIRED[@]+"${MISSING_REQUIRED[@]}"} ${MISSING_OPTIONAL[@]+"${MISSING_OPTIONAL[@]}"}; do printf 'missing: %s\n' "${m}"; done
    exit 0
fi

# ---------------------------------------------------------------------------
# Precondition checks
# ---------------------------------------------------------------------------
info "Checking preconditions…"

# 1. Clean working tree
if ! git diff --quiet HEAD; then
    fail "Working tree has uncommitted changes. Commit or stash before releasing."
fi
ok "Git working tree is clean."

# 2. gh CLI present
if ! command -v gh &>/dev/null; then
    fail "'gh' CLI not found. Install it from https://cli.github.com/ and authenticate."
fi
ok "gh CLI found: $(gh --version | head -1)."

# 3. Linux artifacts are required; Windows and macOS ones are attached when present.
if (( ${#MISSING_REQUIRED[@]} > 0 )); then
    fail "Required artifact(s) missing: ${MISSING_REQUIRED[*]}. Run scripts/build-appimage.sh and scripts/build-deb.sh first."
fi
for m in ${MISSING_OPTIONAL[@]+"${MISSING_OPTIONAL[@]}"}; do
    printf 'missing: %s\n' "${m}" >&2
done
ok "Assets: ${ASSETS[*]}"

# ---------------------------------------------------------------------------
# Build gate: fmt / clippy / test
# ---------------------------------------------------------------------------
info "Running scripts/gate.sh…"
scripts/gate.sh
ok "Gate green."

# ---------------------------------------------------------------------------
# Create annotated tag
# ---------------------------------------------------------------------------
info "Creating annotated tag ${RELEASE_TAG}…"
if git rev-parse "${RELEASE_TAG}" &>/dev/null; then
    fail "Tag ${RELEASE_TAG} already exists. Delete it first: git tag -d ${RELEASE_TAG}"
fi
git tag -a "${RELEASE_TAG}" -m "Release ${RELEASE_TAG#v}"
ok "Tag ${RELEASE_TAG} created."

# ---------------------------------------------------------------------------
# Push tag
# ---------------------------------------------------------------------------
info "Pushing tag to origin…"
git push origin "${RELEASE_TAG}"
ok "Tag pushed."

# ---------------------------------------------------------------------------
# Extract the [<version>] block from CHANGELOG as release notes
# ---------------------------------------------------------------------------
info "Extracting release notes from CHANGELOG.md…"
# Print lines between the first "## [<version>]" heading and the next "## [" heading.
awk -v h="## [${VERSION}]" 'index($0, h) == 1 {found=1; next} found && /^## \[/{exit} found{print}' CHANGELOG.md \
    | sed '/^[[:space:]]*$/d' \
    > "${NOTES_FILE}"

if [[ ! -s "${NOTES_FILE}" ]]; then
    fail "Release notes extraction produced an empty file. Check CHANGELOG.md."
fi
ok "Release notes written to ${NOTES_FILE}."

# ---------------------------------------------------------------------------
# Create GitHub release
# ---------------------------------------------------------------------------
info "Creating GitHub release ${RELEASE_TAG}…"
gh release create "${RELEASE_TAG}" \
    --title "${RELEASE_TITLE}" \
    --notes-file "${NOTES_FILE}" \
    "${ASSETS[@]}"
ok "GitHub release ${RELEASE_TAG} created."

# ---------------------------------------------------------------------------
# Post-release checklist
# ---------------------------------------------------------------------------
cat <<'CHECKLIST'

════════════════════════════════════════════════════════════════
  Post-release checklist
════════════════════════════════════════════════════════════════
  [ ] Verify the release page on GitHub shows the correct notes.
  [ ] Download and smoke-test the AppImage on a clean Linux VM.
  [ ] Install and smoke-test the .deb on Ubuntu/Debian.
  [ ] If attached, smoke-test the Windows .zip and the macOS .dmg (docs/install.md).
  [ ] Announce on the project channel / forum if applicable.
════════════════════════════════════════════════════════════════

CHECKLIST

info "Release ${RELEASE_TAG} complete."
