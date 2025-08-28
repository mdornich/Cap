# Future Features

This document tracks potential future features and enhancements for Cap/Klip. Features are organized by category and include implementation complexity estimates.

## 🎯 What's Next - Priority Queue

### Immediate Priorities (This Week)
1. **Video Transcription** ✅ **COMPLETE**
   - ✅ Basic transcription working with microphone audio
   - ✅ Mixed audio sources handled correctly
   - ✅ Model selection already available (Whisper models)
   - Note: Language translation moved to viewer features (let players handle translation of SRT files)

2. **Closed Captioning Display**
   - ✅ WebVTT/SRT export (completed 2025-08-28)
   - ⏳ Add caption editing interface
   - ⏳ Burn-in option for exported videos

3. **Collaborative Feedback on Shareable Links**
   - Timestamp-based commenting
   - Threaded discussions
   - Real-time updates

### Next Sprint (Next 2 Weeks)
1. **Filler Word Removal** - Clean up recordings automatically
2. **Silence Removal** - Make videos more concise
3. **Auto-generate Bug Reports** - Jira/Linear integration

### Backlog (1-2 Months)
1. **Cloudflare Stream Integration** - Alternative storage provider
2. **Detailed Engagement Analytics** - Viewer behavior insights
3. **Auto Summaries** - AI-generated video summaries
4. **Transform Videos into Text Documents** - Content repurposing

---

## Recently Completed ✅

See [COMPLETED_FEATURES.md](./COMPLETED_FEATURES.md) for full list of implemented features.

- **Quick Caption Toggle** (2025-08-26) - Keyboard shortcut 'C' with visual feedback
- **Transcription Fixes** (2025-08-26) - Fixed crashes, improved mic audio handling
- **Command+R Recording Hotkey** (2025-08-25) - Global recording shortcuts

---

## Storage & Infrastructure

### Cloudflare Stream Integration
**Priority:** Medium | **Complexity:** High (2-3 weeks)

Add Cloudflare Stream as an alternative video storage provider alongside existing S3-compatible options.

**Benefits:**
- Automatic video encoding and optimization
- Built-in adaptive streaming (HLS/DASH)
- Per-minute pricing model (vs per-GB)
- No bandwidth/egress fees
- Integrated video player with customization options
- Automatic thumbnail generation

**Technical Requirements:**
- New database schema for Stream providers (account ID, API token)
- Implement `VideoStreamProvider` interface parallel to `S3BucketProvider`
- Support for TUS protocol (resumable uploads for files >200MB)
- Direct creator upload URLs for user-generated content
- Webhook handlers for encoding completion
- Modified upload flow to handle Stream's REST API

**UI Changes:**
- Add "Cloudflare Stream" to storage provider dropdown
- New configuration fields (Account ID, API Token)
- Display Stream-specific features and pricing model
- Show encoding progress and video analytics

**Considerations:**
- Stream is video-only (would need hybrid approach with S3 for other assets)
- Different URL structure for playback
- Automatic encoding means less control over output formats

---

## Collaboration & Sharing

### Emoji Reactions
**Priority:** Low | **Complexity:** Low (3 days)
**Requested by:** @mdornich | **Date:** 2025-01-23

Add emoji reactions to videos for quick feedback.

**Benefits:**
- Quick viewer feedback
- Engagement tracking
- Fun interaction method
- Lightweight feedback option

**Technical Requirements:**
- Emoji picker UI
- Reaction storage and display
- Real-time reaction updates
- Analytics tracking

### Video Comments
**Priority:** Medium | **Complexity:** Medium (1 week)
**Requested by:** @mdornich | **Date:** 2025-01-23

Enable commenting directly on videos.

**Benefits:**
- Viewer engagement
- Feedback collection
- Discussion threads
- Community building

**Technical Requirements:**
- Comment system implementation
- Threaded replies
- Moderation tools
- Notification system

