#!/bin/bash

# Create a professional DMG installer for Klip

echo "Creating Klip installer DMG..."

# Clean up any existing temp directories
rm -rf /tmp/klip-dmg
mkdir -p /tmp/klip-dmg

# Copy the app
cp -R target/release/bundle/macos/Klip.app /tmp/klip-dmg/

# Create a symbolic link to Applications
ln -s /Applications /tmp/klip-dmg/Applications

# Create the DMG
hdiutil create -volname "Klip Installer" \
    -srcfolder /tmp/klip-dmg \
    -ov \
    -format UDZO \
    ~/Desktop/Klip-Installer.dmg

# Clean up
rm -rf /tmp/klip-dmg

echo "✅ Installer created at ~/Desktop/Klip-Installer.dmg"
echo ""
echo "Instructions for your colleague:"
echo "1. Open Klip-Installer.dmg"
echo "2. Drag Klip to the Applications folder"
echo "3. Eject the installer"
echo "4. Launch Klip from Applications"
echo "5. If macOS shows a security warning:"
echo "   - Go to System Settings > Privacy & Security"
echo "   - Click 'Open Anyway' next to the Klip message"
echo "   - Or right-click Klip in Applications and select 'Open'"