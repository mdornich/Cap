#!/bin/bash

# Klip Notarization Script
# This script notarizes the Klip app for production distribution

set -e

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${YELLOW}🚀 Klip Notarization Process${NC}"
echo ""

# Check if app exists
APP_PATH="/Users/mitchdornich/Documents/GitHub/Cap/target/release/bundle/macos/Klip.app"
if [ ! -d "$APP_PATH" ]; then
    echo -e "${RED}❌ Error: Klip.app not found at $APP_PATH${NC}"
    echo "Please build the app first with: pnpm build:tauri"
    exit 1
fi

# Verify the app is signed
echo -e "${YELLOW}🔍 Verifying app signature...${NC}"
if codesign -dv --verbose=4 "$APP_PATH" 2>&1 | grep -q "Developer ID Application: mitch dornich"; then
    echo -e "${GREEN}✅ App is properly signed${NC}"
else
    echo -e "${RED}❌ App is not properly signed${NC}"
    exit 1
fi

# Create a ZIP for notarization (notarytool prefers ZIP over DMG for apps)
echo -e "${YELLOW}📦 Creating ZIP for notarization...${NC}"
ZIP_PATH="/Users/mitchdornich/Documents/GitHub/Cap/target/release/bundle/macos/Klip.zip"
rm -f "$ZIP_PATH"
ditto -c -k --keepParent "$APP_PATH" "$ZIP_PATH"
echo -e "${GREEN}✅ ZIP created at $ZIP_PATH${NC}"

# Get credentials
echo ""
echo -e "${YELLOW}📝 Please enter your Apple credentials:${NC}"
echo "You'll need:"
echo "1. Your Apple ID email"
echo "2. An app-specific password from https://appleid.apple.com"
echo ""

read -p "Apple ID email: " APPLE_ID
read -s -p "App-specific password (will be hidden): " APP_PASSWORD
echo ""

# Store credentials in keychain for future use (optional)
echo ""
read -p "Store credentials in keychain for future use? (y/n): " -n 1 -r
echo ""
if [[ $REPLY =~ ^[Yy]$ ]]; then
    xcrun notarytool store-credentials "klip-notarization" \
        --apple-id "$APPLE_ID" \
        --password "$APP_PASSWORD" \
        --team-id "ZPTY96LTPB"
    CREDENTIALS="--keychain-profile klip-notarization"
else
    CREDENTIALS="--apple-id \"$APPLE_ID\" --password \"$APP_PASSWORD\" --team-id ZPTY96LTPB"
fi

# Submit for notarization
echo ""
echo -e "${YELLOW}📤 Submitting app for notarization...${NC}"
echo "This may take 5-10 minutes..."

if [ -n "$CREDENTIALS" ] && [[ "$CREDENTIALS" == *"keychain"* ]]; then
    SUBMISSION_ID=$(xcrun notarytool submit "$ZIP_PATH" \
        --keychain-profile klip-notarization \
        --wait 2>&1 | tee /dev/tty | grep "id:" | head -1 | awk '{print $2}')
else
    SUBMISSION_ID=$(xcrun notarytool submit "$ZIP_PATH" \
        --apple-id "$APPLE_ID" \
        --password "$APP_PASSWORD" \
        --team-id "ZPTY96LTPB" \
        --wait 2>&1 | tee /dev/tty | grep "id:" | head -1 | awk '{print $2}')
fi

# Check notarization status
echo ""
echo -e "${YELLOW}📊 Checking notarization status...${NC}"

if [ -n "$CREDENTIALS" ] && [[ "$CREDENTIALS" == *"keychain"* ]]; then
    xcrun notarytool info "$SUBMISSION_ID" --keychain-profile klip-notarization
else
    xcrun notarytool info "$SUBMISSION_ID" \
        --apple-id "$APPLE_ID" \
        --password "$APP_PASSWORD" \
        --team-id "ZPTY96LTPB"
fi

# Staple the notarization ticket to the app
echo ""
echo -e "${YELLOW}📌 Stapling notarization ticket to app...${NC}"
xcrun stapler staple "$APP_PATH"

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ Notarization complete!${NC}"
    
    # Verify notarization
    echo ""
    echo -e "${YELLOW}🔍 Verifying notarization...${NC}"
    spctl -a -vvv -t install "$APP_PATH"
    
    echo ""
    echo -e "${GREEN}🎉 Success! Your app is notarized and ready for distribution.${NC}"
    echo ""
    echo "Next steps:"
    echo "1. Create a DMG: hdiutil create -volname Klip -srcfolder $APP_PATH -ov -format UDZO Klip.dmg"
    echo "2. Distribute the app or DMG to users"
    echo "3. Users can now run the app without security warnings"
else
    echo -e "${RED}❌ Failed to staple notarization ticket${NC}"
    echo "Check the notarization log for details:"
    if [ -n "$CREDENTIALS" ] && [[ "$CREDENTIALS" == *"keychain"* ]]; then
        echo "xcrun notarytool log \"$SUBMISSION_ID\" --keychain-profile klip-notarization"
    else
        echo "xcrun notarytool log \"$SUBMISSION_ID\" --apple-id \"$APPLE_ID\" --password \"$APP_PASSWORD\" --team-id \"ZPTY96LTPB\""
    fi
    exit 1
fi

# Clean up ZIP
rm -f "$ZIP_PATH"