### Multi-Language Caption Translation (Viewer-Side)
**Priority:** Medium | **Complexity:** Low (viewer platform dependent)
**Requested by:** @mdornich | **Date:** 2025-01-23
**Updated:** 2025-08-28

Enable viewers to translate SRT captions to their preferred language using video player capabilities.

**Benefits:**
- Global reach without pre-generating multiple languages
- Leverage existing player translation features (YouTube, Vimeo, etc.)
- More efficient than transcribing in multiple languages
- Viewers get real-time translation in their preferred language
- No additional storage for multiple caption files

**Technical Requirements:**
- Ensure SRT files are properly formatted for player compatibility
- Metadata to indicate source language of captions
- Documentation on which platforms support auto-translation
- Consider providing translation API integration for custom player

**Note:** Most modern video platforms can automatically translate SRT files. This is more efficient than pre-generating multiple language versions during transcription.

### Watch Later Functionality
**Priority:** Low | **Complexity:** Low (2 days)
**Requested by:** @mdornich | **Date:** 2025-01-23

Save videos to watch later list.

**Benefits:**
- Better content organization
- Viewer retention
- Personal video library
- Bookmark management

**Technical Requirements:**
- User watchlist storage
- Queue management
- Sync across devices
- Reminder notifications

### Viewer Insights
**Priority:** Medium | **Complexity:** Medium (1 week)
**Requested by:** @mdornich | **Date:** 2025-01-23

Basic analytics for video performance.

**Benefits:**
- Understand viewer behavior
- Content optimization
- Engagement tracking
- ROI measurement

**Technical Requirements:**
- View tracking
- Watch time analytics
- Viewer demographics
- Basic reporting dashboard

### Collaborative Feedback on Shareable Links
**Priority:** High | **Complexity:** Medium (1-2 weeks)
**Requested by:** @mdornich | **Date:** 2025-01-23

Enable built-in threaded commenting and collaboration directly on shareable video links, eliminating the need to switch between different tools for feedback.

**Benefits:**
- Streamlined feedback workflow - all in one place
- Context-aware comments tied to specific timestamps
- Threaded discussions for organized conversations
- No need for external collaboration tools (Slack, email, etc.)
- Faster iteration cycles on video content
- Professional presentation for client reviews

**Technical Requirements:**
- Comments database schema (comments, replies, users)
- Real-time updates using WebSockets or SSE
- Timestamp-based comment anchoring
- User authentication for commenters (or anonymous with names)
- Comment notifications system
- UI overlay for video player with comment sidebar
- Markdown support for rich text comments
- @mentions for directing feedback
- Comment resolution/status tracking

**UI/UX Features:**
- Floating comment bubbles on video timeline
- Side panel for threaded discussions
- Click on timeline to add comment at timestamp
- Comment filters (resolved/unresolved, by user)
- Email notifications for new comments
- Export comments as PDF/document for records

**Considerations:**
- Need moderation features (delete/edit permissions)
- Storage implications for comment data
- Privacy controls (who can comment, view comments)
- Mobile-responsive commenting interface
- Integration with existing sharing permissions

---

## Accessibility & Captions

### Video Transcription
**Priority:** High | **Complexity:** Medium (1-2 weeks)
**Requested by:** @mdornich | **Date:** 2025-01-23

Automatically generate transcripts for recorded videos using speech-to-text technology.

**Benefits:**
- Improved accessibility for hearing-impaired users
- Searchable video content
- Quick reference without watching entire video
- SEO benefits for shared content
- Foundation for closed captions feature

**Technical Requirements:**
- Integration with transcription service (Whisper API, Google Speech-to-Text, or AWS Transcribe)
- Background processing queue for transcript generation
- Storage for transcript data (JSON/VTT format)
- UI for viewing and editing transcripts
- Export options (TXT, SRT, VTT formats)
- Timestamp synchronization with video playback

**UI/UX Features:**
- Transcript panel in video player
- Click on transcript to jump to timestamp
- Search within transcript
- Edit/correct transcript text
- Download transcript in multiple formats

