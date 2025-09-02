#[cfg(test)]
mod tests {
    use crate::hotkeys::*;
    use tauri_plugin_global_shortcut::{Code, Modifiers};

    #[test]
    fn test_hotkey_to_shortcut_conversion() {
        let hotkey = Hotkey {
            code: Code::KeyP,
            meta: true,
            ctrl: false,
            alt: false,
            shift: true,
        };
        
        let shortcut = hotkey.to_shortcut();
        
        // Since Shortcut doesn't expose modifiers() method, we verify via Debug output
        let debug_str = format!("{:?}", shortcut);
        assert!(debug_str.contains("META"));
        assert!(debug_str.contains("SHIFT"));
        assert!(!debug_str.contains("CONTROL"));
        assert!(!debug_str.contains("ALT"));
    }

    #[test]
    fn test_hotkey_with_all_modifiers() {
        let hotkey = Hotkey {
            code: Code::KeyA,
            meta: true,
            ctrl: true,
            alt: true,
            shift: true,
        };
        
        let shortcut = hotkey.to_shortcut();
        
        // Verify all modifiers are set via Debug output
        let debug_str = format!("{:?}", shortcut);
        assert!(debug_str.contains("META"));
        assert!(debug_str.contains("SHIFT"));
        assert!(debug_str.contains("CONTROL"));
        assert!(debug_str.contains("ALT"));
    }

    #[test]
    fn test_hotkey_with_no_modifiers() {
        let hotkey = Hotkey {
            code: Code::F1,
            meta: false,
            ctrl: false,
            alt: false,
            shift: false,
        };
        
        let shortcut = hotkey.to_shortcut();
        
        // Verify no modifiers are set via Debug output
        let debug_str = format!("{:?}", shortcut);
        // When no modifiers, the debug string should not contain modifier flags
        assert!(!debug_str.contains("META"));
        assert!(!debug_str.contains("SHIFT"));
        assert!(!debug_str.contains("CONTROL"));
        assert!(!debug_str.contains("ALT"));
    }

    #[test]
    fn test_hotkey_action_serialization() {
        use serde_json;
        
        let action = HotkeyAction::StartRecording;
        let serialized = serde_json::to_string(&action).unwrap();
        assert_eq!(serialized, "\"startRecording\"");
        
        let action = HotkeyAction::StopRecording;
        let serialized = serde_json::to_string(&action).unwrap();
        assert_eq!(serialized, "\"stopRecording\"");
    }

    #[test]
    fn test_hotkey_store_operations() {
        use std::collections::HashMap;
        
        let mut store = HotkeysStore {
            hotkeys: HashMap::new(),
        };
        
        let hotkey = Hotkey {
            code: Code::KeyR,
            meta: true,
            ctrl: false,
            alt: false,
            shift: false,
        };
        
        // Test insertion
        store.hotkeys.insert(HotkeyAction::StartRecording, hotkey);
        assert_eq!(store.hotkeys.len(), 1);
        assert!(store.hotkeys.contains_key(&HotkeyAction::StartRecording));
        
        // Test removal
        store.hotkeys.remove(&HotkeyAction::StartRecording);
        assert_eq!(store.hotkeys.len(), 0);
    }
}