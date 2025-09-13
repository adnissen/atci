// atci (andrew's transcript and clipping interface)
// Copyright (C) 2025 Andrew Nissen

use crate::auth::AuthGuard;
use crate::config::load_config_or_default;
use crate::files;
use crate::model_manager;
use crate::queue;
use crate::video_processor;
use crate::web::ApiResponse;
use dialoguer::Select;
use rocket::serde::{Deserialize, json::Json};
use rocket::{get, post};
use std::fs;
use std::path::Path;

pub fn get_transcript(video_path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let video_path_obj = Path::new(video_path);
    let txt_path = video_path_obj.with_extension("txt");

    if !txt_path.exists() {
        return Err(format!("Transcript file does not exist: {}", txt_path.display()).into());
    }

    let content = fs::read_to_string(&txt_path)?;
    Ok(content)
}

pub fn set_line(
    video_path: &str,
    line_number: usize,
    new_content: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if line_number == 0 {
        return Err("Line number must be greater than 0".into());
    }

    let video_path_obj = Path::new(video_path);
    let txt_path = video_path_obj.with_extension("txt");

    if !txt_path.exists() {
        return Err(format!("Transcript file does not exist: {}", txt_path.display()).into());
    }

    let content = fs::read_to_string(&txt_path)?;
    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

    let line_index = line_number - 1; // Convert to 0-based index

    if line_index >= lines.len() {
        return Err(format!(
            "Line number {} is beyond the end of the file (file has {} lines)",
            line_number,
            lines.len()
        )
        .into());
    }

    lines[line_index] = new_content.to_string();

    // Preserve the original line ending style
    let line_ending = if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };

    let updated_content = lines.join(line_ending);
    fs::write(&txt_path, updated_content)?;

    Ok(())
}

fn set_with_config(
    video_path: &str,
    new_content: &str,
    config: &crate::AtciConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let video_path_obj = Path::new(video_path);

    // Validate that the video path exists
    if !video_path_obj.exists() {
        return Err(format!("Video file does not exist: {}", video_path_obj.display()).into());
    }

    // Check if the video path is within any of the watch directories
    let video_canonical = video_path_obj.canonicalize().map_err(|e| {
        format!(
            "Cannot canonicalize video path {}: {}",
            video_path_obj.display(),
            e
        )
    })?;

    let mut is_in_watch_dir = false;
    for watch_dir in &config.watch_directories {
        if let Ok(watch_canonical) = Path::new(watch_dir).canonicalize()
            && video_canonical.starts_with(&watch_canonical)
        {
            is_in_watch_dir = true;
            break;
        }
    }

    if !is_in_watch_dir {
        return Err(format!(
            "Video path {} is not within any watch directory",
            video_path_obj.display()
        )
        .into());
    }

    let txt_path = video_path_obj.with_extension("txt");
    fs::write(txt_path, new_content)?;
    Ok(())
}

pub fn set(video_path: &str, new_content: &str) -> Result<(), Box<dyn std::error::Error>> {
    let config = load_config_or_default();
    set_with_config(video_path, new_content, &config)
}

pub fn regenerate(
    video_path: &str,
    model: Option<String>,
    subtitle_stream_index: Option<i32>,
) -> Result<(), Box<dyn std::error::Error>> {
    queue::add_to_queue(video_path, model, subtitle_stream_index)?;

    files::get_and_save_video_info_from_disk()?;

    Ok(())
}

pub fn rename(video_path: &str, new_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let video_path_obj = Path::new(video_path);
    let new_path_obj = Path::new(new_path);

    // Validate that the video path exists
    if !video_path_obj.exists() {
        return Err(format!("Video file does not exist: {}", video_path_obj.display()).into());
    }

    // Validate that it's a video file by checking common video extensions
    let is_video = video_path_obj
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| files::get_video_extensions().contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false);

    if !is_video {
        return Err(format!(
            "File is not a supported video format: {}",
            video_path_obj.display()
        )
        .into());
    }

    // Check if corresponding txt file exists
    let txt_path = video_path_obj.with_extension("txt");
    if !txt_path.exists() {
        return Err(format!("Transcript file does not exist: {}", txt_path.display()).into());
    }

    // Validate new path has same extension as original
    let original_ext = video_path_obj.extension();
    let new_ext = new_path_obj.extension();
    if original_ext != new_ext {
        return Err("New path must have the same file extension as the original".into());
    }

    // Check if new paths already exist
    if new_path_obj.exists() {
        return Err(format!(
            "Target video file already exists: {}",
            new_path_obj.display()
        )
        .into());
    }

    let new_txt_path = new_path_obj.with_extension("txt");
    if new_txt_path.exists() {
        return Err(format!(
            "Target transcript file already exists: {}",
            new_txt_path.display()
        )
        .into());
    }

    // Rename both files
    fs::rename(video_path_obj, new_path_obj)?;
    fs::rename(&txt_path, &new_txt_path)?;

    // Update cache
    files::get_and_save_video_info_from_disk()?;

    Ok(())
}

