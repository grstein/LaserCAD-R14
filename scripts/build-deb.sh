#!/usr/bin/env bash
set -euo pipefail

# scripts/build-deb.sh — Build a Debian .deb package for LaserCAD.
#
# Usage: ./scripts/build-deb.sh
# Run from the repository root on an x86_64 Ubuntu 24.04 host with Rust 1.88
# and dpkg-deb installed (part of the default dpkg package).
#
# Outputs: dist/lasercad_VERSION_amd64.deb

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${REPO_ROOT}"

# ---------------------------------------------------------------------------
# 0. Extract VERSION from Cargo.toml — never hardcoded
# ---------------------------------------------------------------------------
VERSION="$(grep '^version[[:space:]]*=' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')"
echo "==> VERSION=${VERSION}"

STAGING="build/deb-staging"
DIST="dist"
OUTPUT="${DIST}/lasercad_${VERSION}_amd64.deb"

# ---------------------------------------------------------------------------
# 1. Build release binary
# ---------------------------------------------------------------------------
echo "==> cargo build --release"
cargo build --release

# ---------------------------------------------------------------------------
# 2. Assemble staging tree (idempotent — always recreate from scratch)
# ---------------------------------------------------------------------------
echo "==> Assembling ${STAGING}/"
rm -rf "${STAGING}"
mkdir -p "${STAGING}/DEBIAN"
mkdir -p "${STAGING}/usr/bin"
mkdir -p "${STAGING}/usr/share/applications"
mkdir -p "${STAGING}/usr/share/icons/hicolor/256x256/apps"

# ---------------------------------------------------------------------------
# 3. Compute Depends: always libc6; append libssl if dynamically linked
# ---------------------------------------------------------------------------
DEPENDS="libc6"
if ldd target/release/lasercad 2>/dev/null | grep -q libssl; then
    DEPENDS="${DEPENDS}, libssl3 | libssl1.1"
fi

# ---------------------------------------------------------------------------
# 4. Write DEBIAN/control
#    Policy §5.1: each field at column 0; continuation lines start with space;
#    file ends with exactly one newline — no trailing blank lines.
# ---------------------------------------------------------------------------
cat > "${STAGING}/DEBIAN/control" << EOF
Package: lasercad
Version: ${VERSION}
Architecture: amd64
Maintainer: LaserCAD contributors <noreply@lasercad.invalid>
Section: graphics
Priority: optional
Description: KISS 2D CAD for laser cutting
 LaserCAD is a lightweight, open-source 2D CAD application designed
 for preparing files for laser cutting machines.
Depends: ${DEPENDS}
EOF

# ---------------------------------------------------------------------------
# 5. Binary (strip already applied via release profile strip = true)
# ---------------------------------------------------------------------------
cp target/release/lasercad "${STAGING}/usr/bin/lasercad"
chmod 0755 "${STAGING}/usr/bin/lasercad"

# ---------------------------------------------------------------------------
# 6. .desktop file (freedesktop spec; fields match LCV-085 AppDir spec)
# ---------------------------------------------------------------------------
cat > "${STAGING}/usr/share/applications/lasercad.desktop" << 'DESKTOP_EOF'
[Desktop Entry]
Name=LaserCAD
Exec=lasercad
Icon=lasercad
Type=Application
Categories=Graphics;
Terminal=false
DESKTOP_EOF

# ---------------------------------------------------------------------------
# 7. Icon — copy from assets (path matches freedesktop Icon Theme Spec)
# ---------------------------------------------------------------------------
cp assets/icon-256.png "${STAGING}/usr/share/icons/hicolor/256x256/apps/lasercad.png"

# ---------------------------------------------------------------------------
# 8. Build the .deb
# ---------------------------------------------------------------------------
echo "==> Building ${OUTPUT}"
mkdir -p "${DIST}"
# --root-owner-group: without it, dpkg-deb stamps every file with the
# builder's own uid/gid (e.g. 1000:1000) instead of root:root, which is wrong
# for files destined for /usr/bin and /usr/share on the installing machine.
# dpkg-deb itself warns and suggests this flag when run as a non-root user.
dpkg-deb --root-owner-group --build "${STAGING}" "${OUTPUT}"

echo "==> Done: ${OUTPUT}"
