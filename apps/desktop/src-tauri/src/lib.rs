mod audio;
mod audio_meter;
mod auth;
mod camera;
mod captions;
mod deeplink_actions;
mod editor;
mod editor_window;
mod export;
mod fake_window;
mod file_operations;
mod flags;
mod general_settings;
mod hotkeys;
mod notifications;
mod permissions;
mod platform;
mod screenshots;
mod presets;
mod recording;
mod system;
mod tray;
mod upload;
mod web_api;
mod windows;

use audio::AppSounds;
use auth::{AuthStore, AuthenticationInvalid, Plan};
use camera::create_camera_preview_ws;
use cap_editor::EditorInstance;
use cap_editor::EditorState;
use cap_media::feeds::RawCameraFrame;
use cap_media::feeds::{AudioInputFeed, AudioInputSamplesSender};
use cap_media::platform::Bounds;
use cap_media::{feeds::CameraFeed, sources::ScreenCaptureTarget};
use cap_project::RecordingMetaInner;
use cap_project::XY;
use cap_project::{ProjectConfiguration, RecordingMeta, SharingMeta, StudioRecordingMeta};
use cap_rendering::ProjectRecordingsMeta;
use clipboard_rs::common::RustImage;
use clipboard_rs::{Clipboard, ClipboardContext};
use editor_window::EditorInstances;
use editor_window::WindowEditorInstance;
use general_settings::GeneralSettingsStore;
use mp4::Mp4Reader;
use notifications::NotificationType;
use png::{ColorType, Encoder};
use recording::InProgressRecording;
use relative_path::RelativePathBuf;

use scap::capturer::Capturer;
use scap::frame::Frame;
use scap::frame::VideoFrame;
use serde::{Deserialize, Serialize};
use serde_json::json;
use specta::Type;
use std::collections::BTreeMap;
use std::{
    fs::File,
    future::Future,
    io::{BufReader, BufWriter},
    marker::PhantomData,
    path::PathBuf,
    process::Command,
    str::FromStr,
    sync::Arc,
};
use tauri::Window;
use tauri::{AppHandle, Manager, State, WindowEvent};
use tauri_plugin_deep_link::DeepLinkExt;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_notification::{NotificationExt, PermissionState};
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_shell::ShellExt;
use tauri_specta::Event;
use tokio::sync::{Mutex, RwLock};
use tracing::debug;
use tracing::error;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer;
use upload::{create_or_get_video, upload_image, upload_video, S3UploadMeta};
use web_api::ManagerExt as WebManagerExt;
use windows::set_window_transparent;
use windows::EditorWindowIds;
use windows::{CapWindowId, ShowCapWindow};

#[derive(specta::Type, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct App {
    #[serde(skip)]
    camera_tx: flume::Sender<RawCameraFrame>,
    camera_ws_port: u16,
    #[serde(skip)]
    camera_feed: Option<Arc<Mutex<CameraFeed>>>,
    #[serde(skip)]
    mic_feed: Option<AudioInputFeed>,
    #[serde(skip)]
    mic_samples_tx: AudioInputSamplesSender,
    #[serde(skip)]
    handle: AppHandle,
    #[serde(skip)]
    current_recording: Option<InProgressRecording>,
    #[serde(skip)]
    recording_logging_handle: LoggingHandle,
    server_url: String,
}

#[derive(specta::Type, Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub enum VideoType {
    Screen,
    Output,
    Camera,
}

#[derive(Serialize, Deserialize, specta::Type, Debug)]
pub enum UploadResult {
    Success(String),
    NotAuthenticated,
    PlanCheckFailed,
    UpgradeRequired,
}

#[derive(Serialize, Deserialize, specta::Type, Debug)]
pub struct VideoRecordingMetadata {
    pub duration: f64,
    pub size: f64,
}

#[derive(Clone, Serialize, Deserialize, specta::Type, Debug)]
pub struct VideoUploadInfo {
    id: String,
    link: String,
    config: S3UploadMeta,
}

impl App {
    pub fn set_current_recording(&mut self, actor: InProgressRecording) {
        self.current_recording = Some(actor);

        CurrentRecordingChanged.emit(&self.handle).ok();
    }

    pub fn clear_current_recording(&mut self) -> Option<InProgressRecording> {
        self.close_occluder_windows();

        self.current_recording.take()
    }

    fn close_occluder_windows(&self) {
        for window in self.handle.webview_windows() {
            if window.0.starts_with("window-capture-occluder-") {
                let _ = window.1.close();
            }
        }
    }
}

#[tauri::command]
#[specta::specta]
async fn set_mic_input(state: MutableState<'_, App>, label: Option<String>) -> Result<(), String> {
    let mut app = state.write().await;

    match (label, &mut app.mic_feed) {
        (Some(label), None) => {
            AudioInputFeed::init(&label)
                .await
                .map_err(|e| e.to_string())
                .map(async |feed| {
                    feed.add_sender(app.mic_samples_tx.clone()).await.unwrap();
                    app.mic_feed = Some(feed);
                })
                .transpose_async()
                .await
        }
        (Some(label), Some(feed)) => feed.switch_input(&label).await.map_err(|e| e.to_string()),
        (None, _) => {
            debug!("removing mic in set_start_recording_options");
            app.mic_feed.take();
            Ok(())
        }
    }
}

