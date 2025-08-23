# Production Build Fixes for Klip

## Issues Identified and Fixed

### 1. Window Selection Not Working in Production

**Root Cause**: `CGWindowListCopyWindowInfo` returns `nil` when Screen Recording permission isn't properly granted or when running in a hardened runtime environment.

**Fixes Applied**:
- Modified `get_on_screen_windows()` in `macos.rs` to try multiple fallback options
- Added proper error handling when window list is null
- Updated permission check to use minimal options first (more reliable in production)

### 2. Microphone/Camera Permissions Failing

**Root Cause**: Missing entitlements for production builds and async permission handling issues.

**Fixes Applied**:
- Added temporary exception entitlements for system services:
  - `com.apple.CoreMediaIO.SystemObject`
  - `com.apple.coremedia.capturesource.connections`
  - `com.apple.tccd`
- Modified `request_permission()` to wait for permission grant and return success status
- Made permission checks more robust with fallback methods

## Build Instructions

1. **Clean Build**:
   ```bash
   cd apps/desktop
   rm -rf src-tauri/target
   ```

2. **Build Production App**:
   ```bash
   pnpm build:tauri
   ```

3. **Code Sign Properly**:
   Make sure your Developer ID certificate is in the keychain:
   ```bash
   security find-identity -v -p codesigning
   ```

4. **Notarize the App** (Required for permissions to work):
   ```bash
   xcrun notarytool submit "src-tauri/target/release/bundle/dmg/Klip.dmg" \
     --apple-id "your-apple-id@email.com" \
     --password "your-app-specific-password" \
     --team-id "ZPTY96LTPB" \
     --wait
   
   xcrun stapler staple "src-tauri/target/release/bundle/dmg/Klip.dmg"
   ```

## Testing Checklist

After building, test these scenarios:

1. **First Launch**:
   - [ ] App requests Screen Recording permission
   - [ ] App requests Microphone permission
   - [ ] App requests Camera permission
   - [ ] All permissions show in System Settings

2. **Window Selection**:
   - [ ] Can see list of windows in dropdown
   - [ ] Can select individual windows
   - [ ] Selected window is captured correctly

3. **Audio/Video**:
   - [ ] Microphone input works
   - [ ] Camera feed displays
   - [ ] Record button is enabled when permissions granted

## Additional Notes

- The app MUST be notarized for permissions to work correctly in production
- Users may need to restart the app after granting permissions
- On first run, users should grant all permissions before trying to record
- If window list is empty, check Screen Recording permission in System Settings

## Debugging Commands

Check app entitlements:
```bash
codesign -d --entitlements - /Applications/Klip.app
```

Check app signature:
```bash
codesign -dv --verbose=4 /Applications/Klip.app
```

Verify notarization:
```bash
spctl -a -vvv -t install /Applications/Klip.app
```