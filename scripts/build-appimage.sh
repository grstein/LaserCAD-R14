#!/usr/bin/env bash
set -euo pipefail

# scripts/build-appimage.sh — Build a Linux AppImage for LaserCAD.
#
# Usage: ./scripts/build-appimage.sh
# Run from the repository root on an x86_64 Ubuntu 24.04 host with Rust 1.88
# and curl installed.  libfuse2 is NOT required on the build host because
# appimagetool is invoked with --appimage-extract-and-run.
#
# Outputs: dist/lasercad-x86_64.AppImage

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${REPO_ROOT}"

APPIMAGETOOL_URL="https://github.com/AppImage/AppImageKit/releases/download/13/appimagetool-x86_64.AppImage"
APPIMAGETOOL_CACHE="build/appimagetool-x86_64.AppImage"
APPDIR="AppDir"
DIST="dist"
OUTPUT="${DIST}/lasercad-x86_64.AppImage"

# ---------------------------------------------------------------------------
# 1. Build release binary
# ---------------------------------------------------------------------------
echo "==> cargo build --release"
cargo build --release

# ---------------------------------------------------------------------------
# 2. Ensure appimagetool is available
# ---------------------------------------------------------------------------
if command -v appimagetool > /dev/null 2>&1; then
    APPIMAGETOOL_BIN="appimagetool"
else
    if [ ! -f "${APPIMAGETOOL_CACHE}" ]; then
        echo "==> Downloading appimagetool → ${APPIMAGETOOL_CACHE}"
        mkdir -p build
        curl -fsSL --output "${APPIMAGETOOL_CACHE}" "${APPIMAGETOOL_URL}"
        chmod +x "${APPIMAGETOOL_CACHE}"
    else
        echo "==> Using cached appimagetool at ${APPIMAGETOOL_CACHE}"
    fi
    APPIMAGETOOL_BIN="${APPIMAGETOOL_CACHE}"
fi

# ---------------------------------------------------------------------------
# 3. Assemble AppDir staging tree (idempotent — always recreate from scratch)
# ---------------------------------------------------------------------------
echo "==> Assembling ${APPDIR}/"
rm -rf "${APPDIR}"
mkdir -p "${APPDIR}/usr/bin"

# AppRun
cat > "${APPDIR}/AppRun" << 'APPRUN_EOF'
#!/usr/bin/env bash
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec "${HERE}/usr/bin/lasercad" "$@"
APPRUN_EOF
chmod +x "${APPDIR}/AppRun"

# .desktop file
cat > "${APPDIR}/lasercad.desktop" << 'DESKTOP_EOF'
[Desktop Entry]
Name=LaserCAD
Exec=lasercad
Icon=lasercad
Type=Application
Categories=Graphics;
Terminal=false
DESKTOP_EOF

# Icon (copy from assets)
cp assets/icon-256.png "${APPDIR}/lasercad.png"

# Binary (strip is already applied via Cargo release profile)
cp target/release/lasercad "${APPDIR}/usr/bin/lasercad"

# ---------------------------------------------------------------------------
# 4. Pack AppImage
# ---------------------------------------------------------------------------
echo "==> Packing AppImage → ${OUTPUT}"
mkdir -p "${DIST}"
ARCH=x86_64 "${APPIMAGETOOL_BIN}" --appimage-extract-and-run "${APPDIR}" "${OUTPUT}"

echo "==> Done: ${OUTPUT}"
