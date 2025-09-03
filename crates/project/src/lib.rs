mod configuration;
pub mod cursor;
mod meta;

pub use configuration::*;
pub use cursor::*;
pub use meta::*;

use serde::{Deserialize, Serialize};
use specta::Type;

#[cfg(test)]
mod scene_track_tests {
    use crate::configuration::{SceneMode, SceneSegment, TimelineConfiguration};

    #[test]
    fn test_get_scene_mode_at_time_basic() {
        let timeline = TimelineConfiguration {
            segments: vec![],
            zoom_segments: vec![],
            scene_segments: Some(vec![
            SceneSegment {
                start: 0.0,
                end: 5.0,
                mode: Some(SceneMode::Default),
            },
            SceneSegment {
                start: 5.0,
                end: 10.0,
                mode: Some(SceneMode::CameraOnly),
            },
            SceneSegment {
                start: 10.0,
                end: 15.0,
                mode: Some(SceneMode::HideCamera),
            },
            ]),
        };

        // Test within first segment
        assert_eq!(timeline.get_scene_mode_at_time(2.5), Some(SceneMode::Default));
        
        // Test within second segment
        assert_eq!(timeline.get_scene_mode_at_time(7.5), Some(SceneMode::CameraOnly));
        
        // Test within third segment
        assert_eq!(timeline.get_scene_mode_at_time(12.5), Some(SceneMode::HideCamera));
        
        // Test beyond all segments
        assert_eq!(timeline.get_scene_mode_at_time(20.0), None);
    }

    #[test]
    fn test_get_scene_mode_at_time_edge_cases() {
        let timeline = TimelineConfiguration {
            segments: vec![],
            zoom_segments: vec![],
            scene_segments: Some(vec![
                SceneSegment {
                    start: 0.0,
                    end: 5.0,
                    mode: Some(SceneMode::Default),
                }
            ]),
        };

        // Test exact start time
        assert_eq!(timeline.get_scene_mode_at_time(0.0), Some(SceneMode::Default));
        
        // Test exact end time (should not match)
        assert_eq!(timeline.get_scene_mode_at_time(5.0), None);
        
        // Test with no segments
        let empty_timeline = TimelineConfiguration {
            segments: vec![],
            zoom_segments: vec![],
            scene_segments: None,
        };
        assert_eq!(empty_timeline.get_scene_mode_at_time(5.0), None);
    }

    #[test]
    fn test_get_scene_mode_at_time_none_mode() {
        let timeline = TimelineConfiguration {
            segments: vec![],
            zoom_segments: vec![],
            scene_segments: Some(vec![
                SceneSegment {
                    start: 0.0,
                    end: 5.0,
                    mode: None, // Should default to SceneMode::Default
                }
            ]),
        };

        assert_eq!(timeline.get_scene_mode_at_time(2.5), Some(SceneMode::Default));
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RecordingConfig {
    pub fps: u32,
    pub resolution: Resolution,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

impl Default for RecordingConfig {
    fn default() -> Self {
        Self {
            fps: 30,
            resolution: Resolution {
                width: 1920,
                height: 1080,
            },
        }
    }
}
