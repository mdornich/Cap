# Caption and Export System Architecture

## Overview
This document describes the caption and export system architecture in the Cap/Klip desktop application, based on extensive analysis conducted on 2025-08-28. The app uses Tauri with a Rust backend and TypeScript/SolidJS frontend.

## Key Discoveries

### 1. Caption Data Flow

#### Storage Locations
- **In-memory**: Captions are stored in the `project.captions` object during editing
- **Persistent storage**: `~/Library/Application Support/com.klip.app/captions/{video-id}/captions.json`
- **SRT export**: `~/Library/Application Support/com.klip.app/captions/{video-id}/captions.srt`

#### Critical Finding: Caption Persistence Gap
**Problem discovered**: Captions were being created and edited in memory but never persisted to disk. This caused exports to fail because `export_captions_srt` expects to find `captions.json` on disk.

**Solution implemented**: Added automatic saving via `saveCaptionsToDisk()` function that triggers after:
- Caption generation
- Segment updates
- Segment deletion  
- Segment addition
- Settings changes

### 2. File Structure

#### Frontend (TypeScript/SolidJS)
```
apps/desktop/src/
├── routes/editor/
│   ├── ExportDialog.tsx       # Main export UI, handles video & SRT export
│   └── CaptionsTab.tsx        # Caption editing UI, now saves to disk
├── utils/
│   ├── tauri.ts              # Tauri command bindings
│   └── export.ts             # Export utilities
```

#### Backend (Rust)
```
apps/desktop/src-tauri/src/
├── lib.rs                    # Main app logic, includes copy_file_to_path
├── captions.rs              # Caption generation, saving, SRT export
└── export.rs                # Video export logic
```

### 3. Tauri Commands (IPC Bridge)

Key commands for caption/export workflow:

```typescript
// Caption Management
commands.generateCaptions(videoId: string, modelPath?: string)
commands.saveCaptions(videoId: string, captions: CaptionData)
commands.loadCaptions(videoId: string)
commands.exportCaptionsSrt(videoId: string) -> PathBuf

// File Operations
commands.copyFileToPath(src: string, dst: string)
commands.exportVideo(projectPath: string, progress: Channel, settings: ExportSettings)
```

### 4. Export Settings Structure

```typescript
interface ExportSettings {
  format: string;           // e.g., "mp4"
  resolution: string;        // e.g., "1080p"
  framerate: number;         // e.g., 30
  captionExport?: "burn" | "srt" | "none";
  // ... other settings
}
```

### 5. Caption Data Structure

```typescript
interface CaptionData {
  segments: CaptionSegment[];
  settings: CaptionSettings | null;
}

interface CaptionSegment {
  start: number;  // Start time in seconds
  end: number;    // End time in seconds
  text: string;   // Caption text
}

interface CaptionSettings {
  font: string;
  fontSize: number;
  color: string;
  // ... other style settings
}
```

## Critical Implementation Details

### 1. Video ID Extraction Pattern
The video ID is consistently extracted from the cap file path:
```typescript
const pathParts = editorInstance.path.split('/');
const capFileName = pathParts[pathParts.length - 1];
const videoId = capFileName.replace('.cap', '');
```

### 2. File Copy Validation Issue
**Problem**: The `copy_file_to_path` function was validating all files as MP4s, causing SRT copy to fail.

**Solution**: Added file type detection:
```rust
let is_srt = src.ends_with(".srt") || dst.ends_with(".srt");
// Skip MP4 validation for SRT files
if !is_screenshot && !is_gif && !is_srt {
    // MP4 validation code
}
```

### 3. SRT Export Flow

1. User selects "SRT" in ExportDialog caption export dropdown
2. Video exports normally to user-chosen location
3. After video export:
   - Extract video ID from project path
   - Call `exportCaptionsSrt(videoId)` to generate SRT in app data
   - Copy SRT file to same directory as exported video
   - Rename to match video filename (e.g., `video.mp4` → `video.srt`)

## Common Pitfalls & Solutions

