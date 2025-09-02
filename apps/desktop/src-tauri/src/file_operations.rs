use std::io::{BufReader, BufRead};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Command;

use mp4::Mp4Reader;
use tauri::{AppHandle, State};

use crate::{notifications, MutableState};

#[tauri::command]
#[specta::specta]
pub async fn copy_file_to_path(app: AppHandle, src: String, dst: String) -> Result<(), String> {
    let is_screenshot = src.contains("screenshots/");
    let is_gif = src.ends_with(".gif") || dst.ends_with(".gif");
    let is_srt = src.ends_with(".srt") || dst.ends_with(".srt");

    let src_path = std::path::Path::new(&src);
    if !src_path.exists() {
        return Err(format!("Source file {} does not exist", src));
    }

    if !is_screenshot && !is_gif && !is_srt {
        if !is_valid_mp4(src_path) {
            let mut attempts = 0;
            while attempts < 10 {
                std::thread::sleep(std::time::Duration::from_secs(1));
                if is_valid_mp4(src_path) {
                    break;
                }
                attempts += 1;
            }
            if attempts == 10 {
                return Err("Source video file is not a valid MP4".to_string());
            }
        }
    }

    if let Some(parent) = std::path::Path::new(&dst).parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("Failed to create target directory: {}", e))?;
    }

    let mut attempts = 0;
    const MAX_ATTEMPTS: u32 = 3;
    let mut last_error = None;

    while attempts < MAX_ATTEMPTS {
        match tokio::fs::copy(&src, &dst).await {
            Ok(bytes) => {
                let src_size = match tokio::fs::metadata(&src).await {
                    Ok(metadata) => metadata.len(),
                    Err(e) => {
                        last_error = Some(format!("Failed to get source file metadata: {}", e));
                        attempts += 1;
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        continue;
                    }
                };

                if bytes != src_size {
                    last_error = Some(format!(
                        "File copy verification failed: copied {} bytes but source is {} bytes",
                        bytes, src_size
                    ));
                    let _ = tokio::fs::remove_file(&dst).await;
                    attempts += 1;
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    continue;
                }

                if !is_screenshot && !is_gif && !is_srt && !is_valid_mp4(std::path::Path::new(&dst)) {
                    last_error = Some("Destination file is not a valid MP4".to_string());
                    let _ = tokio::fs::remove_file(&dst).await;
                    attempts += 1;
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    continue;
                }

                notifications::send_notification(
                    &app,
                    if is_screenshot {
                        notifications::NotificationType::ScreenshotSaved
                    } else {
                        notifications::NotificationType::VideoSaved
                    },
                );
                return Ok(());
            }
            Err(e) => {
                last_error = Some(e.to_string());
                attempts += 1;
                if attempts < MAX_ATTEMPTS {
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    continue;
                }
            }
        }
    }

    eprintln!(
        "Failed to copy file from {} to {} after {} attempts. Last error: {}",
        src,
        dst,
        MAX_ATTEMPTS,
        last_error.as_ref().unwrap()
    );

    notifications::send_notification(
        &app,
        if is_screenshot {
            notifications::NotificationType::ScreenshotSaveFailed
        } else {
            notifications::NotificationType::VideoSaveFailed
        },
    );

    Err(last_error.unwrap_or_else(|| "Maximum retry attempts exceeded".to_string()))
}

pub fn is_valid_mp4(path: &std::path::Path) -> bool {
    if let Ok(file) = std::fs::File::open(path) {
        let file_size = match file.metadata() {
            Ok(metadata) => metadata.len(),
            Err(_) => return false,
        };
        let reader = std::io::BufReader::new(file);
        Mp4Reader::read_header(reader, file_size).is_ok()
    } else {
        false
    }
}

#[tauri::command]
#[specta::specta]
pub async fn open_file_path(_app: AppHandle, path: PathBuf) -> Result<(), String> {
    let path_str = path.to_str().ok_or("Invalid path")?;

    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .args(["/select,", path_str])
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg("-R")
            .arg(path_str)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(
                path.parent()
                    .ok_or("Invalid path")?
                    .to_str()
                    .ok_or("Invalid path")?,
            )
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    Ok(())
}