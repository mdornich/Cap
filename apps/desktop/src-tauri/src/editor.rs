use std::sync::Arc;
use std::path::PathBuf;
use std::str::FromStr;

use cap_editor::EditorState;
use cap_project::{ProjectConfiguration, RecordingMeta, XY};
use clipboard_rs::Clipboard;
use cap_rendering::ProjectRecordingsMeta;
use serde::Serialize;
use specta::Type;
use tauri::{AppHandle, Manager, State, Window};

use crate::{
    editor_window::{EditorInstances, WindowEditorInstance},
    windows::EditorWindowIds,
    CapWindowId, ClipboardContext, MutableState, notifications,
    audio,
};

#[derive(Serialize, Type, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SerializedEditorInstance {
    pub frames_socket_url: String,
    pub recording_duration: f64,
    pub saved_project_config: ProjectConfiguration,
    pub recordings: Arc<ProjectRecordingsMeta>,
    pub path: PathBuf,
}

#[derive(Serialize, specta::Type, tauri_specta::Event, Debug, Clone)]
pub struct EditorStateChanged {
    playhead_position: u32,
}

impl EditorStateChanged {
    pub fn new(s: &EditorState) -> Self {
        Self {
            playhead_position: s.playhead_position,
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn start_playback(
    editor_instance: WindowEditorInstance,
    fps: u32,
    resolution_base: XY<u32>,
) -> Result<(), String> {
    editor_instance.start_playback(fps, resolution_base).await;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn stop_playback(editor_instance: WindowEditorInstance) -> Result<(), String> {
    let mut state = editor_instance.state.lock().await;
    if let Some(handle) = state.playback_task.take() {
        handle.stop();
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn create_editor_instance(window: Window) -> Result<SerializedEditorInstance, String> {
    let CapWindowId::Editor { id } = CapWindowId::from_str(window.label()).unwrap() else {
        return Err("Invalid window".to_string());
    };

    let path = {
        let window_ids = EditorWindowIds::get(window.app_handle());
        let window_ids = window_ids.ids.lock().unwrap();

        let Some((path, _)) = window_ids.iter().find(|(_, _id)| *_id == id) else {
            return Err("Editor instance not found".to_string());
        };
        path.clone()
    };

    let editor_instance = EditorInstances::get_or_create(&window, path).await?;
    let meta = editor_instance.meta();
    println!("Pretty name: {}", meta.pretty_name);

    Ok(SerializedEditorInstance {
        frames_socket_url: format!("ws://localhost:{}", editor_instance.ws_port),
        recording_duration: editor_instance.recordings.duration(),
        saved_project_config: {
            let project_config = editor_instance.project_config.1.borrow();
            project_config.clone()
        },
        recordings: editor_instance.recordings.clone(),
        path: editor_instance.project_path.clone(),
    })
}

#[tauri::command]
#[specta::specta]
pub async fn get_editor_meta(editor: WindowEditorInstance) -> Result<RecordingMeta, String> {
    let path = editor.project_path.clone();
    RecordingMeta::load_for_project(&path).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn set_playhead_position(
    editor_instance: WindowEditorInstance,
    frame_number: u32,
) -> Result<(), String> {
    editor_instance
        .modify_and_emit_state(|state| {
            state.playhead_position = frame_number;
        })
        .await;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn seek_to(editor_instance: WindowEditorInstance, frame_number: u32) -> Result<(), String> {
    editor_instance
        .modify_and_emit_state(|state| {
            state.playhead_position = frame_number;
        })
        .await;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn get_mic_waveforms(editor_instance: WindowEditorInstance) -> Result<Vec<Vec<f32>>, String> {
    let mut out = Vec::new();

    for segment in editor_instance.segments.iter() {
        if let Some(audio) = &segment.audio {
            out.push(audio::get_waveform(audio));
        } else {
            out.push(Vec::new());
        }
    }

    Ok(out)
}

#[tauri::command]
#[specta::specta]
pub async fn copy_video_to_clipboard(
    app: AppHandle,
    clipboard: MutableState<'_, ClipboardContext>,
    path: String,
) -> Result<(), String> {
    println!("copying");
    let _ = clipboard.write().await.set_files(vec![path]);

    notifications::send_notification(
        &app,
        notifications::NotificationType::VideoCopiedToClipboard,
    );
    Ok(())
}