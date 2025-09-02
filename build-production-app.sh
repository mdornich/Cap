#!/bin/bash

# Klip Production Build Script
# This script builds a production-ready macOS app with code signing

set -e  # Exit on error

echo "🚀 Starting Klip production build..."

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if we're in the right directory
if [ ! -f "package.json" ]; then
    echo "❌ Error: package.json not found. Please run this script from the Cap repository root."
    exit 1
fi

# Step 1: Install dependencies
echo -e "${YELLOW}📦 Installing dependencies...${NC}"
pnpm install

# Step 2: Build the application
echo -e "${YELLOW}🔨 Building Klip for production...${NC}"
cd apps/desktop
pnpm build:tauri

# Step 3: Check if build was successful
if [ -f "src-tauri/target/release/bundle/dmg/Klip_0.0.1_aarch64.dmg" ] || [ -f "src-tauri/target/release/bundle/dmg/Klip_0.0.1_x64.dmg" ]; then
    echo -e "${GREEN}✅ Build successful!${NC}"
    
    # Rename DMG to simple "Klip.dmg"
    echo -e "${YELLOW}📝 Renaming DMG to Klip.dmg...${NC}"
    if [ -f "src-tauri/target/release/bundle/dmg/Klip_0.0.1_aarch64.dmg" ]; then
        mv "src-tauri/target/release/bundle/dmg/Klip_0.0.1_aarch64.dmg" "src-tauri/target/release/bundle/dmg/Klip.dmg"
    elif [ -f "src-tauri/target/release/bundle/dmg/Klip_0.0.1_x64.dmg" ]; then
        mv "src-tauri/target/release/bundle/dmg/Klip_0.0.1_x64.dmg" "src-tauri/target/release/bundle/dmg/Klip.dmg"
    fi
    
    echo ""
    echo "📁 Your installer is ready at:"
    ls -la src-tauri/target/release/bundle/dmg/Klip.dmg
    echo ""
    echo "🎯 Next steps:"
    echo "1. Test the .dmg file by opening it"
    echo "2. Drag Klip to your Applications folder"
    echo "3. Run the app and verify it works"
    echo ""
    echo "📝 Note: On first run, users may see a security prompt."
    echo "   They can right-click the app and select 'Open' to bypass this."
else
    echo "❌ Build failed. Check the error messages above."
    exit 1
fi

# Step 4: Optional - Notarize the app (requires Apple credentials)
echo -e "${YELLOW}📋 Notarization Info:${NC}"
echo "To distribute without security warnings, you'll need to notarize the app."
echo "This requires your Apple Developer credentials."
echo ""
read -p "Do you want to notarize the app now? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo "Please enter your Apple ID (email):"
    read APPLE_ID
    echo "Please enter your app-specific password (from appleid.apple.com):"
    read -s APPLE_PASSWORD
    
    # Find the DMG file
    DMG_FILE="src-tauri/target/release/bundle/dmg/Klip.dmg"
    
    echo -e "${YELLOW}📤 Submitting for notarization...${NC}"
    xcrun notarytool submit "$DMG_FILE" \
        --apple-id "$APPLE_ID" \
        --password "$APPLE_PASSWORD" \
        --team-id "ZPTY96LTPB" \
        --wait
    
    echo -e "${YELLOW}📝 Stapling notarization ticket...${NC}"
    xcrun stapler staple "$DMG_FILE"
    
    echo -e "${GREEN}✅ Notarization complete!${NC}"
    echo "Your app can now be distributed without security warnings."
else
    echo ""
    echo "Skipping notarization. The app will work but users will see a security warning on first launch."
fi

echo ""
echo -e "${GREEN}🎉 Build process complete!${NC}"