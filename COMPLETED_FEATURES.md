# Completed Features

This document tracks successfully implemented features for Cap/Klip, organized by completion date.

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