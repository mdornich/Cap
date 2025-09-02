use serde::{Deserialize, Serialize};

#[cfg(target_os = "macos")]
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(options: core_foundation::dictionary::CFDictionaryRef)
        -> bool;
}

#[derive(Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum OSPermission {
    ScreenRecording,
    Camera,
    Microphone,
    Accessibility,
}

#[tauri::command(async)]
#[specta::specta]
pub fn open_permission_settings(permission: OSPermission) {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;

        match permission {
            OSPermission::ScreenRecording => {
                Command::new("open")
                    .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture")
                    .spawn()
                    .expect("Failed to open Screen Recording settings");
            }
            OSPermission::Camera => {
                Command::new("open")
                    .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Camera")
                    .spawn()
                    .expect("Failed to open Camera settings");
            }
            OSPermission::Microphone => {
                Command::new("open")
                    .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone")
                    .spawn()
                    .expect("Failed to open Microphone settings");
            }
            OSPermission::Accessibility => {
                Command::new("open")
                    .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
                    .spawn()
                    .expect("Failed to open Accessibility settings");
            }
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn request_permission(permission: OSPermission) -> bool {
    #[cfg(target_os = "macos")]
    {
        use cap_media::platform::AVMediaType;
        use std::time::Duration;
        use tokio::time::sleep;

        match permission {
            OSPermission::ScreenRecording => {
                scap::request_permission();
                // Wait a bit for the permission to be processed
                sleep(Duration::from_millis(500)).await;
                // Check if permission was granted
                scap::has_permission() || check_screen_recording_permission_via_window_list()
            }
            OSPermission::Camera => {
                request_av_permission(AVMediaType::Video);
                // Wait for permission dialog to be processed
                sleep(Duration::from_millis(500)).await;
                matches!(check_av_permission(AVMediaType::Video), OSPermissionStatus::Granted)
            },
            OSPermission::Microphone => {
                request_av_permission(AVMediaType::Audio);
                // Wait for permission dialog to be processed
                sleep(Duration::from_millis(500)).await;
                matches!(check_av_permission(AVMediaType::Audio), OSPermissionStatus::Granted)
            },
            OSPermission::Accessibility => {
                request_accessibility_permission();
                // Wait a bit for the permission to be processed
                sleep(Duration::from_millis(500)).await;
                matches!(check_accessibility_permission(), OSPermissionStatus::Granted)
            },
        }
    }
    
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

#[cfg(target_os = "macos")]
fn request_av_permission(media_type: cap_media::platform::AVMediaType) {
    use objc::{runtime::*, *};
    use tauri_nspanel::block::ConcreteBlock;

    let callback = move |_: BOOL| {};
    let cls = class!(AVCaptureDevice);
    let objc_fn_block: ConcreteBlock<(BOOL,), (), _> = ConcreteBlock::new(callback);
    let objc_fn_pass = objc_fn_block.copy();
    unsafe {
        let _: () = msg_send![cls, requestAccessForMediaType:media_type.into_ns_str() completionHandler:objc_fn_pass];
    };
}

#[derive(Serialize, Deserialize, Debug, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum OSPermissionStatus {
    // This platform does not require this permission
    NotNeeded,
    // The user has neither granted nor denied permission
    Empty,
    // The user has explicitly granted permission
    Granted,
    // The user has denied permission, or has granted it but not yet restarted
    Denied,
}

impl OSPermissionStatus {
    pub fn permitted(&self) -> bool {
        match self {
            Self::NotNeeded | Self::Granted => true,
            _ => false,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct OSPermissionsCheck {
    pub screen_recording: OSPermissionStatus,
    pub microphone: OSPermissionStatus,
    pub camera: OSPermissionStatus,
    pub accessibility: OSPermissionStatus,
}

impl OSPermissionsCheck {
    pub fn necessary_granted(&self) -> bool {
        // Only screen recording is truly necessary
        // Accessibility is optional (only needed for window focusing)
        self.screen_recording.permitted()
    }
    
    pub fn all_granted(&self) -> bool {
        self.screen_recording.permitted() 
            && self.accessibility.permitted()
            && self.microphone.permitted()
            && self.camera.permitted()
    }
}

#[cfg(target_os = "macos")]
fn check_av_permission(media_type: cap_media::platform::AVMediaType) -> OSPermissionStatus {
    use cap_media::platform::AVAuthorizationStatus;
    use objc::*;

    let cls = objc::class!(AVCaptureDevice);
    let status: AVAuthorizationStatus =
        unsafe { msg_send![cls, authorizationStatusForMediaType:media_type.into_ns_str()] };
    match status {
        AVAuthorizationStatus::NotDetermined => OSPermissionStatus::Empty,
        AVAuthorizationStatus::Authorized => OSPermissionStatus::Granted,
        _ => OSPermissionStatus::Denied,
    }
}

#[tauri::command(async)]
#[specta::specta]
pub fn do_permissions_check(initial_check: bool) -> OSPermissionsCheck {
    #[cfg(target_os = "macos")]
    {
        use cap_media::platform::AVMediaType;

        OSPermissionsCheck {
            screen_recording: {
                // First try scap
                let mut result = scap::has_permission();
                
                // If scap says no, double-check using CGWindowListCopyWindowInfo
                // This works because it returns nil if we don't have permission
                if !result {
                    result = check_screen_recording_permission_via_window_list();
                }
                
                match (result, initial_check) {
                    (true, _) => OSPermissionStatus::Granted,
                    (false, true) => OSPermissionStatus::Empty,
                    (false, false) => OSPermissionStatus::Denied,
                }
            },
            microphone: check_av_permission(AVMediaType::Audio),
            camera: check_av_permission(AVMediaType::Video),
            accessibility: { check_accessibility_permission() },
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        OSPermissionsCheck {
            screen_recording: OSPermissionStatus::NotNeeded,
            microphone: OSPermissionStatus::NotNeeded,
            camera: OSPermissionStatus::NotNeeded,
            accessibility: OSPermissionStatus::NotNeeded,
        }
    }
}

#[cfg(target_os = "macos")]
fn check_screen_recording_permission_via_window_list() -> bool {
    use core_foundation::array::CFArray;
    use core_foundation::base::{CFType, TCFType};
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::number::CFNumber;
    use core_foundation::string::CFString;
    
    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGWindowListCopyWindowInfo(option: u32, relativeToWindow: u32) -> *const core_foundation::array::__CFArray;
    }
    
    unsafe {
        // kCGWindowListOptionAll = 0
        // kCGWindowListOptionOnScreenOnly = 1 << 0
        // kCGWindowListExcludeDesktopElements = 1 << 4
        // kCGNullWindowID = 0
        
        // Try with minimal options first (more reliable in production)
        let window_list_ptr = CGWindowListCopyWindowInfo(0, 0);
        
        if window_list_ptr.is_null() {
            // If we get null with minimal options, we definitely don't have permission
            return false;
        }
        
        // We got a window list, so we have permission
        let window_list = CFArray::<CFDictionary>::wrap_under_create_rule(window_list_ptr);
        
        // In production, even with permission, the list might be empty initially
        // So we just check that we got a non-null list
        true
    }
}

pub fn check_accessibility_permission() -> OSPermissionStatus {
    #[cfg(target_os = "macos")]
    {
        let is_trusted = unsafe { AXIsProcessTrusted() };
        eprintln!("[Accessibility] AXIsProcessTrusted returned: {}", is_trusted);
        
        // In production, we might need to prompt the user
        if is_trusted {
            OSPermissionStatus::Granted
        } else {
            // Check if we're running in a production build
            #[cfg(not(debug_assertions))]
            {
                eprintln!("[Accessibility] Production build detected, prompting for permission");
                // Try to trigger the prompt
                request_accessibility_permission();
            }
            OSPermissionStatus::Denied
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        // For non-macOS platforms, assume permission is granted
        OSPermissionStatus::NotNeeded
    }
}

pub fn request_accessibility_permission() {
    #[cfg(target_os = "macos")]
    {
        use core_foundation::base::TCFType;
        use core_foundation::dictionary::CFDictionary;
        use core_foundation::string::CFString;

        eprintln!("[Accessibility] Requesting accessibility permission...");

        let prompt_key = CFString::new("AXTrustedCheckOptionPrompt");
        let prompt_value = core_foundation::boolean::CFBoolean::true_value();

        let options =
            CFDictionary::from_CFType_pairs(&[(prompt_key.as_CFType(), prompt_value.as_CFType())]);

        let result = unsafe {
            AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef())
        };
        
        eprintln!("[Accessibility] AXIsProcessTrustedWithOptions returned: {}", result);
        
        if !result {
            eprintln!("[Accessibility] Permission not granted. User needs to:");
            eprintln!("  1. Open System Settings > Privacy & Security > Accessibility");
            eprintln!("  2. Add and enable Klip.app");
            eprintln!("  3. Restart Klip after granting permission");
        }
    }
}