### Pitfall 1: Captions Not Found During Export
**Symptom**: "No captions found" error when exporting SRT
**Cause**: Captions only exist in memory, not on disk
**Solution**: Ensure `saveCaptionsToDisk()` is called after any caption modification

### Pitfall 2: SRT File in Wrong Location
**Symptom**: SRT file created but in app's internal directory
**Cause**: Missing or failing copy operation
**Solution**: Use `copyFileToPath` to copy SRT to video location

### Pitfall 3: Copy Operation Fails Silently
**Symptom**: No error but SRT not in expected location
**Cause**: `copy_file_to_path` validates all files as MP4s
**Solution**: Add file type detection to skip validation for SRT files

### Pitfall 4: Assuming Commands Exist
**Symptom**: "commands.functionName is not a function" errors
**Cause**: Not all Rust functions are exposed as Tauri commands
**Solution**: Check `lib.rs` for `#[tauri::command]` decorators and command registration

## Debugging Tips

### 1. Enable Logging
Add console.log statements at key points:
```typescript
console.log('Caption export setting:', settings.captionExport);
console.log('Video ID extracted:', videoId);
console.log('SRT path:', srtPath);
```

### 2. Check Rust Logs
Rust backend uses `tracing` for logging:
```rust
tracing::info!("Starting SRT export for video_id: {}", video_id);
```

### 3. Verify File Existence
Always check if files exist before operations:
- Check if captions.json exists before export
- Verify SRT was created before copying
- Confirm video export succeeded before SRT export

## Future Improvements

### Recommended Enhancements

1. **Error Recovery**: Add retry logic for file operations
2. **Progress Feedback**: Show SRT export progress in UI
3. **Batch Operations**: Support exporting multiple caption formats
4. **Validation**: Pre-flight checks before export
5. **Settings Persistence**: Remember user's caption export preference

### Architecture Improvements

1. **Centralized State**: Consider moving caption persistence to a state manager
2. **Event System**: Use events to trigger auto-save instead of manual calls
3. **Type Safety**: Ensure all Tauri commands have proper TypeScript types
4. **Error Handling**: Implement comprehensive error boundaries and recovery

## Testing Checklist

When modifying caption/export functionality:

- [ ] Generate captions for a video
- [ ] Edit caption text
- [ ] Delete a caption segment
- [ ] Change caption style settings
- [ ] Export video without captions
- [ ] Export video with burned-in captions
- [ ] Export video with SRT file
- [ ] Verify SRT file location
- [ ] Check SRT file content/format
- [ ] Test with videos of different lengths
- [ ] Test with empty captions
- [ ] Test error scenarios (disk full, permissions, etc.)

## Related Files for Context

When working on caption/export features, review these files:

1. **Frontend**:
   - `ExportDialog.tsx`: Export UI and orchestration
   - `CaptionsTab.tsx`: Caption editing and persistence
   - `tauri.ts`: Command bindings
   - `export.ts`: Export utilities

2. **Backend**:
   - `captions.rs`: Core caption logic
   - `lib.rs`: File operations and command registration
   - `export.rs`: Video export implementation

3. **Configuration**:
   - `Cargo.toml`: Rust dependencies
   - `package.json`: Frontend dependencies

## Command Reference

### Generate Captions
```typescript
await commands.generateCaptions(videoId, modelPath?)
```

### Save Captions
```typescript
await commands.saveCaptions(videoId, captionData)
```

### Export SRT
```typescript
const srtPath = await commands.exportCaptionsSrt(videoId)
```

### Copy File
```typescript
await commands.copyFileToPath(sourcePath, destinationPath)
```

## Notes for Future Development

1. **Always persist before export**: Captions must be saved to disk before export
2. **Path handling is critical**: Many issues stem from incorrect path parsing
3. **File type awareness**: The copy function needs to know what type of file it's handling
4. **User feedback is essential**: Always show progress and success/failure states
5. **Test the full flow**: Individual components may work but fail when integrated

---

*Document created: 2025-08-28*
*Based on: Extensive debugging session fixing SRT export functionality*
*Key lesson: "Step back and really think about what's going on" - systematic analysis beats guessing*