#!/bin/bash

# Klip Notarization Script
# This script notarizes the Klip app for distribution without security warnings

set -e  # Exit on error

echo "🔐 Klip Notarization Script"
echo "=========================="
echo ""

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Configuration
TEAM_ID="ZPTY96LTPB"
BUNDLE_ID="com.klip.app"

# Check if we're in the right directory
if [ ! -f "apps/desktop/src-tauri/tauri.conf.json" ]; then
    echo -e "${RED}❌ Error: Not in the Cap repository root. Please run from the repository root.${NC}"
    exit 1
fi

# Find the DMG file
DMG_PATH=""
if [ -f "apps/desktop/src-tauri/target/release/bundle/dmg/Klip_0.0.1_aarch64.dmg" ]; then
    DMG_PATH="apps/desktop/src-tauri/target/release/bundle/dmg/Klip_0.0.1_aarch64.dmg"
elif [ -f "apps/desktop/src-tauri/target/release/bundle/dmg/Klip_0.0.1_x64.dmg" ]; then
    DMG_PATH="apps/desktop/src-tauri/target/release/bundle/dmg/Klip_0.0.1_x64.dmg"
elif [ -f "apps/desktop/src-tauri/target/release/bundle/dmg/Klip.dmg" ]; then
    DMG_PATH="apps/desktop/src-tauri/target/release/bundle/dmg/Klip.dmg"
else
    echo -e "${RED}❌ Error: DMG file not found. Please build the app first.${NC}"
    echo "Run: cd apps/desktop && pnpm build:tauri"
    exit 1
fi

echo -e "${GREEN}✓ Found DMG at: $DMG_PATH${NC}"
echo ""

# Check if the app is properly signed
echo -e "${YELLOW}🔍 Checking code signature...${NC}"
# Mount the DMG temporarily to check signature
MOUNT_POINT=$(mktemp -d)
hdiutil attach "$DMG_PATH" -nobrowse -mountpoint "$MOUNT_POINT" > /dev/null 2>&1

if codesign -dv --verbose=4 "$MOUNT_POINT/Klip.app" 2>&1 | grep -q "Developer ID Application: mitch dornich"; then
    echo -e "${GREEN}✓ App is properly signed${NC}"
else
    echo -e "${RED}❌ App is not properly signed with Developer ID${NC}"
    hdiutil detach "$MOUNT_POINT" > /dev/null 2>&1
    exit 1
fi

hdiutil detach "$MOUNT_POINT" > /dev/null 2>&1
rm -rf "$MOUNT_POINT"

# Get Apple ID credentials
echo ""
echo -e "${YELLOW}📋 Apple Developer Credentials Required${NC}"
echo "You need an app-specific password from https://appleid.apple.com"
echo ""

# Check if credentials are in environment variables
if [ -z "$APPLE_ID" ]; then
    read -p "Enter your Apple ID email: " APPLE_ID
else
    echo "Using Apple ID from environment: $APPLE_ID"
fi

if [ -z "$APPLE_APP_PASSWORD" ]; then
    echo "Enter your app-specific password (will be hidden):"
    read -s APPLE_APP_PASSWORD
    echo ""
else
    echo "Using app password from environment variable"
fi

# Store credentials in keychain for future use (optional)
echo ""
read -p "Do you want to store credentials in keychain for future use? (y/n) " -n 1 -r
echo ""
if [[ $REPLY =~ ^[Yy]$ ]]; then
    xcrun notarytool store-credentials "klip-notarize" \
        --apple-id "$APPLE_ID" \
        --password "$APPLE_APP_PASSWORD" \
        --team-id "$TEAM_ID"
    echo -e "${GREEN}✓ Credentials stored in keychain as 'klip-notarize'${NC}"
    NOTARIZE_ARGS="--keychain-profile klip-notarize"
else
    NOTARIZE_ARGS="--apple-id \"$APPLE_ID\" --password \"$APPLE_APP_PASSWORD\" --team-id \"$TEAM_ID\""
fi

# Submit for notarization
echo ""
echo -e "${YELLOW}📤 Submitting DMG for notarization...${NC}"
echo "This may take several minutes..."

SUBMISSION_ID=$(eval xcrun notarytool submit "$DMG_PATH" $NOTARIZE_ARGS --wait 2>&1 | tee /dev/tty | grep -o 'id: [a-f0-9-]*' | head -1 | awk '{print $2}')

if [ -z "$SUBMISSION_ID" ]; then
    echo -e "${RED}❌ Failed to get submission ID${NC}"
    exit 1
fi

echo ""
echo -e "${YELLOW}📊 Checking notarization status...${NC}"

# Get detailed log
eval xcrun notarytool log "$SUBMISSION_ID" $NOTARIZE_ARGS

# Check final status
STATUS=$(eval xcrun notarytool info "$SUBMISSION_ID" $NOTARIZE_ARGS 2>&1 | grep "status:" | awk '{print $2}')

if [ "$STATUS" = "Accepted" ]; then
    echo -e "${GREEN}✅ Notarization successful!${NC}"
    
    # Staple the ticket to the DMG
    echo -e "${YELLOW}📎 Stapling notarization ticket to DMG...${NC}"
    xcrun stapler staple "$DMG_PATH"
    
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✅ Notarization ticket stapled successfully!${NC}"
    else
        echo -e "${RED}⚠️  Warning: Failed to staple ticket, but notarization was successful${NC}"
    fi
    
    # Verify the notarization
    echo ""
    echo -e "${YELLOW}🔍 Verifying notarization...${NC}"
    spctl -a -vvv -t install "$DMG_PATH" 2>&1
    
    echo ""
    echo -e "${GREEN}🎉 Notarization complete!${NC}"
    echo ""
    echo "Your notarized DMG is ready at:"
    echo "  $DMG_PATH"
    echo ""
    echo "Users can now install Klip without security warnings!"
    
    # Offer to rename to simple name
    echo ""
    read -p "Do you want to rename the DMG to 'Klip-Notarized.dmg'? (y/n) " -n 1 -r
    echo ""
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        NEW_PATH="${DMG_PATH%/*}/Klip-Notarized.dmg"
        mv "$DMG_PATH" "$NEW_PATH"
        echo -e "${GREEN}✓ Renamed to: $NEW_PATH${NC}"
    fi
else
    echo -e "${RED}❌ Notarization failed with status: $STATUS${NC}"
    echo "Check the log above for details"
    exit 1
fi