**Considerations:**
- Cost per minute of transcription
- Processing time for longer videos
- Language detection and multi-language support
- Privacy concerns for sensitive content
- Local vs cloud processing options

### Closed Captioning
**Priority:** High | **Complexity:** Medium (1 week, builds on transcription)
**Requested by:** @mdornich | **Date:** 2025-01-23

Display synchronized closed captions directly on videos during playback.

**Benefits:**
- Full accessibility compliance
- Better engagement in sound-off environments
- Multi-language support potential
- Professional video presentation
- Required for many platforms/clients

**Technical Requirements:**
- Build on transcription feature (prerequisite)
- WebVTT/SRT caption file generation
- Caption rendering engine in video player
- Caption styling options (font, size, color, background)
- Burn-in option for exported videos
- Multi-track caption support (different languages)

**UI/UX Features:**
- Toggle captions on/off
- Caption style customization
- Position adjustment (top/bottom)
- Auto-generated vs manual caption indicators
- Caption editing interface with preview

**Considerations:**
- Performance impact of real-time caption rendering
- Subtitle vs caption standards (timing, formatting)
- Mobile device caption display
- Export with embedded vs burned-in captions
- Platform-specific caption requirements

---

## Advanced Analytics

### Detailed Engagement Insights
**Priority:** High | **Complexity:** High (2 weeks)
**Requested by:** @mdornich | **Date:** 2025-01-23

Comprehensive analytics dashboard with deep engagement metrics.

**Benefits:**
- Understand viewer behavior patterns
- Identify drop-off points
- Optimize content strategy
- Track ROI and performance

**Technical Requirements:**
- Advanced analytics engine
- Heat map generation
- Funnel analysis
- Cohort tracking
- Real-time data processing

### Exportable Engagement Insights
**Priority:** Medium | **Complexity:** Low (3 days)
**Requested by:** @mdornich | **Date:** 2025-01-23

Export analytics data for external analysis and reporting.

**Benefits:**
- Custom reporting
- Data portability
- Integration with BI tools
- Stakeholder presentations

**Technical Requirements:**
- Export to CSV/Excel
- API for data access
- Scheduled reports
- Custom date ranges

### Enhanced Viewer Analytics
**Priority:** High | **Complexity:** Medium (1 week)
**Requested by:** @mdornich | **Date:** 2025-01-23

Detailed viewer behavior and demographic insights.

**Benefits:**
- Viewer segmentation
- Personalization opportunities
- Content recommendations
- Audience understanding

**Technical Requirements:**
- Viewer tracking system
- Geographic data
- Device/browser analytics
- Engagement scoring
- Retention metrics

---

## Loom AI Suite

### Auto Titles
**Priority:** Medium | **Complexity:** Low (3-5 days)
**Requested by:** @mdornich | **Date:** 2025-01-23

AI-generated video titles based on content analysis.

**Benefits:**
- Save time on manual titling
- Consistent naming conventions
- SEO-optimized titles
- Context-aware descriptions

**Technical Requirements:**
- Integration with LLM API (OpenAI, Claude, etc.)
- Video transcript as input
- Title generation prompt engineering
- Fallback to manual entry

### Auto Summaries
**Priority:** High | **Complexity:** Medium (1 week)
**Requested by:** @mdornich | **Date:** 2025-01-23

Automatic video summaries for quick content overview.

**Benefits:**
- Quick content scanning
- Better video discovery
- Email/Slack sharing snippets
- Documentation generation

**Technical Requirements:**
- Transcript-based summarization
- Key point extraction
- Multiple summary lengths (tweet, paragraph, detailed)
- Export to various formats

### Auto Chapters
**Priority:** Medium | **Complexity:** Medium (1 week)
**Requested by:** @mdornich | **Date:** 2025-01-23

Automatic chapter breaks for video navigation.

**Benefits:**
- Improved video navigation
- Quick topic jumping
- Better viewer retention
- Professional presentation

