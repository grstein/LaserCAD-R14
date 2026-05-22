#!/usr/bin/env bash
set -euo pipefail

# generate-icons.sh — Regenerate all PNG icon sizes from the SVG source.
#
# Dependency: librsvg2-bin  (sudo apt install librsvg2-bin)
# Run from the repository root:  ./scripts/generate-icons.sh
#
# This script is NOT called by scripts/build-appimage.sh.  The committed PNGs
# are the authoritative build artifacts; this script is the reproducibility
# record so any contributor can regenerate them after editing lasercad.svg.

# Change to the repository root so all paths are relative and predictable.
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/.."

echo "Generating icons from assets/icons/lasercad.svg ..."

rsvg-convert -w 16  -h 16  assets/icons/lasercad.svg -o assets/icons/lasercad-16.png
rsvg-convert -w 32  -h 32  assets/icons/lasercad.svg -o assets/icons/lasercad-32.png
rsvg-convert -w 64  -h 64  assets/icons/lasercad.svg -o assets/icons/lasercad-64.png
rsvg-convert -w 128 -h 128 assets/icons/lasercad.svg -o assets/icons/lasercad-128.png
rsvg-convert -w 256 -h 256 assets/icons/lasercad.svg -o assets/icons/lasercad-256.png

cp assets/icons/lasercad-256.png assets/icon-256.png

echo "Done.  Regenerated:"
for size in 16 32 64 128 256; do
    echo "  assets/icons/lasercad-${size}.png"
done
echo "  assets/icon-256.png"
