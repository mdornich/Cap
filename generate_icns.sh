#!/bin/bash

# Generate macOS .icns file from PNG icons

ICONS_DIR="/Users/mitchdornich/Documents/GitHub/Cap/apps/desktop/src-tauri/icons"
ICONSET_DIR="$ICONS_DIR/Klip.iconset"

# Create iconset directory
mkdir -p "$ICONSET_DIR"

# Create various required sizes for macOS
# Using the 128x128@2x as the source (256x256)
SOURCE_ICON="$ICONS_DIR/128x128@2x.png"

# Generate all required sizes
sips -z 16 16     "$SOURCE_ICON" --out "$ICONSET_DIR/icon_16x16.png"
sips -z 32 32     "$SOURCE_ICON" --out "$ICONSET_DIR/icon_16x16@2x.png"
sips -z 32 32     "$SOURCE_ICON" --out "$ICONSET_DIR/icon_32x32.png"
sips -z 64 64     "$SOURCE_ICON" --out "$ICONSET_DIR/icon_32x32@2x.png"
sips -z 128 128   "$SOURCE_ICON" --out "$ICONSET_DIR/icon_128x128.png"
sips -z 256 256   "$SOURCE_ICON" --out "$ICONSET_DIR/icon_128x128@2x.png"
sips -z 256 256   "$SOURCE_ICON" --out "$ICONSET_DIR/icon_256x256.png"
sips -z 512 512   "$SOURCE_ICON" --out "$ICONSET_DIR/icon_256x256@2x.png"
sips -z 512 512   "$SOURCE_ICON" --out "$ICONSET_DIR/icon_512x512.png"
sips -z 1024 1024 "$SOURCE_ICON" --out "$ICONSET_DIR/icon_512x512@2x.png"

# Backup old icns
if [ -f "$ICONS_DIR/icon.icns" ]; then
    cp "$ICONS_DIR/icon.icns" "$ICONS_DIR/icons_blue_backup/icon.icns"
    echo "Backed up old icon.icns"
fi

# Create the icns file
iconutil -c icns "$ICONSET_DIR" -o "$ICONS_DIR/icon.icns"

# Also update the macos specific icon
cp "$ICONS_DIR/icon.icns" "$ICONS_DIR/macos/icon.icns"

# Clean up
rm -rf "$ICONSET_DIR"

echo "✅ Generated purple icon.icns for macOS"