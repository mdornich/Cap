#!/bin/bash

# Create DMG for Klip

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

APP_PATH="/Users/mitchdornich/Documents/GitHub/Cap/target/release/bundle/macos/Klip.app"
DMG_PATH="/Users/mitchdornich/Documents/GitHub/Cap/Klip.dmg"

echo -e "${YELLOW}📦 Creating DMG for Klip${NC}"

# Remove old DMG if exists
if [ -f "$DMG_PATH" ]; then
    echo "Removing old DMG..."
    rm -f "$DMG_PATH"
fi

# Create DMG
echo "Creating new DMG..."
hdiutil create -volname "Klip" \
    -srcfolder "$APP_PATH" \
    -ov \
    -format UDZO \
    "$DMG_PATH"

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ DMG created successfully${NC}"
    echo "Location: $DMG_PATH"
    echo ""
    
    # Get file size
    SIZE=$(du -h "$DMG_PATH" | cut -f1)
    echo "Size: $SIZE"
    
    # Verify DMG is notarized
    echo ""
    echo "Verifying DMG notarization..."
    if spctl -a -vvv -t open --context context:primary-signature "$DMG_PATH" 2>&1 | grep -q "accepted"; then
        echo -e "${GREEN}✅ DMG is properly notarized${NC}"
    else
        echo -e "${YELLOW}⚠️  DMG notarization not verified (app inside should still work)${NC}"
    fi
    
    echo ""
    echo -e "${GREEN}🎉 DMG is ready for distribution!${NC}"
    echo ""
    echo "To install:"
    echo "1. Open $DMG_PATH"
    echo "2. Drag Klip to Applications folder"
    echo "3. Run Klip from Applications"
else
    echo -e "${RED}❌ Failed to create DMG${NC}"
    exit 1
fi