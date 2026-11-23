#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

PROFILE="${1:-release}"
if [[ "$PROFILE" == "--debug" ]]; then
    echo ">>> Building Rust binaries (debug profile)..."
    cargo build --bins
    TARGET_DIR="$ROOT_DIR/target/debug"
else
    echo ">>> Building Rust binaries (release profile)..."
    cargo build --release --bins
    TARGET_DIR="$ROOT_DIR/target/release"
fi

echo ">>> Compiling native macOS Menu Bar controller (LocalColliderMenuBar)..."
swiftc -O "$ROOT_DIR/macos/MenuBarController.swift" \
    -framework Cocoa \
    -o "$TARGET_DIR/LocalColliderMenuBar"

create_bundle() {
    local APP_DIR="$1"
    rm -rf "$APP_DIR"
    mkdir -p "$APP_DIR/Contents/MacOS"
    mkdir -p "$APP_DIR/Contents/Resources"

    cp "$ROOT_DIR/macos/Info.plist" "$APP_DIR/Contents/Info.plist"
    cp "$TARGET_DIR/LocalColliderMenuBar" "$APP_DIR/Contents/MacOS/LocalColliderMenuBar"
    cp "$TARGET_DIR/collider-engine" "$APP_DIR/Contents/MacOS/collider-engine"
    cp "$TARGET_DIR/collider-viz" "$APP_DIR/Contents/MacOS/collider-viz"
    cp "$TARGET_DIR/local-collider" "$APP_DIR/Contents/MacOS/local-collider"

    chmod +x "$APP_DIR/Contents/MacOS/LocalColliderMenuBar" \
             "$APP_DIR/Contents/MacOS/collider-engine" \
             "$APP_DIR/Contents/MacOS/collider-viz" \
             "$APP_DIR/Contents/MacOS/local-collider"

    if [[ -f "$TARGET_DIR/AppIcon.icns" ]]; then
        cp "$TARGET_DIR/AppIcon.icns" "$APP_DIR/Contents/Resources/AppIcon.icns"
    fi

    # Strip AppleDouble resource fork files on external volumes before ad-hoc signing
    find "$APP_DIR" -name "._*" -delete 2>/dev/null || true
    codesign --force --deep --sign - "$APP_DIR" 2>/dev/null || true
}

# Generate macOS .icns icon
ICONSET_DIR="$TARGET_DIR/AppIcon.iconset"
rm -rf "$ICONSET_DIR"
mkdir -p "$ICONSET_DIR"
if swift "$ROOT_DIR/macos/generate_icon.swift" "$ICONSET_DIR/icon_512x512.png" 2>/dev/null; then
    cp "$ICONSET_DIR/icon_512x512.png" "$ICONSET_DIR/icon_512x512@2x.png"
    sips -z 256 256 "$ICONSET_DIR/icon_512x512.png" --out "$ICONSET_DIR/icon_256x256.png" >/dev/null 2>&1 || true
    sips -z 128 128 "$ICONSET_DIR/icon_512x512.png" --out "$ICONSET_DIR/icon_128x128.png" >/dev/null 2>&1 || true
    sips -z 32 32   "$ICONSET_DIR/icon_512x512.png" --out "$ICONSET_DIR/icon_32x32.png" >/dev/null 2>&1 || true
    sips -z 16 16   "$ICONSET_DIR/icon_512x512.png" --out "$ICONSET_DIR/icon_16x16.png" >/dev/null 2>&1 || true
    iconutil -c icns "$ICONSET_DIR" -o "$TARGET_DIR/AppIcon.icns" 2>/dev/null || true
fi

mkdir -p "$ROOT_DIR/dist"
create_bundle "$ROOT_DIR/dist/Local Collider.app"
create_bundle "$ROOT_DIR/Local Collider.app"

echo "✓ Built macOS app bundle:"
echo "  • $ROOT_DIR/Local Collider.app"
echo "  • $ROOT_DIR/dist/Local Collider.app"
echo "Double-click 'Local Collider.app' in Finder to launch the Menu Bar guardian & open all 3 terminals!"
