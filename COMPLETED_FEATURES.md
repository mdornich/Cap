# Completed Features

This document tracks successfully implemented features for Cap/Klip, organized by completion date.

## 2025-08-29

### ✅ Black Screen Fix for Caption Toggle Components
**Requested by:** @mitchdornich | **Completed:** 2025-08-29
**Implementation Time:** 3 hours (after multiple debugging sessions)

Successfully fixed a persistent issue where toggling caption settings in CaptionsTab caused the video preview to go black.

**Problem Identified:**
- The Kobalte Toggle/Switch components caused the GPU rendering pipeline to get stuck when used in CaptionsTab
- Issue was specific to CaptionsTab - same toggles worked fine in other configuration tabs
- WebSocket remained connected but frames stopped rendering

**Solution Implemented:**
- Created custom `CaptionToggle` component that forces frame re-render after state changes
- Maintains full accessibility support and visual parity with original Toggle
- Applied to all 5 toggle instances in CaptionsTab

**Technical Details:**
- Custom component triggers `renderFrameEvent.emit()` 100ms after toggle click
- This forces the rendering pipeline to restart and resume frame display
- Workaround avoids the underlying Kobalte component issue

---

## 2025-08-26

### ✅ Quick Caption Toggle & Transcription Fixes
**Requested by:** @mitchdornich | **Completed:** 2025-08-26
**Implementation Time:** 1 day

Successfully added a quick-access caption toggle button to the editor toolbar and fixed critical transcription issues.

**Features Implemented:**
- Quick caption toggle button in editor toolbar
- Keyboard shortcut 'C' for instant caption visibility
- Visual feedback (button highlights when enabled)
- Smart state management between global store and local state
- Fixed transcription crash caused by unsafe memory operations
- Corrected microphone audio detection in metadata
- Improved audio stream prioritization (microphone over system audio)

**Technical Details:**
- Fixed unsafe `std::ptr::read` operation causing double-free
- Updated metadata path lookup from `segment["audio"]` to `segment["mic"]`
- Added fallback for legacy "audio" field
- Simplified WhisperContext creation to avoid ownership issues
- Added comprehensive logging for audio source selection

---

## 2025-08-25

### ✅ Command+R Recording Hotkey
**Requested by:** @mitchdornich | **Completed:** 2025-08-25
**Implementation Time:** 2 hours

Implemented global hotkey support for starting/stopping recordings.

**Features Implemented:**
- Command+R to start recording
- Command+S to stop recording
- System-wide hotkey registration
- Conflict prevention with browser shortcuts

---

*For upcoming features, see [FUTURE_FEATURES.md](./FUTURE_FEATURES.md)*