# Abstraction Strategy for Easier Upstream Merges

## Goal
Make our customizations modular and easily reapplicable after upstream merges, reducing merge conflicts and maintenance burden.

## Current Pain Points
Every upstream merge requires manually preserving:
- Klip branding scattered across multiple files
- Auth bypass logic intertwined with core code
- Custom defaults mixed with application logic
- Workarounds embedded directly in components

## Proposed Abstractions

### 1. 🎨 Branding Configuration System
**Current State:** Branding hardcoded in 15+ locations
**Proposed Solution:** Single configuration file

#### Create `config/branding.ts`:
```typescript
export const BRANDING = {
  productName: process.env.PRODUCT_NAME || "Klip",
  companyName: "Klip",
  bundleId: "com.klip.app",
  executable: "Klip",
  icons: {
    mac: "icons/klip-icon.icns",
    windows: "icons/klip-icon.ico",
    png: "icons/klip-icon.png"
  },
  colors: {
    primary: "#YourBrandColor",
    secondary: "#YourSecondaryColor"
  },
  urls: {
    website: "https://klip.app",
    support: "https://support.klip.app"
  }
};
```

#### Update `tauri.conf.json` to use environment variables:
```json
{
  "productName": "${PRODUCT_NAME}",
  "identifier": "${BUNDLE_ID}"
}
```

#### Build script `scripts/build-branded.sh`:
```bash
#!/bin/bash
export PRODUCT_NAME="Klip"
export BUNDLE_ID="com.klip.app"
pnpm tauri build
```

### 2. 🔐 Feature Flags System
**Current State:** Auth bypass scattered through codebase
**Proposed Solution:** Centralized feature flags

#### Create `config/features.ts`:
```typescript
export const FEATURES = {
  // Core feature toggles
  requireAuth: false,           // Bypass authentication
  premiumFeatures: "unlocked",  // "locked" | "unlocked" | "trial"
  analytics: false,              // Disable telemetry
  cloudSync: false,              // Local-only mode
  
  // Recording defaults
  defaultCameraSize: 0.20,       // 20% instead of 30%
  instantModePath: "~/Movies/Klip",
  defaultRecordingMode: "instant",
  
  // UI Features
  showUpgradePrompts: false,
  showPricingPage: false,
  customWallpaper: true,
  
  // Workarounds
  captionToggleWorkaround: true, // Black screen fix
  windowSelectionFix: true       // Production build fix
};
```

#### Usage in code:
```typescript
import { FEATURES } from '@/config/features';

// Instead of commenting out auth checks:
if (FEATURES.requireAuth && !user.isPro) {
  return <UpgradePrompt />;
}

// For camera size:
const DEFAULT_CAMERA_SIZE = FEATURES.defaultCameraSize;
```

### 3. 🔧 Custom Hooks Registry
**Current State:** Workarounds embedded in components
**Proposed Solution:** Separate workaround layer

#### Create `lib/workarounds/index.ts`:
```typescript
// Registry of all our workarounds
export * from './captionToggleFix';
export * from './windowSelectionFix';
export * from './whisperMemoryFix';
```

#### Create `lib/workarounds/captionToggleFix.tsx`:
```typescript
// Documented workaround for black screen issue
export const CaptionToggle = (props) => {
  // Our custom implementation that forces re-render
  // Completely replaces Kobalte Toggle
};
```

### 4. 📁 Project Structure Changes

```
/config
  ├── branding.ts        # All branding configuration
  ├── features.ts        # Feature flags and defaults
  └── build-env.ts       # Build-time configuration

/lib
  ├── /workarounds       # All our fixes in one place
  │   ├── index.ts
  │   ├── captionToggleFix.tsx
  │   └── windowSelectionFix.rs
  └── /customizations    # Our added features
      ├── wallpaper.ts
      ├── hotkeys.ts
      └── srtExport.ts

/scripts
  ├── build-branded.sh   # Branded build script
  ├── apply-patches.sh   # Apply our patches after merge
  └── merge-upstream.sh  # Automated merge helper
```

### 5. 🔄 Git Strategy Improvements

#### Create `.gitattributes`:
```
# Always keep our versions during merge
config/branding.ts merge=ours
config/features.ts merge=ours
lib/workarounds/* merge=ours
lib/customizations/* merge=ours

# Handle differently
tauri.conf.json merge=json
package.json merge=json
```

#### Patch System
Instead of modifying core files, create patches:

```bash
# After making a core change, create a patch
git diff > patches/001-auth-bypass.patch
git diff > patches/002-camera-defaults.patch

# After upstream merge, reapply
for patch in patches/*.patch; do
  git apply $patch || echo "Patch $patch needs manual resolution"
done
```

### 6. 🤖 Automated Merge Helper

#### Create `scripts/merge-upstream.sh`:
```bash
#!/bin/bash

echo "🔄 Starting upstream merge..."

# 1. Fetch upstream
git fetch upstream

# 2. Create merge branch
git checkout -b merge-upstream-$(date +%Y%m%d)

# 3. Merge upstream
git merge upstream/main --no-commit

# 4. Restore our critical files
git checkout HEAD -- config/branding.ts
git checkout HEAD -- config/features.ts

# 5. Apply patches
./scripts/apply-patches.sh

# 6. Update version
npm version patch --no-git-tag-version

# 7. Run tests
pnpm test

echo "✅ Merge prepared. Please review and test."
```

## Implementation Priority

### Phase 1: Quick Wins (1 day)
1. Create `config/branding.ts` with all branding
2. Create `config/features.ts` with feature flags
3. Create build scripts

### Phase 2: Refactor Core (3 days)
1. Replace hardcoded branding with config references
2. Replace auth bypasses with feature flags
3. Move workarounds to separate modules

### Phase 3: Automation (1 day)
1. Create patch system
2. Create merge helper scripts
3. Document the process

## Benefits

1. **Faster Merges**: Most upstream changes won't conflict with our customizations
2. **Cleaner Code**: Our changes are clearly separated from core
3. **Easier Testing**: Can toggle features on/off for testing
4. **Better Documentation**: All customizations in one place
5. **Reduced Risk**: Less chance of losing customizations during merge
6. **Team Scalability**: New developers can understand our changes quickly

## Migration Checklist

- [ ] Create config directory structure
- [ ] Extract all branding to config
- [ ] Convert auth bypass to feature flags
- [ ] Move workarounds to lib/workarounds
- [ ] Create build scripts
- [ ] Create patch files for existing modifications
- [ ] Test branded build
- [ ] Document the new process
- [ ] Create merge helper scripts
- [ ] Test merge process with a small upstream change

## Files to Update

### High Priority (Most Conflicts)
- `tauri.conf.json` - Use environment variables
- `src/routes/auth/*` - Feature flag for auth
- `src/recording.rs` - Feature flags for defaults
- `src/routes/editor/CaptionsTab.tsx` - Move workaround

### Medium Priority
- `package.json` - Branding references
- `src/routes/editor/Header.tsx` - Branding
- `src/windows.rs` - Window titles

### Low Priority
- Icon files - Keep in separate directory
- README.md - Keep our version

## Success Metrics

- Upstream merge time reduced from hours to minutes
- Zero lost customizations during merges
- All team members can perform merges
- Automated testing catches integration issues

---

*Created: 2025-09-02*
*Purpose: Make Klip maintainable long-term as Cap evolves*