#[tauri::command]
#[specta::specta]
async fn set_camera_input(
    state: MutableState<'_, App>,
    label: Option<String>,
) -> Result<bool, String> {
    let mut app = state.write().await;

    match (&label, app.camera_feed.as_ref()) {
        (Some(label), Some(camera_feed)) => {
            camera_feed
                .lock()
                .await
                .switch_cameras(label)
                .await
                .map_err(|e| e.to_string())?;
            Ok(true)
        }
        (Some(label), None) => {
            let camera_tx = app.camera_tx.clone();
            drop(app);

            let init_rx = CameraFeed::init_async(label);

            loop {
                tokio::select! {
                    result = init_rx.recv_async() => {
                        match result {
                            Ok(Ok(feed)) => {
                                let mut app = state.write().await;
                                if app.camera_feed.is_none() {
                                    feed.attach(camera_tx);
                                    app.camera_feed = Some(Arc::new(Mutex::new(feed)));
                                    return Ok(true);
                                } else {
                                    return Ok(false);
                                }
                            }
                            Ok(Err(e)) => return Err(e.to_string()),
                            Err(_) => return Ok(false),
                        }
                    }
                    _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
                        let app = state.read().await;

                        if app.camera_feed.is_some() {
                            return Ok(false);
                        }
                    }
                }
            }
        }
        (None, _) => {
            app.camera_feed.take();
            Ok(true)
        }
    }
}

#[derive(specta::Type, Serialize, tauri_specta::Event, Clone)]
pub struct RecordingOptionsChanged;

#[derive(Deserialize, specta::Type, Serialize, tauri_specta::Event, Debug, Clone)]
pub struct NewStudioRecordingAdded {
    path: PathBuf,
}

#[derive(Deserialize, specta::Type, Serialize, tauri_specta::Event, Debug, Clone)]
pub struct NewScreenshotAdded {
    path: PathBuf,
}

#[derive(Deserialize, specta::Type, Serialize, tauri_specta::Event, Debug, Clone)]
pub struct RecordingStarted;

#[derive(Deserialize, specta::Type, Serialize, tauri_specta::Event, Debug, Clone)]
pub struct RecordingStopped;

#[derive(Deserialize, specta::Type, Serialize, tauri_specta::Event, Debug, Clone)]
pub struct RequestStartRecording;

#[derive(Deserialize, specta::Type, Serialize, tauri_specta::Event, Debug, Clone)]
pub struct RequestNewScreenshot;

#[derive(Deserialize, specta::Type, Serialize, tauri_specta::Event, Debug, Clone)]
pub struct RequestOpenSettings {
    page: String,
}

#[derive(Deserialize, specta::Type, Serialize, tauri_specta::Event, Debug, Clone)]
pub struct NewNotification {
    title: String,
    body: String,
    is_error: bool,
}

type ArcLock<T> = Arc<RwLock<T>>;
pub type MutableState<'a, T> = State<'a, Arc<RwLock<T>>>;

type SingleTuple<T> = (T,);

#[derive(Serialize, Type)]
struct JsonValue<T>(
    #[serde(skip)] PhantomData<T>,
    #[specta(type = SingleTuple<T>)] serde_json::Value,
);

impl<T> Clone for JsonValue<T> {
    fn clone(&self) -> Self {
        Self(PhantomData, self.1.clone())
    }
}

impl<T: Serialize> JsonValue<T> {
    fn new(value: &T) -> Self {
        Self(PhantomData, json!(value))
    }
}

#[derive(Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RecordingInfo {
    capture_target: ScreenCaptureTarget,
}

#[derive(Serialize, Type)]
#[serde(rename_all = "camelCase")]
enum CurrentRecordingTarget {
    Window { id: u32, bounds: Bounds },
    Screen { id: u32 },
    Area { screen: u32, bounds: Bounds },
}

#[derive(Serialize, Type)]
#[serde(rename_all = "camelCase")]
struct CurrentRecording {
    target: CurrentRecordingTarget,
    r#type: RecordingType,
}

#[tauri::command]
#[specta::specta]
async fn get_current_recording(
    state: MutableState<'_, App>,
) -> Result<JsonValue<Option<CurrentRecording>>, ()> {
    let state = state.read().await;
    Ok(JsonValue::new(&state.current_recording.as_ref().map(|r| {
        let bounds = r.bounds();

        let target = match r.capture_target() {
            ScreenCaptureTarget::Screen { id } => CurrentRecordingTarget::Screen { id: *id },
            ScreenCaptureTarget::Window { id } => CurrentRecordingTarget::Window {
                id: *id,
                bounds: bounds.clone(),
            },
            ScreenCaptureTarget::Area { screen, bounds } => CurrentRecordingTarget::Area {
                screen: *screen,
                bounds: bounds.clone(),
            },
        };

        CurrentRecording {
            target,
            r#type: match r {
                InProgressRecording::Instant { .. } => RecordingType::Instant,
                InProgressRecording::Studio { .. } => RecordingType::Studio,
            },
        }
    })))
}

#[derive(Serialize, Type, tauri_specta::Event, Clone)]
pub struct CurrentRecordingChanged;


#[derive(Deserialize, specta::Type, tauri_specta::Event, Debug, Clone)]
struct RenderFrameEvent {
    frame_number: u32,
    fps: u32,
    resolution_base: XY<u32>,
}




