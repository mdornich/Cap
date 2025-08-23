# Future Features

This document tracks potential future features and enhancements for Cap/Klip. Features are organized by category and include implementation complexity estimates.

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