**Technical Requirements:**
- Topic segmentation algorithm
- Timestamp generation
- Chapter title creation
- Manual adjustment interface

### Auto Tasks
**Priority:** High | **Complexity:** Medium (1 week)
**Requested by:** @mdornich | **Date:** 2025-01-23

AI-identified action items from video content.

**Benefits:**
- Never miss follow-ups
- Automatic task tracking
- Team accountability
- Meeting efficiency

**Technical Requirements:**
- Action item detection
- Assignee identification
- Due date extraction
- Integration with task management tools

### Auto CTA
**Priority:** Low | **Complexity:** Low (3 days)
**Requested by:** @mdornich | **Date:** 2025-01-23

Custom call-to-action buttons on videos.

**Benefits:**
- Drive viewer actions
- Increase conversions
- Track engagement
- Flexible messaging

**Technical Requirements:**
- CTA template library
- Custom button designer
- Click tracking
- A/B testing support

### Filler Word Removal
**Priority:** High | **Complexity:** High (2 weeks)
**Requested by:** @mdornich | **Date:** 2025-01-23

Removes "ums," "ahs," and verbal fillers.

**Benefits:**
- More professional videos
- Shorter video duration
- Better viewer experience
- Cleaner transcripts

**Technical Requirements:**
- Audio analysis and detection
- Natural silence preservation
- Video/audio sync maintenance
- Customizable filler word list
- Preview before applying

### Silence Removal
**Priority:** High | **Complexity:** Medium (1 week)
**Requested by:** @mdornich | **Date:** 2025-01-23

Automatic dead air removal from recordings.

**Benefits:**
- Shorter, more engaging videos
- Professional editing
- Time savings for viewers
- Improved pacing

**Technical Requirements:**
- Silence detection algorithm
- Configurable silence threshold
- Natural pause preservation
- Undo/redo functionality

---

## AI Workflows

### Transform Videos into Text Documents
**Priority:** High | **Complexity:** Medium (1 week)
**Requested by:** @mdornich | **Date:** 2025-01-23

Convert video content into formatted text documents.

**Benefits:**
- Repurpose video content
- Create written documentation
- Improve accessibility
- Enable text-based workflows

**Technical Requirements:**
- Transcript formatting engine
- Document templates
- Export to multiple formats (MD, DOCX, PDF)
- Style customization

### Generate SOPs
**Priority:** Medium | **Complexity:** Medium (1 week)
**Requested by:** @mdornich | **Date:** 2025-01-23

Create Standard Operating Procedures from video demonstrations.

**Benefits:**
- Automated documentation
- Consistent procedures
- Training material generation
- Compliance documentation

**Technical Requirements:**
- Step extraction from video
- Screenshot capture at key moments
- Template-based formatting
- Version control integration

### Create Step-by-Step Guides
**Priority:** Medium | **Complexity:** Medium (1 week)
**Requested by:** @mdornich | **Date:** 2025-01-23

Generate detailed guides from video tutorials.

**Benefits:**
- Learning material creation
- Onboarding documentation
- Knowledge base articles
- Tutorial generation

**Technical Requirements:**
- Action detection in video
- Automatic screenshot capture
- Numbered step generation
- Annotation tools

### Generate PR Descriptions
**Priority:** Low | **Complexity:** Low (3 days)
**Requested by:** @mdornich | **Date:** 2025-01-23

Create pull request descriptions from code walkthrough videos.

**Benefits:**
- Faster PR creation
- Better code documentation
- Consistent PR format
- Change summary automation

**Technical Requirements:**
- Code change detection
- PR template integration
- GitHub/GitLab API integration
- Commit message generation

### Create QA Testing Steps
**Priority:** Medium | **Complexity:** Medium (1 week)
**Requested by:** @mdornich | **Date:** 2025-01-23

Generate QA test cases from bug reproduction videos.

**Benefits:**
- Automated test documentation
- Reproducible test scenarios
- Better bug tracking
- QA efficiency