#[derive(Serialize, Deserialize, specta::Type, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
pub struct FramesRendered {
    rendered_count: u32,
    total_frames: u32,
}

#[tauri::command]
#[specta::specta]
async fn set_project_config(
    editor_instance: WindowEditorInstance,
    config: ProjectConfiguration,
) -> Result<(), String> {
    config.write(&editor_instance.project_path).unwrap();

    editor_instance.project_config.0.send(config).ok();

    Ok(())
}

#[tauri::command]
#[specta::specta]
async fn list_audio_devices() -> Result<Vec<String>, ()> {
    if !permissions::do_permissions_check(false)
        .microphone
        .permitted()
    {
        return Ok(vec![]);
    }

    Ok(AudioInputFeed::list_devices().keys().cloned().collect())
}

#[derive(Serialize, Type, tauri_specta::Event, Debug, Clone)]
pub struct UploadProgress {
    progress: f64,
}

#[derive(Deserialize, Type)]
pub enum UploadMode {
    Initial {
        pre_created_video: Option<VideoUploadInfo>,
    },
    Reupload,
}

#[tauri::command]
#[specta::specta]
async fn upload_exported_video(
    app: AppHandle,
    path: PathBuf,
    mode: UploadMode,
) -> Result<UploadResult, String> {
    let Ok(Some(auth)) = AuthStore::get(&app) else {
        AuthStore::set(&app, None).map_err(|e| e.to_string())?;
        return Ok(UploadResult::NotAuthenticated);
    };

    let screen_metadata = system::get_video_metadata(path.clone()).await.map_err(|e| {
        sentry::capture_message(
            &format!("Failed to get video metadata: {}", e),
            sentry::Level::Error,
        );

        "Failed to read video metadata. The recording may be from an incompatible version."
            .to_string()
    })?;

    let camera_metadata = system::get_video_metadata(path.clone()).await.ok();

    let duration = screen_metadata.duration.max(
        camera_metadata
            .map(|m| m.duration)
            .unwrap_or(screen_metadata.duration),
    );

    if !auth.is_upgraded() && duration > 300.0 {
        return Ok(UploadResult::UpgradeRequired);
    }

    let mut meta = RecordingMeta::load_for_project(&path).map_err(|v| v.to_string())?;

    let output_path = meta.output_path();
    if !output_path.exists() {
        notifications::send_notification(&app, notifications::NotificationType::UploadFailed);
        return Err("Failed to upload video: Rendered video not found".to_string());
    }

    UploadProgress { progress: 0.0 }.emit(&app).ok();

    let s3_config = async {
        let video_id = match mode {
            UploadMode::Initial { pre_created_video } => {
                if let Some(pre_created) = pre_created_video {
                    return Ok(pre_created.config);
                }
                None
            }
            UploadMode::Reupload => {
                let Some(sharing) = meta.sharing.clone() else {
                    return Err("No sharing metadata found".to_string());
                };

                Some(sharing.id)
            }
        };

        create_or_get_video(&app, false, video_id, Some(meta.pretty_name.clone())).await
    }
    .await?;

    let upload_id = s3_config.id().to_string();

    match upload_video(
        &app,
        upload_id.clone(),
        output_path,
        Some(s3_config),
        Some(meta.project_path.join("screenshots/display.jpg")),
    )
    .await
    {
        Ok(uploaded_video) => {
            UploadProgress { progress: 1.0 }.emit(&app).ok();

            meta.sharing = Some(SharingMeta {
                link: uploaded_video.link.clone(),
                id: uploaded_video.id.clone(),
            });
            meta.save_for_project().ok();

            let _ = app
                .state::<ArcLock<ClipboardContext>>()
                .write()
                .await
                .set_text(uploaded_video.link.clone());

            NotificationType::ShareableLinkCopied.send(&app);
            Ok(UploadResult::Success(uploaded_video.link))
        }
        Err(e) => {
            error!("Failed to upload video: {e}");

            NotificationType::UploadFailed.send(&app);
            Err(e)
        }
    }
}

#[tauri::command]
#[specta::specta]
async fn save_file_dialog(
    app: AppHandle,
    file_name: String,
    file_type: String,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    println!(
        "save_file_dialog called with file_name: {}, file_type: {}",
        file_name, file_type
    );

    let file_name = file_name
        .strip_suffix(".cap")
        .unwrap_or(&file_name)
        .to_string();
    println!("File name after removing .cap suffix: {}", file_name);

    let (name, extension) = match file_type.as_str() {
        "recording" => {
            println!("File type is recording");
            ("MP4 Video", "mp4")
        }
        "screenshot" => {
            println!("File type is screenshot");
            ("PNG Image", "png")
        }
        _ => {
            println!("Invalid file type: {}", file_type);
            return Err("Invalid file type".to_string());
        }
    };

    println!(
        "Showing save dialog with name: {}, extension: {}",
        name, extension
    );

    let (tx, rx) = std::sync::mpsc::channel();
    println!("Created channel for communication");

    app.dialog()
        .file()
        .set_title("Save File")
        .set_file_name(file_name)
        .add_filter(name, &[extension])
        .save_file(move |path| {
            println!("Save file callback triggered");
            let _ = tx.send(
                path.as_ref()
                    .and_then(|p| p.as_path())
                    .map(|p| p.to_string_lossy().to_string()),
            );
        });

    println!("Waiting for user selection");
    match rx.recv() {
        Ok(result) => {
            println!("Save dialog result: {:?}", result);
            Ok(result)
        }
        Err(e) => {
            println!("Error receiving result: {}", e);
            notifications::send_notification(
                &app,
                notifications::NotificationType::VideoSaveFailed,
            );
            Err(e.to_string())
        }
    }
}

