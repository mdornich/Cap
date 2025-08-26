use std::fs;
use tempfile::TempDir;

fn create_test_recording_with_captions() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path();
    
    // Create recording-meta.json
    let meta_content = r#"{
        "pretty_name": "Test Recording with Captions",
        "sharing": null,
        "platform": "MacOS",
        "display": {
            "path": "content/display.mp4",
            "fps": 30
        },
        "camera": null,
        "audio": null,
        "cursor": null
    }"#;
    
    fs::write(project_path.join("recording-meta.json"), meta_content)
        .expect("Failed to write recording-meta.json");
    
    // Create captions.json with test data
    let captions_content = r##"{
        "segments": [
            {
                "id": "1",
                "start": 0.0,
                "end": 2.5,
                "text": "Welcome to this demo recording."
            },
            {
                "id": "2",
                "start": 2.5,
                "end": 5.0,
                "text": "This is a test of our caption export functionality."
            },
            {
                "id": "3",
                "start": 5.0,
                "end": 8.0,
                "text": "We can export captions in multiple formats."
            },
            {
                "id": "4",
                "start": 8.0,
                "end": 11.0,
                "text": "Including WebVTT, SRT, and plain text."
            },
            {
                "id": "5",
                "start": 11.0,
                "end": 14.0,
                "text": "Each format has its own use cases."
            }
        ],
        "settings": {
            "enabled": true,
            "font_family": "Arial",
            "font_size": 20,
            "font_weight": 500,
            "font_color": "#FFFFFF",
            "background_enabled": true,
            "background_color": "#000000",
            "background_opacity": 0.5,
            "position": "bottom",
            "offset": 20
        }
    }"##;
    
    fs::write(project_path.join("captions.json"), captions_content)
        .expect("Failed to write captions.json");
    
    temp_dir
}

#[tokio::test]
async fn test_export_captions_to_vtt() {
    let temp_dir = create_test_recording_with_captions();
    let project_path = temp_dir.path().to_path_buf();
    
    // Test export to string (no output path)
    let result = klip_desktop::captions::export_captions_to_vtt(project_path.clone(), None)
        .await
        .expect("Failed to export to VTT");
    
    // Check VTT format
    assert!(result.starts_with("WEBVTT\n\n"));
    assert!(result.contains("00:00:00.000 --> 00:00:02.500"));
    assert!(result.contains("Welcome to this demo recording."));
    assert!(result.contains("00:00:02.500 --> 00:00:05.000"));
    assert!(result.contains("This is a test of our caption export functionality."));
    
    // Test export to file
    let output_path = temp_dir.path().join("test_output.vtt");
    let file_result = klip_desktop::captions::export_captions_to_vtt(
        project_path, 
        Some(output_path.clone())
    )
    .await
    .expect("Failed to export VTT to file");
    
    // The function should return the content even when writing to file
    assert_eq!(result, file_result);
    
    // Check file was created with correct content
    let file_content = fs::read_to_string(&output_path)
        .expect("Failed to read output file");
    assert_eq!(file_content, result);
}

#[tokio::test]
async fn test_export_captions_to_srt() {
    let temp_dir = create_test_recording_with_captions();
    let project_path = temp_dir.path().to_path_buf();
    
    // Test export to string (no output path)
    let result = klip_desktop::captions::export_captions_to_srt(project_path.clone(), None)
        .await
        .expect("Failed to export to SRT");
    
    // Check SRT format
    assert!(result.starts_with("1\n"));
    assert!(result.contains("00:00:00,000 --> 00:00:02,500"));
    assert!(result.contains("Welcome to this demo recording."));
    assert!(result.contains("2\n"));
    assert!(result.contains("00:00:02,500 --> 00:00:05,000"));
    
    // Test export to file
    let output_path = temp_dir.path().join("test_output.srt");
    let file_result = klip_desktop::captions::export_captions_to_srt(
        project_path,
        Some(output_path.clone())
    )
    .await
    .expect("Failed to export SRT to file");
    
    assert_eq!(result, file_result);
    
    let file_content = fs::read_to_string(&output_path)
        .expect("Failed to read output file");
    assert_eq!(file_content, result);
}

#[tokio::test]
async fn test_export_captions_to_text() {
    let temp_dir = create_test_recording_with_captions();
    let project_path = temp_dir.path().to_path_buf();
    
    // Test export to string (no output path)
    let result = klip_desktop::captions::export_captions_to_text(project_path.clone(), None)
        .await
        .expect("Failed to export to text");
    
    // Check plain text format (segments separated by double newlines)
    let expected = "Welcome to this demo recording.\n\n\
                   This is a test of our caption export functionality.\n\n\
                   We can export captions in multiple formats.\n\n\
                   Including WebVTT, SRT, and plain text.\n\n\
                   Each format has its own use cases.";
    assert_eq!(result, expected);
    
    // Test export to file
    let output_path = temp_dir.path().join("test_output.txt");
    let file_result = klip_desktop::captions::export_captions_to_text(
        project_path,
        Some(output_path.clone())
    )
    .await
    .expect("Failed to export text to file");
    
    assert_eq!(result, file_result);
    
    let file_content = fs::read_to_string(&output_path)
        .expect("Failed to read output file");
    assert_eq!(file_content, result);
}

#[tokio::test]
async fn test_has_captions() {
    // Test with captions
    let temp_dir_with = create_test_recording_with_captions();
    let has_captions = klip_desktop::captions::has_captions(temp_dir_with.path().to_path_buf())
        .await
        .expect("Failed to check captions");
    assert!(has_captions);
    
    // Test without captions
    let temp_dir_without = TempDir::new().expect("Failed to create temp dir");
    let meta_content = r#"{
        "pretty_name": "Test Recording without Captions",
        "sharing": null,
        "platform": "MacOS",
        "display": {
            "path": "content/display.mp4",
            "fps": 30
        }
    }"#;
    fs::write(temp_dir_without.path().join("recording-meta.json"), meta_content)
        .expect("Failed to write recording-meta.json");
    
    let has_no_captions = klip_desktop::captions::has_captions(temp_dir_without.path().to_path_buf())
        .await
        .expect("Failed to check captions");
    assert!(!has_no_captions);
}

#[tokio::test]
async fn test_export_no_captions_error() {
    // Test error handling when no captions exist
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let meta_content = r#"{
        "pretty_name": "Test Recording without Captions",
        "sharing": null,
        "platform": "MacOS",
        "display": {
            "path": "content/display.mp4",
            "fps": 30
        }
    }"#;
    fs::write(temp_dir.path().join("recording-meta.json"), meta_content)
        .expect("Failed to write recording-meta.json");
    
    // Try to export VTT - should fail gracefully
    let result = klip_desktop::captions::export_captions_to_vtt(
        temp_dir.path().to_path_buf(),
        None
    ).await;
    
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("No captions found"));
}