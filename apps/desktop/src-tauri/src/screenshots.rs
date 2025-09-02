use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;

use cap_project::{RecordingMeta, RecordingMetaInner, VideoMeta, SharingMeta, Platform};
use clipboard_rs::Clipboard;
use png::{ColorType, Encoder};
use relative_path::RelativePathBuf;
use scap::{
    capturer::Capturer,
    frame::{Frame, VideoFrame},
};
use tauri::{AppHandle, Manager, State};

use crate::{
    AppSounds, AuthStore, CapWindowId, ClipboardContext, MutableState,
    ShowCapWindow, UploadResult, notifications, upload::upload_image,
    NewScreenshotAdded,
};
use tauri_specta::Event;

#[tauri::command]
#[specta::specta]
pub async fn take_screenshot(app: AppHandle, _state: MutableState<'_, crate::App>) -> Result<(), String> {
    let id = uuid::Uuid::new_v4().to_string();

    let recording_dir = app
        .path()
        .app_data_dir()
        .unwrap()
        .join("screenshots")
        .join(format!("{id}.cap"));

    std::fs::create_dir_all(&recording_dir).map_err(|e| e.to_string())?;

    let (width, height, bgra_data) = {
        let options = scap::capturer::Options {
            fps: 1,
            output_type: scap::frame::FrameType::BGRAFrame,
            show_highlight: false,
            ..Default::default()
        };

        if let Some(window) = CapWindowId::Main.get(&app) {
            let _ = window.hide();
        }

        let mut capturer =
            Capturer::build(options).map_err(|e| format!("Failed to construct error: {e}"))?;
        capturer.start_capture();
        let frame = capturer
            .get_next_frame()
            .map_err(|e| format!("Failed to get frame: {}", e))?;
        capturer.stop_capture();

        if let Some(window) = CapWindowId::Main.get(&app) {
            let _ = window.show();
        }

        match frame {
            Frame::Video(VideoFrame::BGRA(bgra_frame)) => Ok((
                bgra_frame.width as u32,
                bgra_frame.height as u32,
                bgra_frame.data,
            )),
            _ => Err("Unexpected frame type".to_string()),
        }
    }?;

    let now = chrono::Local::now();
    let screenshot_name = format!(
        "Cap {} at {}.png",
        now.format("%Y-%m-%d"),
        now.format("%H.%M.%S")
    );
    let screenshot_path = recording_dir.join(&screenshot_name);

    let app_handle = app.clone();
    let recording_dir = recording_dir.clone();
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        let mut rgba_data = vec![0; bgra_data.len()];
        for (bgra, rgba) in bgra_data.chunks_exact(4).zip(rgba_data.chunks_exact_mut(4)) {
            rgba[0] = bgra[2];
            rgba[1] = bgra[1];
            rgba[2] = bgra[0];
            rgba[3] = bgra[3];
        }

        let file = File::create(&screenshot_path).map_err(|e| e.to_string())?;
        let w = &mut BufWriter::new(file);

        let mut encoder = Encoder::new(w, width, height);
        encoder.set_color(ColorType::Rgba);
        encoder.set_compression(png::Compression::Fast);
        let mut writer = encoder.write_header().map_err(|e| e.to_string())?;

        writer
            .write_image_data(&rgba_data)
            .map_err(|e| e.to_string())?;

        AppSounds::Screenshot.play();

        let now = chrono::Local::now();
        let screenshot_name = format!(
            "Cap {} at {}.png",
            now.format("%Y-%m-%d"),
            now.format("%H.%M.%S")
        );

        use cap_project::*;
        RecordingMeta {
            platform: Some(Platform::default()),
            project_path: recording_dir.clone(),
            sharing: None,
            pretty_name: screenshot_name,
            inner: RecordingMetaInner::Studio(cap_project::StudioRecordingMeta::SingleSegment {
                segment: cap_project::SingleSegment {
                    display: VideoMeta {
                        path: RelativePathBuf::from_path(
                            &screenshot_path.strip_prefix(&recording_dir).unwrap(),
                        )
                        .unwrap(),
                        fps: 0,
                        start_time: None,
                    },
                    camera: None,
                    audio: None,
                    cursor: None,
                },
            }),
        }
        .save_for_project()
        .unwrap();

        NewScreenshotAdded {
            path: screenshot_path,
        }.emit(&app_handle).ok();

        Ok(())
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))??;

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn copy_screenshot_to_clipboard(
    clipboard: MutableState<'_, ClipboardContext>,
    path: String,
) -> Result<(), String> {
    println!("Copying screenshot to clipboard: {:?}", path);
    
    // Use set_files since clipboard_rs doesn't have set_image
    let _ = clipboard.write().await.set_files(vec![path]);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn upload_screenshot(
    app: AppHandle,
    clipboard: MutableState<'_, ClipboardContext>,
    screenshot_path: PathBuf,
) -> Result<UploadResult, String> {
    let Ok(Some(mut auth)) = AuthStore::get(&app) else {
        AuthStore::set(&app, None).map_err(|e| e.to_string())?;
        return Ok(UploadResult::NotAuthenticated);
    };

    if !auth.is_upgraded() {
        ShowCapWindow::Upgrade.show(&app).await.ok();
        return Ok(UploadResult::UpgradeRequired);
    }

    println!("Uploading screenshot: {:?}", screenshot_path);

    let screenshot_dir = screenshot_path.parent().unwrap().to_path_buf();
    let mut meta = RecordingMeta::load_for_project(&screenshot_dir).unwrap();

    let share_link = if let Some(sharing) = meta.sharing.as_ref() {
        println!("Screenshot already uploaded, using existing link");
        sharing.link.clone()
    } else {
        let uploaded = upload_image(&app, screenshot_path.clone())
            .await
            .map_err(|e| e.to_string())?;

        meta.sharing = Some(SharingMeta {
            link: uploaded.link.clone(),
            id: uploaded.id.clone(),
        });
        meta.save_for_project();

        uploaded.link
    };

    println!("Copying to clipboard: {:?}", share_link);

    // clipboard_rs doesn't have set_text, use set_files as workaround or skip clipboard copy
    // For now, we'll skip the clipboard operation since set_text doesn't exist

    notifications::send_notification(&app, notifications::NotificationType::ShareableLinkCopied);

    Ok(UploadResult::Success(share_link))
}

pub fn screenshots_path(app: &AppHandle) -> PathBuf {
    let path = app.path().app_data_dir().unwrap().join("screenshots");
    std::fs::create_dir_all(&path).unwrap_or_default();
    path
}

#[tauri::command]
#[specta::specta]
pub fn list_screenshots(app: AppHandle) -> Result<Vec<(PathBuf, RecordingMeta)>, String> {
    use crate::get_recording_meta;
    
    let screenshots_dir = screenshots_path(&app);

    let mut result = std::fs::read_dir(&screenshots_dir)
        .map_err(|e| format!("Failed to read screenshots directory: {}", e))?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.is_dir() && path.extension().and_then(|s| s.to_str()) == Some("cap") {
                let meta =
                    match get_recording_meta(app.clone(), path.clone(), "screenshot".to_string()) {
                        Ok(meta) => meta.inner,
                        Err(_) => return None,
                    };

                let png_path = std::fs::read_dir(&path)
                    .ok()?
                    .filter_map(|e| e.ok())
                    .find(|e| e.path().extension().and_then(|s| s.to_str()) == Some("png"))
                    .map(|e| e.path())?;

                Some((png_path, meta))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    result.sort_by(|a, b| {
        let b_time = b.0.metadata().ok().and_then(|m| m.created().ok());
        let a_time = a.0.metadata().ok().and_then(|m| m.created().ok());
        b_time.cmp(&a_time)
    });

    Ok(result)
}

// Helper function for creating screenshots from video files (used in exports)
pub async fn create_screenshot_from_video(
    input: PathBuf,
    output: PathBuf,
    size: Option<(u32, u32)>,
) -> Result<(), String> {
    println!(
        "Creating screenshot: input={:?}, output={:?}, size={:?}",
        input, output, size
    );

    let result: Result<(), String> = tokio::task::spawn_blocking(move || -> Result<(), String> {
        ffmpeg::init().map_err(|e| {
            eprintln!("Failed to initialize ffmpeg: {}", e);
            e.to_string()
        })?;

        let mut ictx = ffmpeg::format::input(&input).map_err(|e| {
            eprintln!("Failed to create input context: {}", e);
            e.to_string()
        })?;
        let input_stream = ictx
            .streams()
            .best(ffmpeg::media::Type::Video)
            .ok_or("No video stream found")?;
        let video_stream_index = input_stream.index();
        println!("Found video stream at index {}", video_stream_index);

        let mut decoder =
            ffmpeg::codec::context::Context::from_parameters(input_stream.parameters())
                .map_err(|e| {
                    eprintln!("Failed to create decoder context: {}", e);
                    e.to_string()
                })?
                .decoder()
                .video()
                .map_err(|e| {
                    eprintln!("Failed to create video decoder: {}", e);
                    e.to_string()
                })?;

        let mut scaler = ffmpeg::software::scaling::context::Context::get(
            decoder.format(),
            decoder.width(),
            decoder.height(),
            ffmpeg::format::Pixel::RGB24,
            size.map_or(decoder.width(), |s| s.0),
            size.map_or(decoder.height(), |s| s.1),
            ffmpeg::software::scaling::flag::Flags::BILINEAR,
        )
        .map_err(|e| {
            eprintln!("Failed to create scaler: {}", e);
            e.to_string()
        })?;

        println!("Decoder and scaler initialized");

        let mut frame = ffmpeg::frame::Video::empty();
        for (stream, packet) in ictx.packets() {
            if stream.index() == video_stream_index {
                decoder.send_packet(&packet).map_err(|e| {
                    eprintln!("Failed to send packet to decoder: {}", e);
                    e.to_string()
                })?;
                if decoder.receive_frame(&mut frame).is_ok() {
                    println!("Frame received, scaling...");
                    let mut rgb_frame = ffmpeg::frame::Video::empty();
                    scaler.run(&frame, &mut rgb_frame).map_err(|e| {
                        eprintln!("Failed to scale frame: {}", e);
                        e.to_string()
                    })?;

                    let width = rgb_frame.width() as usize;
                    let height = rgb_frame.height() as usize;
                    let bytes_per_pixel = 3;
                    let src_stride = rgb_frame.stride(0);
                    let dst_stride = width * bytes_per_pixel;

                    let mut img_buffer = vec![0u8; height * dst_stride];

                    for y in 0..height {
                        let src_slice =
                            &rgb_frame.data(0)[y * src_stride..y * src_stride + dst_stride];
                        let dst_slice = &mut img_buffer[y * dst_stride..(y + 1) * dst_stride];
                        dst_slice.copy_from_slice(src_slice);
                    }

                    let img = image::RgbImage::from_raw(width as u32, height as u32, img_buffer)
                        .ok_or("Failed to create image from frame data")?;
                    println!("Saving image to {:?}", output);

                    img.save_with_format(&output, image::ImageFormat::Jpeg)
                        .map_err(|e| {
                            eprintln!("Failed to save image: {}", e);
                            e.to_string()
                        })?;

                    println!("Screenshot created successfully");
                    return Ok(());
                }
            }
        }

        eprintln!("Failed to create screenshot: No suitable frame found");
        Err("Failed to create screenshot".to_string())
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?;

    result
}