pub async fn regenerate_interactive(video_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let video_path_obj = Path::new(video_path);

    // Validate that the video path exists
    if !video_path_obj.exists() {
        return Err(format!("Video file does not exist: {}", video_path_obj.display()).into());
    }

    let cfg = crate::config::load_config()?;

    // Display file information
    println!("\n=== File Information ===");
    println!("File: {}", video_path_obj.display());

    // Get file size
    let metadata = fs::metadata(video_path_obj)?;
    let file_size = metadata.len();
    let size_mb = file_size as f64 / (1024.0 * 1024.0);
    println!("Size: {:.2} MB", size_mb);

    // Get file duration if possible
    match video_processor::get_video_duration(video_path_obj, Path::new(&cfg.ffprobe_path)).await {
        Ok(duration) => println!("Duration: {}", duration),
        Err(e) => println!("Duration: Unable to determine ({})", e),
    }

    println!("\n=== Processing Options ===");

    let mut options = Vec::new();
    let mut option_types = Vec::new();

    // Check for subtitle streams
    match video_processor::get_subtitle_streams(video_path_obj, Path::new(&cfg.ffprobe_path)).await
    {
        Ok(streams) if !streams.is_empty() => {
            for stream in streams.iter() {
                let lang_display = stream.language_display();
                options.push(format!("Subtitles: {} ({})", lang_display, stream.index));
                option_types.push(format!("subtitle_{}", stream.index));
            }
        }
        Ok(_) => {
            println!("No subtitle streams available");
        }
        Err(e) => {
            println!("Could not check for subtitle streams: {}", e);
        }
    }

    // Check for available Whisper models
    let models = model_manager::list_models();
    let downloaded_models: Vec<_> = models.iter().filter(|m| m.downloaded).collect();

    if !downloaded_models.is_empty() && cfg.allow_whisper {
        for model in &downloaded_models {
            options.push(format!("Whisper Model: {}{}", model.name, ""));
            option_types.push(format!("whisper_{}", model.name));
        }
    } else if !cfg.allow_whisper {
        println!("Whisper transcription is disabled in configuration");
    } else {
        println!("No Whisper models are downloaded");
    }

    options.push("Cancel".to_string());
    option_types.push("cancel".to_string());

    if options.len() == 1 {
        return Err("No processing options available".into());
    }

    // Prompt user to select an option
    let selection = Select::new()
        .with_prompt("Choose a processing method")
        .items(&options)
        .default(0)
        .interact()?;

    match option_types[selection].as_str() {
        "cancel" => {
            println!("Cancelled.");
            return Ok(());
        }
        option if option.starts_with("subtitle_") || option.starts_with("whisper_") => {
            let (model, subtitle_stream_index, process_type) = if option.starts_with("subtitle_") {
                let stream_index = option.strip_prefix("subtitle_").unwrap().parse::<i32>()?;
                println!("Processing with subtitle stream {}...", stream_index);
                (None, Some(stream_index), "subtitles")
            } else {
                let model_name = option.strip_prefix("whisper_").unwrap();
                println!("Processing with Whisper model: {}...", model_name);
                (Some(model_name.to_string()), None, "transcript")
            };

            println!("🚀 Processing {} for: {}", process_type, video_path);
            match video_processor::cancellable_create_transcript(
                video_path_obj,
                model,
                subtitle_stream_index,
            )
            .await
            {
                Ok(true) => {
                    // Add length metadata after successful processing
                    match video_processor::cancellable_add_length_to_metadata(video_path_obj).await
                    {
                        Ok(true) => {}
                        Ok(false) => {
                            println!("⚠️ Length metadata addition was cancelled");
                        }
                        Err(e) => {
                            eprintln!("Warning: Failed to add length metadata: {}", e);
                        }
                    }
                    println!(
                        "Successfully created {} for: {}",
                        process_type,
                        video_path_obj.display()
                    );
                }
                Ok(false) => {
                    println!("Processing was cancelled for: {}", video_path_obj.display());
                }
                Err(e) => {
                    eprintln!("Failed to create {}: {}", process_type, e);
                    return Err(e);
                }
            }
        }
        _ => {
            return Err("Invalid selection".into());
        }
    }

    Ok(())
}

#[derive(Deserialize)]
pub struct ReplaceTranscriptRequest {
    pub video_path: String,
    pub new_content: String,
}

