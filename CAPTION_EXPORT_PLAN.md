# Caption Export & Multi-Language Support Plan

## User Journey & Requirements

### Core Requirements
- **Caption Generation**: Happens in editor during recording post-processing
- **Multi-Language Support**: Recorder can generate captions in multiple languages before sharing
- **Not Burned In**: Captions are separate files, not embedded in video
- **Viewer Control**: Recipients can toggle captions on/off and select language
- **Similar to TV CC**: Professional closed caption experience

### User Journey

#### Recorder's Journey (Desktop App)
1. **Record Video** → Complete recording
2. **Open Editor** → Navigate to Captions tab
3. **Generate Captions** → AI transcribes in recorder's language
4. **Add Languages** → Optionally translate to other languages
5. **Review & Edit** → Fix any transcription/translation errors
6. **Export Video** → Includes all caption tracks as separate files
7. **Share Link** → Upload video + caption files

#### Viewer's Journey (Web Player)
1. **Open Shared Link** → Load video player
2. **See CC Button** → Notice captions are available
3. **Toggle Captions** → Turn on/off with CC button
4. **Select Language** → Choose preferred language from dropdown
5. **Watch with Captions** → Captions overlay on video (not burned in)

## Current System Analysis

### What Works Today
- ✅ Whisper AI transcription in desktop app
- ✅ Caption generation with timestamps
- ✅ Caption display in editor with toggle
- ✅ Web player has CC toggle button
- ✅ VTT parsing capability exists

### What's Missing
- ❌ Multi-language data model
- ❌ Translation capability in editor
- ❌ Caption file export (VTT/SRT)
- ❌ Upload captions with video
- ❌ Language selection in web player

## Implementation Plan

### Phase 1: Basic Caption Export (3 days)
**Goal**: Export single language caption files

#### 1.1 Add Export Functions (Backend)
```rust
// In src-tauri/src/captions.rs
pub async fn export_captions_to_vtt(project_path: PathBuf) -> Result<String>
pub async fn export_captions_to_srt(project_path: PathBuf) -> Result<String>
pub async fn export_captions_to_text(project_path: PathBuf) -> Result<String>
```

#### 1.2 Update Export Dialog (Frontend)
- Add caption section to ExportDialog.tsx
- Show when captions exist
- Export format options: VTT, SRT, Text
- Independent download buttons

#### 1.3 Format Converters
```rust
fn segments_to_vtt(segments: Vec<CaptionSegment>) -> String
fn segments_to_srt(segments: Vec<CaptionSegment>) -> String
```

### Phase 2: Multi-Language Support (5 days)
**Goal**: Support multiple language tracks

#### 2.1 Update Data Model
```rust
// crates/project/src/configuration.rs
pub struct CaptionTrack {
    pub language: String,      // ISO 639-1 code
    pub label: String,          // Display name
    pub is_original: bool,
    pub segments: Vec<CaptionSegment>,
}

pub struct CaptionsData {
    pub tracks: Vec<CaptionTrack>,
    pub settings: CaptionSettings,
}
```

#### 2.2 Translation in Editor
- Add "Add Language" button in CaptionsTab.tsx
- Integrate translation API (Google/DeepL)
- Allow editing each language track
- Save all tracks to project

#### 2.3 Multi-Track Export
- Export each language as separate file
- Naming: `captions-en.vtt`, `captions-hi.vtt`

### Phase 3: Upload Integration (3 days)
**Goal**: Include captions with shared videos

#### 3.1 Upload Flow
- Upload video file to S3/R2
- Upload each caption file
- Store URLs in database
- Return caption metadata in API

#### 3.2 Database Schema
```sql
-- Add to video metadata
captions: {
  "en": "https://storage/id/captions-en.vtt",
  "hi": "https://storage/id/captions-hi.vtt",
  "es": "https://storage/id/captions-es.vtt"
}
```

### Phase 4: Web Player Enhancement (2 days)
**Goal**: Multi-language caption playback

#### 4.1 Player Updates
- Load available caption tracks
- Language selector dropdown
- Fetch VTT file for selected language
- Render captions as overlay

#### 4.2 UI Components
```tsx
// Caption controls
<Button onClick={() => setCaptionsEnabled(!captionsEnabled)}>CC</Button>

// Language selector (shown when CC enabled)
<Select value={selectedLang} onChange={setSelectedLang}>
  <option value="en">English</option>
  <option value="hi">Hindi</option>
  <option value="es">Spanish</option>
</Select>
```

## Technical Considerations

### Translation Strategy
- **When**: During editing, not playback
- **Why**: Quality control, one-time cost, faster playback
- **How**: API integration with review/edit capability

### Storage Approach
- **Format**: Separate VTT files per language
- **Size**: ~50KB per language track
- **Location**: Same S3/R2 bucket as video

### Fallback Behavior
- Default to original language if preferred not available
- Show "Auto-translated" badge for non-original tracks
- Allow downloading caption files

## File Modifications

### Desktop App
- `/apps/desktop/src-tauri/src/captions.rs` - Export functions
- `/apps/desktop/src/routes/editor/ExportDialog.tsx` - UI integration
- `/apps/desktop/src/routes/editor/CaptionsTab.tsx` - Multi-language UI
- `/crates/project/src/configuration.rs` - Data model updates

### Web App
- `/apps/web/app/s/[videoId]/_components/CapVideoPlayer.tsx` - Language selector
- `/apps/web/app/api/video/metadata/route.ts` - Return caption URLs
- `/apps/web/app/s/[videoId]/_components/tabs/Transcript.tsx` - Multi-track display

## Success Metrics
- Viewers can toggle captions on/off
- Multiple language options available
- No video quality degradation (not burned in)
- Smooth language switching
- Proper synchronization maintained

## Future Enhancements
- Auto-detect viewer's preferred language
- Crowd-sourced caption improvements
- Real-time collaborative translation
- Caption styling customization per viewer

---
*Last Updated: 2025-08-26*
*This document captures the complete user journey and implementation plan for multi-language caption support*