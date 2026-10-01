#!/usr/bin/env bash
set -euo pipefail

# scripts/build-dmg.sh — Build LaserCAD.app and wrap it in a .dmg (LCV-201).
#
# Usage: ./scripts/build-dmg.sh
# Run on an Apple Silicon (arm64) macOS 12.0+ host with Rust 1.98 (via
# rust-toolchain.toml) and Xcode Command Line Tools (xcode-select --install).
# Intel Macs build from source. No third-party tools are required.
#
# Uses target/release/lasercad when it already exists (CI downloads it from the
# build job); otherwise runs `cargo build --release` first.
#
# Output: dist/lasercad-<version>-macos-aarch64.dmg holding LaserCAD.app
# (ad-hoc signed, not notarized), an Applications link and FIRST-RUN.txt.

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${REPO_ROOT}"

# ---------------------------------------------------------------------------
# 0. Refuse any host that is not Apple Silicon macOS — before touching disk
# ---------------------------------------------------------------------------
if [[ "$(uname -s)" != "Darwin" || "$(uname -m)" != "arm64" ]]; then
    echo "ERROR: build-dmg.sh needs an Apple Silicon (arm64) macOS host; got $(uname -s) $(uname -m)." >&2
    exit 1
fi

# ---------------------------------------------------------------------------
# 1. Extract VERSION from Cargo.toml at runtime — never hardcoded
# ---------------------------------------------------------------------------
VERSION="$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)"
echo "==> VERSION=${VERSION}"

# ---------------------------------------------------------------------------
# 2. Path constants
# ---------------------------------------------------------------------------
APP_DIR="build/LaserCAD.app"
ICONSET_DIR="build/lasercad.iconset"
DMG_STAGING="build/dmg-staging"
DIST="dist"
OUTPUT="${DIST}/lasercad-${VERSION}-macos-aarch64.dmg"

# ---------------------------------------------------------------------------
# 3. Build the release binary unless CI already provided it
# ---------------------------------------------------------------------------
if [[ ! -f target/release/lasercad ]]; then
    echo "==> cargo build --release"
    cargo build --release
fi

# ---------------------------------------------------------------------------
# 4. Assemble .app bundle (idempotent — always recreate from scratch)
# ---------------------------------------------------------------------------
echo "==> Assembling ${APP_DIR}/"
rm -rf "${APP_DIR}" "${ICONSET_DIR}" "${DMG_STAGING}"
mkdir -p "${APP_DIR}/Contents/MacOS"
mkdir -p "${APP_DIR}/Contents/Resources"

# ---------------------------------------------------------------------------
# 5. Copy and strip the release binary into the bundle
# ---------------------------------------------------------------------------
echo "==> Copying binary"
cp "target/release/lasercad" "${APP_DIR}/Contents/MacOS/lasercad"
chmod 0755 "${APP_DIR}/Contents/MacOS/lasercad"
# strip debug info if strip(1) is available (Xcode CLT provides it)
strip "${APP_DIR}/Contents/MacOS/lasercad" 2>/dev/null || true

# ---------------------------------------------------------------------------
# 6. Generate .icns from assets/icon-256.png using sips + iconutil; iconset
#    file names follow Apple's icon_<size>x<size>[@2x].png convention
#    (both ship with Xcode Command Line Tools — no additional tooling required)
# ---------------------------------------------------------------------------
echo "==> Generating .icns"
mkdir -p "${ICONSET_DIR}"

sips -z 16  16  assets/icon-256.png  --out "${ICONSET_DIR}/icon_16x16.png"   > /dev/null
sips -z 32  32  assets/icon-256.png  --out "${ICONSET_DIR}/icon_32x32.png"   > /dev/null
sips -z 32  32  assets/icon-256.png  --out "${ICONSET_DIR}/icon_16x16@2x.png" > /dev/null
sips -z 64  64  assets/icon-256.png  --out "${ICONSET_DIR}/icon_32x32@2x.png" > /dev/null
sips -z 128 128 assets/icon-256.png  --out "${ICONSET_DIR}/icon_128x128.png" > /dev/null
sips -z 256 256 assets/icon-256.png  --out "${ICONSET_DIR}/icon_128x128@2x.png" > /dev/null
sips -z 256 256 assets/icon-256.png  --out "${ICONSET_DIR}/icon_256x256.png" > /dev/null

iconutil -c icns "${ICONSET_DIR}" -o "build/lasercad.icns"
cp "build/lasercad.icns" "${APP_DIR}/Contents/Resources/lasercad.icns"

# ---------------------------------------------------------------------------
# 7. Write Info.plist
#    NSHighResolutionCapable must be a Boolean <true/>, not a string.
# ---------------------------------------------------------------------------
echo "==> Writing Info.plist"
cat > "${APP_DIR}/Contents/Info.plist" << PLIST_EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>lasercad</string>
    <key>CFBundleIdentifier</key>
    <string>io.lasercad.lasercad</string>
    <key>CFBundleName</key>
    <string>LaserCAD</string>
    <key>CFBundleDisplayName</key>
    <string>LaserCAD</string>
    <key>CFBundleVersion</key>
    <string>${VERSION}</string>
    <key>CFBundleShortVersionString</key>
    <string>${VERSION}</string>
    <key>CFBundleIconFile</key>
    <string>lasercad</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>LSMinimumSystemVersion</key>
    <string>12.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
PLIST_EOF

# ---------------------------------------------------------------------------
# 8. Ad-hoc signature: no certificate, but Apple Silicon refuses to launch an
#    unsigned binary. Gatekeeper still warns (see docs/install.md).
# ---------------------------------------------------------------------------
echo "==> Ad-hoc signing ${APP_DIR}"
codesign --force --deep -s - "${APP_DIR}"

# ---------------------------------------------------------------------------
# 9. Create dmg-staging directory:
#    - LaserCAD.app (copy of the signed bundle)
#    - Applications -> /Applications  (drag-to-install symlink)
#    - FIRST-RUN.txt (Gatekeeper first-run steps)
# ---------------------------------------------------------------------------
echo "==> Preparing DMG staging area"
mkdir -p "${DMG_STAGING}"
cp -R "${APP_DIR}" "${DMG_STAGING}/LaserCAD.app"
ln -s /Applications "${DMG_STAGING}/Applications"
cp assets/FIRST-RUN.txt "${DMG_STAGING}/FIRST-RUN.txt"

# ---------------------------------------------------------------------------
# 10. Create the .dmg with hdiutil
# ---------------------------------------------------------------------------
echo "==> Creating ${OUTPUT}"
mkdir -p "${DIST}"
# Remove any stale image from a previous run (hdiutil -ov handles this but
# also resets the format; explicit removal keeps the message clean).
rm -f "${OUTPUT}"
hdiutil create \
    -volname "LaserCAD" \
    -srcfolder "${DMG_STAGING}" \
    -ov \
    -format UDZO \
    -o "${OUTPUT}"

echo "==> Done: ${OUTPUT}"
