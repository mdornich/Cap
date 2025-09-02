use std::collections::BTreeMap;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use cap_project::{RecordingMeta, RecordingMetaInner, StudioRecordingMeta};
use mp4::Mp4Reader;
use serde::Serialize;
use specta::Type;
use tauri::AppHandle;

use crate::{
    editor_window::WindowEditorInstance, CapWindowId, ShowCapWindow,
    VideoRecordingMetadata,
};

/// Creates a thumbnail from an input image file
pub async fn create_thumbnail(input: PathBuf, output: PathBuf, size: (u32, u32)) -> Result<(), String> {
    println!(
        "Creating thumbnail: input={:?}, output={:?}, size={:?}",
        input, output, size
    );

    tokio::task::spawn_blocking(move || -> Result<(), String> {
        let img = image::open(&input).map_err(|e| {
            eprintln!("Failed to open image: {}", e);
            e.to_string()
        })?;

        let width = img.width() as usize;
        let height = img.height() as usize;
        let bytes_per_pixel = 3;
        let src_stride = width * bytes_per_pixel;

        let rgb_img = img.to_rgb8();
        let img_buffer = rgb_img.as_raw();

        let mut corrected_buffer = vec![0u8; height * src_stride];

        for y in 0..height {
            let src_slice = &img_buffer[y * src_stride..(y + 1) * src_stride];
            let dst_slice = &mut corrected_buffer[y * src_stride..(y + 1) * src_stride];
            dst_slice.copy_from_slice(src_slice);
        }

        let corrected_img =
            image::RgbImage::from_raw(width as u32, height as u32, corrected_buffer)
                .ok_or("Failed to create corrected image")?;

        let thumbnail = image::imageops::resize(
            &corrected_img,
            size.0,
            size.1,
            image::imageops::FilterType::Lanczos3,
        );

        thumbnail
            .save_with_format(&output, image::ImageFormat::Png)
            .map_err(|e| {
                eprintln!("Failed to save thumbnail: {}", e);
                e.to_string()
            })?;

        println!("Thumbnail created successfully");
        Ok(())
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
#[specta::specta]
pub async fn get_video_metadata(path: PathBuf) -> Result<VideoRecordingMetadata, String> {
    let recording_meta = RecordingMeta::load_for_project(&path).map_err(|v| v.to_string())?;

    fn get_duration_for_path(path: PathBuf) -> Result<f64, String> {
        let reader = BufReader::new(
            File::open(&path).map_err(|e| format!("Failed to open video file: {}", e))?,
        );
        let file_size = path
            .metadata()
            .map_err(|e| format!("Failed to get file metadata: {}", e))?
            .len();

        let current_duration = match Mp4Reader::read_header(reader, file_size) {
            Ok(mp4) => mp4.duration().as_secs_f64(),
            Err(e) => {
                println!(
                    "Failed to read MP4 header: {}. Falling back to default duration.",
                    e
                );
                0.0_f64
            }
        };

        Ok(current_duration)
    }

    let display_paths = match &recording_meta.inner {
        RecordingMetaInner::Instant(_) => {
            vec![path.join("content/output.mp4")]
        }
        RecordingMetaInner::Studio(meta) => match meta {
            StudioRecordingMeta::SingleSegment { segment } => {
                vec![recording_meta.path(&segment.display.path)]
            }
            StudioRecordingMeta::MultipleSegments { inner, .. } => inner
                .segments
                .iter()
                .map(|s| recording_meta.path(&s.display.path))
                .collect(),
        },
    };

    let duration = display_paths
        .into_iter()
        .map(get_duration_for_path)
        .sum::<Result<_, _>>()?;

    let (width, height) = (1920, 1080);
    let fps = 30;

    let base_bitrate = if width <= 1280 && height <= 720 {
        4_000_000.0
    } else if width <= 1920 && height <= 1080 {
        8_000_000.0
    } else if width <= 2560 && height <= 1440 {
        14_000_000.0
    } else {
        20_000_000.0
    };

    let fps_factor = (fps as f64) / 30.0;
    let video_bitrate = base_bitrate * fps_factor;
    let audio_bitrate = 192_000.0;
    let total_bitrate = video_bitrate + audio_bitrate;
    let estimated_size_mb = (total_bitrate * duration) / (8.0 * 1024.0 * 1024.0);

    Ok(VideoRecordingMetadata {
        size: estimated_size_mb,
        duration,
    })
}

#[tauri::command]
#[specta::specta]
pub fn close_recordings_overlay_window(app: AppHandle) {
    #[cfg(target_os = "macos")]
    {
        use tauri_nspanel::ManagerExt;
        if let Ok(panel) = app.get_webview_panel(&CapWindowId::RecordingsOverlay.label()) {
            panel.released_when_closed(true);
            panel.close();
        }
    }

    if !cfg!(target_os = "macos") {
        if let Some(window) = CapWindowId::RecordingsOverlay.get(&app) {
            let _ = window.close();
        }
    }
}

#[tauri::command(async)]
#[specta::specta]
pub fn focus_captures_panel(app: AppHandle) {
    #[cfg(target_os = "macos")]
    {
        use tauri_nspanel::ManagerExt;
        if let Ok(panel) = app.get_webview_panel(&CapWindowId::RecordingsOverlay.label()) {
            panel.make_key_window();
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn is_camera_window_open(app: AppHandle) -> bool {
    CapWindowId::Camera.get(&app).is_some()
}

#[tauri::command]
#[specta::specta]
pub async fn get_system_audio_waveforms(
    editor_instance: WindowEditorInstance,
) -> Result<Vec<Vec<f32>>, String> {
    let mut out = Vec::new();

    for segment in editor_instance.segments.iter() {
        if let Some(_audio) = &segment.system_audio {
            // TODO: Implement proper waveform extraction
            out.push(Vec::new());
        } else {
            out.push(Vec::new());
        }
    }

    Ok(out)
}

// Keep this async otherwise opening windows may hang on windows
#[tauri::command]
#[specta::specta]
pub async fn show_window(app: AppHandle, window: ShowCapWindow) -> Result<(), String> {
    window.show(&app).await.unwrap();
    Ok(())
}

#[tauri::command(async)]
#[specta::specta]
pub fn list_fails() -> Result<BTreeMap<String, bool>, ()> {
    Ok(cap_fail::get_state())
}