#[derive(Deserialize)]
pub struct RegenerateTranscriptRequest {
    pub video_path: String,
    pub model: Option<String>,
    pub subtitle_stream_index: Option<i32>,
}

#[derive(Deserialize)]
pub struct RenameTranscriptRequest {
    pub video_path: String,
    pub new_path: String,
}

#[get("/api/transcripts?<video_path>")]
pub fn web_get_transcript_by_path(
    _auth: AuthGuard,
    video_path: String,
) -> Json<ApiResponse<String>> {
    match get_transcript(&video_path) {
        Ok(content) => Json(ApiResponse::success(content)),
        Err(e) => Json(ApiResponse::error(format!(
            "Failed to get transcript: {}",
            e
        ))),
    }
}

#[post("/api/transcripts/replace", data = "<request>")]
pub fn web_replace_transcript(
    _auth: AuthGuard,
    request: Json<ReplaceTranscriptRequest>,
) -> Json<ApiResponse<String>> {
    match set(&request.video_path, &request.new_content) {
        Ok(_) => Json(ApiResponse::success(
            "Transcript replaced successfully".to_string(),
        )),
        Err(e) => Json(ApiResponse::error(format!(
            "Failed to replace transcript: {}",
            e
        ))),
    }
}

#[post("/api/transcripts/regenerate", data = "<request>")]
pub fn web_regenerate_transcript(
    _auth: AuthGuard,
    request: Json<RegenerateTranscriptRequest>,
) -> Json<ApiResponse<String>> {
    match regenerate(
        &request.video_path,
        request.model.clone(),
        request.subtitle_stream_index,
    ) {
        Ok(_) => Json(ApiResponse::success(
            "Transcript regenerated successfully".to_string(),
        )),
        Err(e) => Json(ApiResponse::error(format!(
            "Failed to regenerate transcript: {}",
            e
        ))),
    }
}

