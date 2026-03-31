#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
SOURCE_PNG="${1:-${PROJECT_DIR}/logo.png}"
OUTPUT_ICNS="${2:-${PROJECT_DIR}/resources/macos/Pig.icns}"
WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/Pig-icon.XXXXXX")"
ICONSET_DIR="${WORK_DIR}/Pig.iconset"
MASTER_PNG="${WORK_DIR}/Pig-master.png"

cleanup() {
    rm -rf "$WORK_DIR"
}
trap cleanup EXIT

if [ ! -f "$SOURCE_PNG" ]; then
    echo "Error: SVG source not found at ${SOURCE_PNG}"
    exit 1
fi

mkdir -p "$ICONSET_DIR"
mkdir -p "$(dirname "$OUTPUT_ICNS")"

echo "Rendering macOS icon from ${SOURCE_PNG}..."
sips -s format png "$SOURCE_PNG" --out "$MASTER_PNG" >/dev/null

render_icon() {
    local size="$1"
    local name="$2"
    sips -z "$size" "$size" "$MASTER_PNG" --out "${ICONSET_DIR}/${name}" >/dev/null
}

render_icon 16 icon_16x16.png
render_icon 32 icon_16x16@2x.png
render_icon 32 icon_32x32.png
render_icon 64 icon_32x32@2x.png
render_icon 128 icon_128x128.png
render_icon 256 icon_128x128@2x.png
render_icon 256 icon_256x256.png
render_icon 512 icon_256x256@2x.png
render_icon 512 icon_512x512.png
render_icon 1024 icon_512x512@2x.png

iconutil -c icns "$ICONSET_DIR" -o "$OUTPUT_ICNS"

echo "Generated ${OUTPUT_ICNS}"
