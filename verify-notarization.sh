#!/bin/bash

# Verify Klip Notarization

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

APP_PATH="/Users/mitchdornich/Documents/GitHub/Cap/target/release/bundle/macos/Klip.app"

echo -e "${YELLOW}🔍 Verifying Klip Notarization Status${NC}"
echo ""

# Check if app is notarized
echo "Checking notarization status..."
if spctl -a -vvv -t install "$APP_PATH" 2>&1 | grep -q "accepted"; then
    echo -e "${GREEN}✅ App is properly notarized${NC}"
    spctl -a -vvv -t install "$APP_PATH"
else
    echo -e "${RED}❌ App is not notarized${NC}"
    spctl -a -vvv -t install "$APP_PATH"
    exit 1
fi

echo ""
echo "Checking stapled ticket..."
if stapler validate "$APP_PATH" 2>&1 | grep -q "validate worked"; then
    echo -e "${GREEN}✅ Notarization ticket is properly stapled${NC}"
else
    echo -e "${YELLOW}⚠️  Ticket may not be stapled (this is okay, online check will work)${NC}"
fi

echo ""
echo -e "${GREEN}🎉 Notarization verification complete!${NC}"
echo ""
echo "Next steps:"
echo "1. Create DMG: ./create-dmg.sh"
echo "2. Test the app by opening it"
echo "3. Verify permissions work (camera, microphone, screen recording)"