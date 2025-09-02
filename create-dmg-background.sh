#!/bin/bash

# Create a new DMG background with the purple Klip icon

# Create a temporary directory
TEMP_DIR=$(mktemp -d)
echo "Working in $TEMP_DIR"

# Copy the purple icon
cp /Users/mitchdornich/Documents/GitHub/Cap/apps/desktop/src-tauri/icons/icon.png "$TEMP_DIR/klip-icon.png"

# Create a simple DMG background with purple icon
# Using ImageMagick if available, otherwise use sips
if command -v convert &> /dev/null; then
    echo "Using ImageMagick to create DMG background..."
    # Create a gradient background
    convert -size 1320x758 \
        radial-gradient:'#6b46c1'-'#4c1d95' \
        "$TEMP_DIR/background.png"
    
    # Resize icon and place it on the background
    convert "$TEMP_DIR/klip-icon.png" -resize 256x256 "$TEMP_DIR/klip-icon-resized.png"
    
    # Composite the icon onto the background (centered at top)
    convert "$TEMP_DIR/background.png" \
        "$TEMP_DIR/klip-icon-resized.png" \
        -gravity north -geometry +0+100 \
        -composite \
        "$TEMP_DIR/dmg-background.png"
else
    echo "ImageMagick not found. Creating a simple background with sips..."
    
    # Create a simple purple background using sips
    # First, we'll use the existing icon as a base
    cp /Users/mitchdornich/Documents/GitHub/Cap/apps/desktop/src-tauri/icons/icon.png "$TEMP_DIR/dmg-background.png"
    
    # Resize to DMG background dimensions
    sips -z 758 1320 "$TEMP_DIR/dmg-background.png" --out "$TEMP_DIR/dmg-background.png"
    
    echo "Note: For a better background, install ImageMagick with: brew install imagemagick"
fi

# Copy the new background to the assets folder
cp "$TEMP_DIR/dmg-background.png" /Users/mitchdornich/Documents/GitHub/Cap/apps/desktop/src-tauri/assets/dmg-background-klip.png

echo "Created new DMG background at: apps/desktop/src-tauri/assets/dmg-background-klip.png"

# Clean up
rm -rf "$TEMP_DIR"