#[derive(Serialize, specta::Type)]
pub struct RecordingMetaWithType {
    #[serde(flatten)]
    pub inner: RecordingMeta,
    pub r#type: RecordingType,
}

impl RecordingMetaWithType {
    fn new(inner: RecordingMeta) -> Self {
        Self {
            r#type: match &inner.inner {
                RecordingMetaInner::Studio(_) => RecordingType::Studio,
                RecordingMetaInner::Instant(_) => RecordingType::Instant,
            },
            inner,
        }
    }
}

#[derive(Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum RecordingType {
    Studio,
    Instant,
}

#[tauri::command(async)]
#[specta::specta]
fn get_recording_meta(
    app: AppHandle,
    path: PathBuf,
    file_type: String,
) -> Result<RecordingMetaWithType, String> {
    RecordingMeta::load_for_project(&path)
        .map(RecordingMetaWithType::new)
        .map_err(|e| format!("Failed to load recording meta: {}", e))
}

#[tauri::command]
#[specta::specta]
fn list_recordings(app: AppHandle) -> Result<Vec<(PathBuf, RecordingMetaWithType)>, String> {
    let recordings_dir = recordings_path(&app);

    if !recordings_dir.exists() {
        return Ok(Vec::new());
    }

    let mut result = std::fs::read_dir(&recordings_dir)
        .map_err(|e| format!("Failed to read recordings directory: {}", e))?
        .filter_map(|entry| {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => return None,
            };

            let path = entry.path();

            if !path.is_dir() {
                return None;
            }

            get_recording_meta(app.clone(), path.clone(), "recording".to_string())
                .ok()
                .map(|meta| (path, meta))
        })
        .collect::<Vec<_>>();

    result.sort_by(|a, b| {
        let b_time =
            b.0.metadata()
                .and_then(|m| m.created())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

        let a_time =
            a.0.metadata()
                .and_then(|m| m.created())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

        b_time.cmp(&a_time)
    });

    Ok(result)
}

#[tauri::command]
#[specta::specta]
async fn check_upgraded_and_update(app: AppHandle) -> Result<bool, String> {
    println!("Checking upgraded status and updating...");

    if let Ok(Some(settings)) = GeneralSettingsStore::get(&app) {
        if settings.commercial_license.is_some() {
            return Ok(true);
        }
    }

    let Ok(Some(auth)) = AuthStore::get(&app) else {
        println!("No auth found, clearing auth store");
        AuthStore::set(&app, None).map_err(|e| e.to_string())?;
        return Ok(false);
    };

    if let Some(ref plan) = auth.plan {
        if plan.manual {
            return Ok(true);
        }
    }

    println!(
        "Fetching plan for user {}",
        auth.user_id.as_deref().unwrap_or("unknown")
    );
    let response = app
        .authed_api_request("/api/desktop/plan", |client, url| client.get(url))
        .await
        .map_err(|e| {
            println!("Failed to fetch plan: {}", e);
            format!("Failed to fetch plan: {}", e)
        })?;

    println!("Plan fetch response status: {}", response.status());
    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        println!("Unauthorized response, clearing auth store");
        AuthStore::set(&app, None).map_err(|e| e.to_string())?;
        return Ok(false);
    }

    let plan_data = response.json::<serde_json::Value>().await.map_err(|e| {
        println!("Failed to parse plan response: {}", e);
        format!("Failed to parse plan response: {}", e)
    })?;

    let is_pro = plan_data
        .get("upgraded")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    println!("Pro status: {}", is_pro);
    let updated_auth = AuthStore {
        secret: auth.secret,
        user_id: auth.user_id,
        intercom_hash: auth.intercom_hash,
        plan: Some(Plan {
            upgraded: is_pro,
            manual: auth.plan.map(|p| p.manual).unwrap_or(false),
            last_checked: chrono::Utc::now().timestamp() as i32,
        }),
    };
    println!("Updating auth store with new pro status");
    AuthStore::set(&app, Some(updated_auth)).map_err(|e| e.to_string())?;

    Ok(is_pro)
}