**Technical Requirements:**
- User action tracking
- Test step generation
- Expected outcome detection
- Test case formatting

### Generate Code Documentation
**Priority:** Low | **Complexity:** Medium (1 week)
**Requested by:** @mdornich | **Date:** 2025-01-23

Create code documentation from code review videos.

**Benefits:**
- Automated documentation
- Code explanation capture
- Architecture documentation
- API documentation

**Technical Requirements:**
- Code snippet extraction
- Comment generation
- Documentation formatting
- Integration with doc tools

### Auto-generate Bug Reports
**Priority:** High | **Complexity:** Medium (1 week)
**Requested by:** @mdornich | **Date:** 2025-01-23

Create bug reports for Jira/Linear from bug demonstration videos.

**Benefits:**
- Faster bug reporting
- Complete reproduction steps
- Visual bug documentation
- Reduced back-and-forth

**Technical Requirements:**
- Bug pattern detection
- Step extraction
- Screenshot/video attachment
- Jira/Linear API integration
- Template customization

### Generate Email and Slack Messages
**Priority:** Medium | **Complexity:** Low (3 days)
**Requested by:** @mdornich | **Date:** 2025-01-23

Create sharing messages for video content.

**Benefits:**
- Quick video sharing
- Consistent messaging
- Context-aware summaries
- Multi-platform support

**Technical Requirements:**
- Message template library
- Video summary inclusion
- Platform-specific formatting
- One-click sharing

---

## User Interface & Controls

### ✅ Quick Caption Toggle
**Priority:** Medium | **Complexity:** Low (2-3 days)
**Requested by:** @mitchdornich | **Date:** 2025-08-23
**Status:** ✅ COMPLETED | **Completed:** 2025-08-25

Add a quick-access caption toggle button to the editor toolbar for instant captions on/off during video playback.

**Benefits:**
- Instant caption visibility toggle without opening config sidebar
- Improved accessibility workflow
- Better user experience for caption review and editing
- Quick testing of caption appearance during editing

**Implementation Completed:**
- ✅ Connected caption button to caption display logic
- ✅ Added state management for caption visibility toggle
- ✅ Integrated with existing transcription system
- ✅ Syncs with caption settings panel
- ✅ Keyboard shortcut 'C' implemented
- ✅ Visual feedback (button highlights when enabled)
- ✅ Smart state checking (disabled when no captions exist)
- ✅ Proper synchronization between global store and local state

---

### ~~Performance Monitor Dashboard~~ (Deprioritized)
**Priority:** ~~Low~~ | **Complexity:** ~~Medium (1-2 weeks)~~  
**Requested by:** @mitchdornich | **Date:** 2025-08-23
**Status:** ⚠️ DEPRIORITIZED - Button reserved for future functionality

~~Add a performance monitoring button to the editor toolbar showing real-time stats during video editing and export.~~

**Note:** Performance monitoring deemed not worth the implementation effort. The existing UI button (gauge icon) will be repurposed for other functionality in the future.

**UI Status:**
- Button exists in `apps/desktop/src/routes/editor/Header.tsx`
- Icon (`IconCapGauge`) placeholder remains
- Marked as `comingSoon` - available for reassignment

---

## How to Contribute

When adding new feature ideas:
1. Choose appropriate category or create new one
2. Include priority (Low/Medium/High) and complexity estimate
3. List key benefits and technical requirements
4. Note any important considerations or trade-offs
5. Add your GitHub username and date

## Feature Request Template

```markdown
### [Feature Name]
**Priority:** [Low/Medium/High] | **Complexity:** [Simple/Medium/High (time estimate)]
**Requested by:** @username | **Date:** YYYY-MM-DD

[Brief description]

**Benefits:**
- [Benefit 1]
- [Benefit 2]

**Technical Requirements:**
- [Requirement 1]
- [Requirement 2]

**Considerations:**
- [Any trade-offs or concerns]
```

---

*Last Updated: 2025-01-23*