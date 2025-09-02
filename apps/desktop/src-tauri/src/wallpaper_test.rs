#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::fs;
    use tempfile::TempDir;

    /// Helper function to validate wallpaper paths
    /// This mimics the security check in delete_wallpaper
    fn validate_wallpaper_path(base_dir: &Path, file_path: &str) -> Result<PathBuf, String> {
        // Only use the file name component to prevent directory traversal
        let file_name = Path::new(file_path)
            .file_name()
            .ok_or("Invalid file name")?
            .to_str()
            .ok_or("Invalid file name encoding")?;
        
        // Validate the filename matches expected wallpaper pattern
        // Must be: wallpaper-{theme}-{timestamp}.{ext}
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
        
        let target_path = base_dir.join(file_name);
        
        // Canonicalize both paths
        let canonical_base = base_dir
            .canonicalize()
            .map_err(|e| format!("Failed to canonicalize base: {}", e))?;
        
        let canonical_target = target_path
            .canonicalize()
            .map_err(|_| "Wallpaper file not found".to_string())?;
        
        // Verify the canonical target path is within the canonical base directory
        if !canonical_target.starts_with(&canonical_base) {
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
        
        Ok(canonical_target)
    }

    #[test]
    fn test_path_traversal_prevention() {
        // Create a temporary directory structure for testing
        let temp_dir = TempDir::new().unwrap();
        let app_data_dir = temp_dir.path();
        
        // Create a test file in the app data directory with valid wallpaper name
        let safe_file = app_data_dir.join("wallpaper-test-123.jpg");
        fs::write(&safe_file, b"test").unwrap();
        
        // Create a file outside the app data directory
        let outside_file = temp_dir.path().parent().unwrap_or(temp_dir.path()).join("sensitive.txt");
        fs::write(&outside_file, b"sensitive").ok(); // May fail if no parent, that's OK
        
        // Test various path traversal attempts
        let malicious_paths = vec![
            "../sensitive.txt",
            "../../sensitive.txt",
            "../../../sensitive.txt",
            "assets/../../../sensitive.txt",
            "./../../sensitive.txt",
        ];
        
        for path in malicious_paths {
            let result = validate_wallpaper_path(app_data_dir, path);
            // These should all fail because they don't start with "wallpaper-"
            assert!(result.is_err(), "Path traversal not prevented for: {}", path);
            assert!(result.unwrap_err().contains("Invalid wallpaper file"), 
                   "Wrong error for path: {}", path);
        }
    }

    #[test]
    fn test_valid_wallpaper_paths() {
        // Create a temporary directory structure for testing
        let temp_dir = TempDir::new().unwrap();
        let app_data_dir = temp_dir.path();
        
        // Create files with valid wallpaper naming pattern
        let valid_files = vec![
            "wallpaper-macOS-123.jpg",
            "wallpaper-dark-456.png",
            "wallpaper-custom-789.jpeg",
            "wallpaper-blue-abc.webp",
            "wallpaper-custom-1234567890123.jpg", // User uploaded with timestamp
        ];
        
        for file_name in &valid_files {
            let file_path = app_data_dir.join(file_name);
            fs::write(&file_path, b"test").unwrap();
        }
        
        // Test that valid paths are accepted
        for file_name in valid_files {
            let result = validate_wallpaper_path(app_data_dir, file_name);
            assert!(result.is_ok(), "Valid path rejected: {}", file_name);
            
            // Verify the result points to the correct file
            let canonical_result = result.unwrap();
            assert!(canonical_result.ends_with(file_name));
            // The canonicalized path might not match exactly due to symlinks or temp dir resolution
            // Just verify it's a valid path that exists
            assert!(canonical_result.exists());
        }
    }

    #[test]
    fn test_invalid_wallpaper_names() {
        let temp_dir = TempDir::new().unwrap();
        let app_data_dir = temp_dir.path();
        
        // Create files with invalid naming patterns
        let invalid_files = vec![
            "background.jpg",      // Doesn't start with wallpaper-
            "image.png",          // Doesn't start with wallpaper-
            "wallpaper.jpg",      // Missing dash and theme
            "wallpaper-.jpg",     // Missing theme/timestamp
            "wallpaper-test.gif", // Invalid extension
            "wallpaper-test.bmp", // Invalid extension
        ];
        
        for file_name in &invalid_files {
            let file_path = app_data_dir.join(file_name);
            fs::write(&file_path, b"test").unwrap();
            
            let result = validate_wallpaper_path(app_data_dir, file_name);
            assert!(result.is_err(), "Invalid file accepted: {}", file_name);
        }
    }

    #[test]
    fn test_non_existent_file_handling() {
        let temp_dir = TempDir::new().unwrap();
        let app_data_dir = temp_dir.path();
        
        // Test with a non-existent file with valid naming pattern
        let result = validate_wallpaper_path(app_data_dir, "wallpaper-missing-123.jpg");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_symlink_attack_prevention() {
        // This test requires Unix-like systems for symlink support
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            
            let temp_dir = TempDir::new().unwrap();
            let app_data_dir = temp_dir.path();
            
            // Create a sensitive file outside
            let sensitive_file = temp_dir.path().parent()
                .unwrap_or(temp_dir.path())
                .join("sensitive.txt");
            fs::write(&sensitive_file, b"sensitive data").ok();
            
            // Create a symlink inside app data pointing outside
            let symlink_path = app_data_dir.join("wallpaper-evil-link.jpg");
            if let Ok(_) = symlink(&sensitive_file, &symlink_path) {
                // The validation should reject this because canonicalization
                // will resolve the symlink to outside the allowed directory
                let result = validate_wallpaper_path(app_data_dir, "wallpaper-evil-link.jpg");
                assert!(result.is_err(), "Symlink attack not prevented");
            }
        }
    }

    #[test]
    fn test_special_characters_in_filename() {
        let temp_dir = TempDir::new().unwrap();
        let app_data_dir = temp_dir.path();
        
        // Test with special characters in valid wallpaper names
        let special_names = vec![
            "wallpaper-test-..123.jpg",   // Dots in timestamp
            "wallpaper-dark-456...jpg",   // Multiple dots (but still ends with .jpg)
            "wallpaper-custom-78 9.jpg",  // Spaces in timestamp
            "wallpaper-blue-abc_def.jpg", // Underscores
        ];
        
        for name in &special_names {
            // Create the file
            let file_path = app_data_dir.join(name);
            fs::write(&file_path, b"test").unwrap();
            
            // Should work with valid wallpaper names
            let result = validate_wallpaper_path(app_data_dir, name);
            assert!(result.is_ok(), "Failed to handle special filename: {}", name);
        }
        
        // Test invalid patterns that should be rejected
        let invalid_names = vec![
            "..wallpaper-test.jpg",    // Starting with dots
            "../wallpaper-test.jpg",   // Path traversal attempt
            "wallpaper-../../etc.jpg", // Path traversal in name
        ];
        
        for name in invalid_names {
            let result = validate_wallpaper_path(app_data_dir, name);
            assert!(result.is_err(), "Invalid name accepted: {}", name);
        }
    }
}