#[tauri::command]
#[specta::specta]
fn open_external_link(app: tauri::AppHandle, url: String) -> Result<(), String> {
    if let Ok(Some(settings)) = GeneralSettingsStore::get(&app) {
        if settings.disable_auto_open_links {
            return Ok(());
        }
    }

    app.shell()
        .open(&url, None)
        .map_err(|e| format!("Failed to open URL: {}", e))?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
async fn reset_camera_permissions(_app: AppHandle) -> Result<(), ()> {
    #[cfg(target_os = "macos")]
    {
        #[cfg(debug_assertions)]
        let bundle_id =
            std::env::var("CAP_BUNDLE_ID").unwrap_or_else(|_| "com.apple.Terminal".to_string());
        #[cfg(not(debug_assertions))]
        let bundle_id = "com.klip.app";

        Command::new("tccutil")
            .arg("reset")
            .arg("Camera")
            .arg(bundle_id)
            .output()
            .expect("Failed to reset camera permissions");
    }

    Ok(())
}

#[tauri::command]
#[specta::specta]
async fn reset_microphone_permissions(_app: AppHandle) -> Result<(), ()> {
    #[cfg(target_os = "macos")]
    {
        #[cfg(debug_assertions)]
        let bundle_id =
            std::env::var("CAP_BUNDLE_ID").unwrap_or_else(|_| "com.apple.Terminal".to_string());
        #[cfg(not(debug_assertions))]
        let bundle_id = "com.klip.app";

        Command::new("tccutil")
            .arg("reset")
            .arg("Microphone")
            .arg(bundle_id)
            .output()
            .expect("Failed to reset microphone permissions");
    }

    Ok(())
}





#[tauri::command(async)]
#[specta::specta]
fn set_fail(name: String, value: bool) {
    cap_fail::set_fail(&name, value)
}

async fn check_notification_permissions(app: AppHandle) {
    let Ok(Some(settings)) = GeneralSettingsStore::get(&app) else {
        return;
    };

    if !settings.enable_notifications {
        return;
    }

    match app.notification().permission_state() {
        Ok(state) if state != PermissionState::Granted => {
            println!("Requesting notification permission");
            match app.notification().request_permission() {
                Ok(PermissionState::Granted) => {
                    println!("Notification permission granted");
                }
                Ok(_) | Err(_) => {
                    GeneralSettingsStore::update(&app, |s| {
                        s.enable_notifications = false;
                    })
                    .ok();
                }
            }
        }
        Ok(_) => {
            println!("Notification permission already granted");
        }
        Err(e) => {
            eprintln!("Error checking notification permission state: {}", e);
        }
    }
}

fn configure_logging(folder: &PathBuf) -> tracing_appender::non_blocking::WorkerGuard {
    let file_appender = tracing_appender::rolling::daily(folder, "cap-logs.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let filter = || tracing_subscriber::filter::EnvFilter::builder().parse_lossy("cap-*=TRACE");

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_target(false)
                .with_writer(non_blocking)
                .with_filter(filter()),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(true)
                .with_target(false)
                .with_filter(filter()),
        )
        .init();

    _guard
}

#[tauri::command]
#[specta::specta]
async fn set_server_url(app: MutableState<'_, App>, server_url: String) -> Result<(), ()> {
    app.write().await.server_url = server_url;
    Ok(())
}

#[tauri::command]
#[specta::specta]
async fn update_auth_plan(app: AppHandle) {
    AuthStore::update_auth_plan(&app).await.ok();
}

pub type FilteredRegistry = tracing_subscriber::layer::Layered<
    tracing_subscriber::filter::FilterFn<fn(m: &tracing::Metadata) -> bool>,
    tracing_subscriber::Registry,
>;

pub type DynLoggingLayer = Box<dyn tracing_subscriber::Layer<FilteredRegistry> + Send + Sync>;
type LoggingHandle = tracing_subscriber::reload::Handle<Option<DynLoggingLayer>, FilteredRegistry>;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run(recording_logging_handle: LoggingHandle) {
    let tauri_context = tauri::generate_context!();

    let specta_builder = tauri_specta::Builder::new()
        .commands(tauri_specta::collect_commands![
            set_mic_input,
            set_camera_input,
            recording::start_recording,
            recording::stop_recording,
            recording::pause_recording,
            recording::resume_recording,
            recording::restart_recording,
            recording::delete_recording,
            recording::list_cameras,
            recording::list_capture_windows,
            recording::list_capture_screens,
            screenshots::take_screenshot,
            list_audio_devices,
            system::close_recordings_overlay_window,
            fake_window::set_fake_window_bounds,
            fake_window::remove_fake_window,
            system::focus_captures_panel,
            get_current_recording,
            export::export_video,
            export::get_export_estimates,
            file_operations::copy_file_to_path,
            editor::copy_video_to_clipboard,
            screenshots::copy_screenshot_to_clipboard,
            file_operations::open_file_path,
            system::get_video_metadata,
            editor::create_editor_instance,
            editor::get_mic_waveforms,
            system::get_system_audio_waveforms,
            editor::start_playback,
            editor::stop_playback,
            editor::set_playhead_position,
            set_project_config,
            permissions::open_permission_settings,
            permissions::do_permissions_check,
            permissions::request_permission,
            upload_exported_video,
            screenshots::upload_screenshot,
            get_recording_meta,
            save_file_dialog,
            delete_wallpaper,
            list_recordings,
            screenshots::list_screenshots,
            check_upgraded_and_update,
            open_external_link,
            hotkeys::set_hotkey,
            reset_camera_permissions,
            reset_microphone_permissions,
            system::is_camera_window_open,
            editor::seek_to,
            windows::position_traffic_lights,
            windows::set_theme,
            global_message_dialog,
            system::show_window,
            write_clipboard_string,
            platform::perform_haptic_feedback,
            system::list_fails,
            set_fail,
            update_auth_plan,
            set_window_transparent,
            editor::get_editor_meta,
            set_server_url,
            captions::create_dir,
            captions::save_model_file,
            captions::transcribe_audio,
            captions::save_captions,
            captions::load_captions,
            captions::download_whisper_model,
            captions::check_model_exists,
            captions::delete_whisper_model,
            captions::export_captions_srt,
            general_settings::set_instant_save_path,
            general_settings::get_instant_save_path
        ])
        .events(tauri_specta::collect_events![
            RecordingOptionsChanged,
            NewStudioRecordingAdded,
            NewScreenshotAdded,
            RenderFrameEvent,
            editor::EditorStateChanged,
            CurrentRecordingChanged,
            RecordingStarted,
            RecordingStopped,
            RequestStartRecording,
            RequestNewScreenshot,
            RequestOpenSettings,
            NewNotification,
            AuthenticationInvalid,
            audio_meter::AudioInputLevelChange,
            UploadProgress,
            captions::DownloadProgress,
        ])
        .error_handling(tauri_specta::ErrorHandlingMode::Throw)
        .typ::<ProjectConfiguration>()
        .typ::<AuthStore>()
        .typ::<presets::PresetsStore>()
        .typ::<hotkeys::HotkeysStore>()
        .typ::<general_settings::GeneralSettingsStore>()
        .typ::<cap_flags::Flags>();

    #[cfg(debug_assertions)]
    specta_builder
        .export(
            specta_typescript::Typescript::default(),
            "../src/utils/tauri.ts",
        )
        .expect("Failed to export typescript bindings");

    let (camera_tx, camera_ws_port, _shutdown) = create_camera_preview_ws().await;

    let (audio_input_tx, audio_input_rx) = AudioInputFeed::create_channel();

    tauri::async_runtime::set(tokio::runtime::Handle::current());

    #[allow(unused_mut)]
    let mut builder =
        tauri::Builder::default().plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            let Some(cap_file) = args
                .iter()
                .find(|arg| arg.ends_with(".cap"))
                .map(PathBuf::from)
            else {
                let _ = ShowCapWindow::Main.show(app);
                return;
            };

            let _ = open_project_from_path(&cap_file, app.clone());
        }));

    #[cfg(target_os = "macos")]
    {
        builder = builder.plugin(tauri_nspanel::init());
    }

    builder
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_oauth::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(flags::plugin::init())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_window_state::Builder::new()
                .with_state_flags({
                    use tauri_plugin_window_state::StateFlags;
                    let mut flags = StateFlags::all();
                    flags.remove(StateFlags::VISIBLE);
                    flags
                })
                .with_denylist(&[
                    CapWindowId::Setup.label().as_str(),
                    "window-capture-occluder",
                    CapWindowId::CaptureArea.label().as_str(),
                    CapWindowId::Camera.label().as_str(),
                    CapWindowId::RecordingsOverlay.label().as_str(),
                    CapWindowId::InProgressRecording.label().as_str(),
                    CapWindowId::Upgrade.label().as_str(),
                ])
                .map_label(|label| match label {
                    label if label.starts_with("editor-") => "editor",
                    label if label.starts_with("window-capture-occluder-") => {
                        "window-capture-occluder"
                    }
                    _ => label,
                })
                .build(),
        )
        .invoke_handler(specta_builder.invoke_handler())
        .setup(move |app| {
            let app = app.handle().clone();
            specta_builder.mount_events(&app);
            hotkeys::init(&app);
            general_settings::init(&app);
            fake_window::init(&app);
            app.manage(EditorWindowIds::default());

            if let Ok(Some(auth)) = AuthStore::load(&app) {
                sentry::configure_scope(|scope| {
                    scope.set_user(auth.user_id.map(|id| sentry::User {
                        id: Some(id),
                        ..Default::default()
                    }));
                });
            }

            {
                app.manage(Arc::new(RwLock::new(App {
                    handle: app.clone(),
                    camera_tx,
                    camera_ws_port,
                    camera_feed: None,
                    mic_samples_tx: audio_input_tx,
                    mic_feed: None,
                    current_recording: None,
                    recording_logging_handle,
                    server_url: GeneralSettingsStore::get(&app)
                        .ok()
                        .flatten()
                        .map(|v| v.server_url.clone())
                        .unwrap_or_else(|| {
                            std::option_env!("VITE_SERVER_URL")
                                .unwrap_or("https://cap.so")
                                .to_string()
                        }),
                })));

                app.manage(Arc::new(RwLock::new(
                    ClipboardContext::new().expect("Failed to create clipboard context"),
                )));
            }

            tokio::spawn(check_notification_permissions(app.clone()));

            println!("Checking startup completion and permissions...");
            let permissions = permissions::do_permissions_check(false);
            println!("Permissions check result: {:?}", permissions);

            tokio::spawn({
                let app = app.clone();
                async move {
                    if !permissions.screen_recording.permitted()
                        || !permissions.accessibility.permitted()
                        || GeneralSettingsStore::get(&app)
                            .ok()
                            .flatten()
                            .map(|s| !s.has_completed_startup)
                            .unwrap_or(false)
                    {
                        let _ = ShowCapWindow::Setup.show(&app).await;
                    } else {
                        println!("Permissions granted, showing main window");

                        let _ = ShowCapWindow::Main.show(&app).await;
                    }
                }
            });

            audio_meter::spawn_event_emitter(app.clone(), audio_input_rx);

            tray::create_tray(&app).unwrap();

            RequestNewScreenshot::listen_any_spawn(&app, |_, app| async move {
                if let Err(e) = screenshots::take_screenshot(app.clone(), app.state()).await {
                    eprintln!("Failed to take screenshot: {}", e);
                }
            });

            RequestOpenSettings::listen_any_spawn(&app, |payload, app| async move {
                let _ = ShowCapWindow::Settings {
                    page: Some(payload.page),
                }
                .show(&app)
                .await;
            });

            let app_handle = app.clone();
            app.deep_link().on_open_url(move |event| {
                deeplink_actions::handle(&app_handle, event.urls());
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            let label = window.label();
            let app = window.app_handle();

            match event {
                WindowEvent::Destroyed => {
                    if let Ok(window_id) = CapWindowId::from_str(label) {
                        match window_id {
                            CapWindowId::Main => {
                                let app = app.clone();
                                tokio::spawn(async move {
                                    let state = app.state::<Arc<RwLock<App>>>();
                                    let app_state = &mut *state.write().await;

                                    if app_state.current_recording.is_none() {
                                        app_state.mic_feed.take();
                                        app_state.camera_feed.take();

                                        if let Some(camera) = CapWindowId::Camera.get(&app) {
                                            let _ = camera.close();
                                        }
                                    }
                                });
                            }
                            CapWindowId::Editor { id } => {
                                let window_ids = EditorWindowIds::get(window.app_handle());
                                window_ids.ids.lock().unwrap().retain(|(_, _id)| *_id != id);

                                tokio::spawn(EditorInstances::remove(window.clone()));
                            }
                            CapWindowId::Settings
                            | CapWindowId::Upgrade
                            | CapWindowId::ModeSelect => {
                                if let Some(window) = CapWindowId::Main.get(&app) {
                                    let _ = window.show();
                                }
                                return;
                            }
                            _ => {}
                        };
                    }

                    if let Some(settings) = GeneralSettingsStore::get(app).unwrap_or(None) {
                        if settings.hide_dock_icon
                            && app.webview_windows().keys().all(|label| {
                                !CapWindowId::from_str(label).unwrap().activates_dock()
                            })
                        {
                            #[cfg(target_os = "macos")]
                            app.set_activation_policy(tauri::ActivationPolicy::Accessory)
                                .ok();
                        }
                    }
                }
                #[cfg(target_os = "macos")]
                WindowEvent::Focused(focused) if *focused => {
                    if let Ok(window_id) = CapWindowId::from_str(label) {
                        if window_id.activates_dock() {
                            app.set_activation_policy(tauri::ActivationPolicy::Regular)
                                .ok();
                        }
                    }
                }
                WindowEvent::DragDrop(event) => {
                    if let tauri::DragDropEvent::Drop { paths, .. } = event {
                        for path in paths {
                            let _ = open_project_from_path(path, app.clone());
                        }
                    }
                }
                _ => {}
            }
        })
        .build(tauri_context)
        .expect("error while running tauri application")
        .run(|handle, event| match event {
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Reopen { .. } => {
                let has_window = handle.webview_windows().iter().any(|(label, _)| {
                    label.starts_with("editor-")
                        || label.as_str() == "settings"
                        || label.as_str() == "signin"
                });

                if has_window {
                    if let Some(window) = handle
                        .webview_windows()
                        .iter()
                        .find(|(label, _)| {
                            label.starts_with("editor-")
                                || label.as_str() == "settings"
                                || label.as_str() == "signin"
                        })
                        .map(|(_, window)| window.clone())
                    {
                        window.set_focus().ok();
                    }
                } else {
                    let handle = handle.clone();
                    let _ = tokio::spawn(async move { ShowCapWindow::Main.show(&handle).await });
                }
            }
            tauri::RunEvent::ExitRequested { code, api, .. } => {
                if code.is_none() {
                    api.prevent_exit();
                }
            }
            _ => {}
        });
}

async fn create_editor_instance_impl(
    app: &AppHandle,
    path: PathBuf,
) -> Result<Arc<EditorInstance>, String> {
    let app = app.clone();

    let instance = EditorInstance::new(path, {
        let app = app.clone();
        move |state| {
            editor::EditorStateChanged::new(state).emit(&app).ok();
        }
    })
    .await?;

    RenderFrameEvent::listen_any(&app, {
        let preview_tx = instance.preview_tx.clone();
        move |e| {
            preview_tx
                .send(Some((
                    e.payload.frame_number,
                    e.payload.fps,
                    e.payload.resolution_base,
                )))
                .ok();
        }
    });

    Ok(instance)
}

fn recordings_path(app: &AppHandle) -> PathBuf {
    let path = app.path().app_data_dir().unwrap().join("recordings");
    std::fs::create_dir_all(&path).unwrap_or_default();
    path
}

fn recording_path(app: &AppHandle, recording_id: &str) -> PathBuf {
    recordings_path(app).join(format!("{}.cap", recording_id))
}

#[tauri::command]
#[specta::specta]
fn global_message_dialog(app: AppHandle, message: String) {
    app.dialog().message(message).show(|_| {});
}

#[tauri::command]
#[specta::specta]
async fn write_clipboard_string(
    clipboard: MutableState<'_, ClipboardContext>,
    text: String,
) -> Result<(), String> {
    let writer = clipboard
        .try_write()
        .map_err(|e| format!("Failed to acquire lock on clipboard state: {e}"))?;
    writer
        .set_text(text)
        .map_err(|e| format!("Failed to write text to clipboard: {e}"))
}

trait EventExt: tauri_specta::Event {
    fn listen_any_spawn<Fut>(
        app: &AppHandle,
        handler: impl Fn(Self, AppHandle) -> Fut + Send + 'static + Clone,
    ) -> tauri::EventId
    where
        Fut: Future + Send,
        Self: serde::de::DeserializeOwned + Send + 'static,
    {
        let app = app.clone();
        Self::listen_any(&app.clone(), move |e| {
            let app = app.clone();
            let handler = handler.clone();
            tokio::spawn(async move {
                (handler)(e.payload, app).await;
            });
        })
    }
}

impl<T: tauri_specta::Event> EventExt for T {}

trait TransposeAsync {
    type Output;

    fn transpose_async(self) -> impl Future<Output = Self::Output>
    where
        Self: Sized;
}

impl<F: Future<Output = T>, T, E> TransposeAsync for Result<F, E> {
    type Output = Result<T, E>;

    fn transpose_async(self) -> impl Future<Output = Self::Output>
    where
        Self: Sized,
    {
        async {
            match self {
                Ok(f) => Ok(f.await),
                Err(e) => Err(e),
            }
        }
    }
}

fn open_project_from_path(path: &PathBuf, app: AppHandle) -> Result<(), String> {
    let meta = RecordingMeta::load_for_project(path).map_err(|v| v.to_string())?;

    match &meta.inner {
        RecordingMetaInner::Studio(_) => {
            let project_path = path.clone();

            tokio::spawn(async move { ShowCapWindow::Editor { project_path }.show(&app).await });
        }
        RecordingMetaInner::Instant(_) => {
            let mp4_path = path.join("content/output.mp4");

            if mp4_path.exists() && mp4_path.is_file() {
                let _ = app
                    .opener()
                    .open_path(mp4_path.to_str().unwrap_or_default(), None::<String>);
                if let Some(main_window) = CapWindowId::Main.get(&app) {
                    main_window.close().ok();
                }
            }
        }
    }

    Ok(())
}

#[tauri::command]
#[specta::specta]
async fn delete_wallpaper(app: AppHandle, file_path: String) -> Result<(), String> {
    use std::fs;
    use std::path::{Path, PathBuf};
    
    // Get the app data directory
    let app_data_dir = app.path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;
    
    // Parse the provided file path
    let file_path_obj = Path::new(&file_path);
    
    // Extract just the filename to prevent directory traversal
    let file_name = file_path_obj
        .file_name()
        .ok_or("Invalid file name")?
        .to_str()
        .ok_or("Invalid file name encoding")?;
    
    // Validate the filename matches expected wallpaper pattern
    // Pattern: wallpaper-{theme}-{timestamp}.{ext}
    if !file_name.starts_with("wallpaper-") {
        return Err("Invalid wallpaper file".to_string());
    }
    
    // Check if it has at least 3 parts when split by dash (wallpaper-theme-timestamp)
    let parts: Vec<&str> = file_name.split('.').next().unwrap_or("").split('-').collect();
    if parts.len() < 3 {
        return Err("Invalid wallpaper file format".to_string());
    }
    
    // Check if it has a valid extension
    let valid_extensions = ["jpg", "jpeg", "png", "webp"];
    let has_valid_extension = valid_extensions.iter().any(|ext| {
        file_name.to_lowercase().ends_with(&format!(".{}", ext))
    });
    
    if !has_valid_extension {
        return Err("Invalid wallpaper file extension".to_string());
    }
    
    // Construct the target path in the app data directory (not in assets/backgrounds)
    let target_path = app_data_dir.join(file_name);
    
    // Canonicalize the app data directory
    let canonical_app_data = app_data_dir
        .canonicalize()
        .unwrap_or_else(|_| app_data_dir.clone());
    
    // Check if the file exists and canonicalize it
    let canonical_target = target_path
        .canonicalize()
        .map_err(|_| "Wallpaper file not found".to_string())?;
    
    // Verify the canonical target path is within the app data directory
    if !canonical_target.starts_with(&canonical_app_data) {
        return Err("Access denied: Path outside allowed directory".to_string());
    }
    
    // Additional security check: ensure the canonical path still has wallpaper prefix
    let canonical_filename = canonical_target
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("Invalid file name")?;
    
    if !canonical_filename.starts_with("wallpaper-") {
        return Err("Access denied: Not a wallpaper file".to_string());
    }
    
    // Check if it's a file (not a directory)
    if !canonical_target.is_file() {
        return Err("Path is not a file".to_string());
    }
    
    // Attempt to delete the file
    match fs::remove_file(&canonical_target) {
        Ok(_) => {
            println!("Successfully deleted wallpaper: {:?}", canonical_target);
            Ok(())
        },
        Err(e) => match e.kind() {
            std::io::ErrorKind::PermissionDenied => {
                Err("Permission denied: Cannot delete this wallpaper".to_string())
            }
            _ => Err(format!("Failed to delete wallpaper: {}", e))
        }
    }
}