#[post("/api/transcripts/rename", data = "<request>")]
pub fn web_rename_transcript(
    _auth: AuthGuard,
    request: Json<RenameTranscriptRequest>,
) -> Json<ApiResponse<String>> {
    match rename(&request.video_path, &request.new_path) {
        Ok(_) => Json(ApiResponse::success(
            "Transcript renamed successfully".to_string(),
        )),
        Err(e) => Json(ApiResponse::error(format!(
            "Failed to rename transcript: {}",
            e
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    fn create_test_file(dir: &Path, name: &str, content: &str) -> std::path::PathBuf {
        let file_path = dir.join(name);
        fs::write(&file_path, content).unwrap();
        file_path
    }

    #[test]
    fn test_get_transcript_success() {
        let temp_dir = TempDir::new().unwrap();
        let video_path = temp_dir.path().join("test_video.mp4");
        let transcript_content = "Line 1\nLine 2\nLine 3";

        create_test_file(temp_dir.path(), "test_video.txt", transcript_content);

        let result = get_transcript(video_path.to_str().unwrap());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), transcript_content);
    }

    #[test]
    fn test_get_transcript_file_not_exists() {
        let temp_dir = TempDir::new().unwrap();
        let video_path = temp_dir.path().join("nonexistent_video.mp4");

        let result = get_transcript(video_path.to_str().unwrap());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Transcript file does not exist")
        );
    }

    #[test]
    fn test_set_line_success() {
        let temp_dir = TempDir::new().unwrap();
        let video_path = temp_dir.path().join("test_video.mp4");
        let original_content = "Line 1\nLine 2\nLine 3";

        create_test_file(temp_dir.path(), "test_video.txt", original_content);

        let result = set_line(video_path.to_str().unwrap(), 2, "Modified Line 2");
        assert!(result.is_ok());

        let updated_content = get_transcript(video_path.to_str().unwrap()).unwrap();
        assert_eq!(updated_content, "Line 1\nModified Line 2\nLine 3");
    }

    #[test]
    fn test_set_line_zero_line_number() {
        let temp_dir = TempDir::new().unwrap();
        let video_path = temp_dir.path().join("test_video.mp4");

        create_test_file(temp_dir.path(), "test_video.txt", "Line 1\nLine 2");

        let result = set_line(video_path.to_str().unwrap(), 0, "New content");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Line number must be greater than 0"
        );
    }

    #[test]
    fn test_set_line_beyond_file_length() {
        let temp_dir = TempDir::new().unwrap();
        let video_path = temp_dir.path().join("test_video.mp4");

        create_test_file(temp_dir.path(), "test_video.txt", "Line 1\nLine 2");

        let result = set_line(video_path.to_str().unwrap(), 5, "New content");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Line number 5 is beyond the end of the file")
        );
    }

    #[test]
    fn test_set_line_file_not_exists() {
        let temp_dir = TempDir::new().unwrap();
        let video_path = temp_dir.path().join("nonexistent_video.mp4");

        let result = set_line(video_path.to_str().unwrap(), 1, "New content");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Transcript file does not exist")
        );
    }

    #[test]
    fn test_set_line_preserves_line_endings() {
        let temp_dir = TempDir::new().unwrap();
        let video_path = temp_dir.path().join("test_video.mp4");
        let original_content = "Line 1\r\nLine 2\r\nLine 3";

        create_test_file(temp_dir.path(), "test_video.txt", original_content);

        let result = set_line(video_path.to_str().unwrap(), 2, "Modified Line 2");
        assert!(result.is_ok());

        let updated_content = get_transcript(video_path.to_str().unwrap()).unwrap();
        assert_eq!(updated_content, "Line 1\r\nModified Line 2\r\nLine 3");
    }

    #[test]
    fn test_set_success() {
        let temp_dir = TempDir::new().unwrap();
        let video_path = temp_dir.path().join("test_video.mp4");
        let new_content = "Completely new content\nWith multiple lines";

        create_test_file(temp_dir.path(), "test_video.mp4", "fake video content");

        let config = crate::config::AtciConfig {
            watch_directories: vec![temp_dir.path().to_string_lossy().to_string()],
            ..Default::default()
        };

        let result = set_with_config(video_path.to_str().unwrap(), new_content, &config);
        assert!(result.is_ok());

        let saved_content = get_transcript(video_path.to_str().unwrap()).unwrap();
        assert_eq!(saved_content, new_content);
    }

    #[test]
    fn test_set_overwrites_existing_file() {
        let temp_dir = TempDir::new().unwrap();
        let video_path = temp_dir.path().join("test_video.mp4");
        let original_content = "Original content";
        let new_content = "New content";

        create_test_file(temp_dir.path(), "test_video.txt", original_content);
        create_test_file(temp_dir.path(), "test_video.mp4", "fake video content");

        let config = crate::config::AtciConfig {
            watch_directories: vec![temp_dir.path().to_string_lossy().to_string()],
            ..Default::default()
        };

        let result = set_with_config(video_path.to_str().unwrap(), new_content, &config);
        assert!(result.is_ok());

        let saved_content = get_transcript(video_path.to_str().unwrap()).unwrap();
        assert_eq!(saved_content, new_content);
    }

    #[test]
    fn test_set_video_file_not_exists() {
        let temp_dir = TempDir::new().unwrap();
        let video_path = temp_dir.path().join("nonexistent_video.mp4");
        let new_content = "New content";

        let result = set(video_path.to_str().unwrap(), new_content);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Video file does not exist")
        );
    }

    #[test]
    fn test_set_path_not_in_watch_directory() {
        let temp_dir = TempDir::new().unwrap();
        let other_dir = TempDir::new().unwrap();
        let video_path = other_dir.path().join("test_video.mp4");
        let new_content = "New content";

        create_test_file(other_dir.path(), "test_video.mp4", "fake video content");

        // Mock config with different watch directory
        let config = crate::config::AtciConfig {
            watch_directories: vec![temp_dir.path().to_string_lossy().to_string()],
            ..Default::default()
        };

        let result = set_with_config(video_path.to_str().unwrap(), new_content, &config);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("is not within any watch directory")
        );
    }

    #[test]
    fn test_regenerate_success_with_both_files() {
        let temp_dir = TempDir::new().unwrap();
        let video_path = temp_dir.path().join("test_video.mp4");

        create_test_file(temp_dir.path(), "test_video.txt", "transcript content");

        let result = regenerate(video_path.to_str().unwrap());
        assert!(result.is_ok());

        let txt_path = temp_dir.path().join("test_video.txt");
        assert!(!txt_path.exists());
    }

    #[test]
    fn test_regenerate_success_with_only_transcript() {
        let temp_dir = TempDir::new().unwrap();
        let video_path = temp_dir.path().join("test_video.mp4");

        create_test_file(temp_dir.path(), "test_video.txt", "transcript content");

        let result = regenerate(video_path.to_str().unwrap());
        assert!(result.is_ok());

        let txt_path = temp_dir.path().join("test_video.txt");
        assert!(!txt_path.exists());
    }

    #[test]
    fn test_regenerate_no_files_to_delete() {
        let temp_dir = TempDir::new().unwrap();
        let video_path = temp_dir.path().join("test_video.mp4");

        let result = regenerate(video_path.to_str().unwrap());
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "No transcript files found to delete"
        );
    }
}
