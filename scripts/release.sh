#!/usr/bin/env bash
# scripts/release.sh — create and publish a LaserCAD GitHub release.
#
# Usage: ./scripts/release.sh
#
# Prerequisites:
#   - Clean git working tree (no uncommitted changes)
#   - gh CLI authenticated (gh auth status)
#   - dist/lasercad-x86_64.AppImage  (built by scripts/build-appimage.sh)
#   - dist/lasercad_*.deb             (built by scripts/build-deb.sh)
#
# The script runs scripts/gate.sh, creates the annotated tag v<version> (version read
# from Cargo.toml), pushes it, extracts the matching [<version>] CHANGELOG block as
# release notes, and creates the GitHub release with the binary assets attached.

set -euo pipefail

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

# 3. AppImage exists
APPIMAGE_PATH=""
if compgen -G "dist/lasercad-x86_64.AppImage" &>/dev/null; then
    APPIMAGE_PATH="dist/lasercad-x86_64.AppImage"
else
    fail "dist/lasercad-x86_64.AppImage not found. Run scripts/build-appimage.sh first."
fi
ok "AppImage: ${APPIMAGE_PATH}"

# 4. .deb exists
DEB_PATH=""
if compgen -G "dist/lasercad_*.deb" &>/dev/null; then
    DEB_PATH=$(echo dist/lasercad_*.deb)
else
    fail "dist/lasercad_*.deb not found. Run scripts/build-deb.sh first."
fi
ok "Debian package: ${DEB_PATH}"

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
    "${APPIMAGE_PATH}" \
    ${DEB_PATH}
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
  [ ] Announce on the project channel / forum if applicable.
════════════════════════════════════════════════════════════════

CHECKLIST

info "Release ${RELEASE_TAG} complete."
