use std::fs;
use std::path::Path;
use rocket::serde::{Deserialize, json::Json};
use rocket::{get, post};
use crate::web::ApiResponse;
use crate::config::load_config_or_default;

pub fn get_transcript(video_path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let video_path_obj = Path::new(video_path);
    let txt_path = video_path_obj.with_extension("txt");
    
    if !txt_path.exists() {
        return Err(format!("Transcript file does not exist: {}", txt_path.display()).into());
    }
    
    let content = fs::read_to_string(&txt_path)?;
    Ok(content)
}

pub fn set_line(video_path: &str, line_number: usize, new_content: &str) -> Result<(), Box<dyn std::error::Error>> {
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
        return Err(format!("Line number {} is beyond the end of the file (file has {} lines)", 
                          line_number, lines.len()).into());
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

pub fn set(video_path: &str, new_content: &str) -> Result<(), Box<dyn std::error::Error>> {
    let video_path_obj = Path::new(video_path);
    
    // Validate that the video path exists
    if !video_path_obj.exists() {
        return Err(format!("Video file does not exist: {}", video_path_obj.display()).into());
    }
    
    // Load config to get watch directories
    let config = load_config_or_default();
    
    // Check if the video path is within any of the watch directories
    let video_canonical = video_path_obj.canonicalize()
        .map_err(|e| format!("Cannot canonicalize video path {}: {}", video_path_obj.display(), e))?;
    
    let mut is_in_watch_dir = false;
    for watch_dir in &config.watch_directories {
        if let Ok(watch_canonical) = Path::new(watch_dir).canonicalize() {
            if video_canonical.starts_with(&watch_canonical) {
                is_in_watch_dir = true;
                break;
            }
        }
    }
    
    if !is_in_watch_dir {
        return Err(format!("Video path {} is not within any watch directory", video_path_obj.display()).into());
    }
    
    let txt_path = video_path_obj.with_extension("txt");
    fs::write(txt_path, new_content)?;
    Ok(())
}

pub fn regenerate(video_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let video_path_obj = Path::new(video_path);
    let txt_path = video_path_obj.with_extension("txt");
    
    let mut deleted_files = Vec::new();
    
    if txt_path.exists() {
        fs::remove_file(&txt_path)?;
        deleted_files.push("transcript");
    }
    
    if deleted_files.is_empty() {
        return Err("No transcript files found to delete".into());
    }
    
    Ok(())
}

#[derive(Deserialize)]
pub struct ReplaceTranscriptRequest {
    pub video_path: String,
    pub new_content: String,
}

#[get("/api/transcripts?<video_path>")]
pub fn web_get_transcript_by_path(video_path: String) -> Json<ApiResponse<String>> {
    match get_transcript(&video_path) {
        Ok(content) => Json(ApiResponse::success(content)),
        Err(e) => Json(ApiResponse::error(format!("Failed to get transcript: {}", e))),
    }
}

#[post("/api/transcripts/replace", data = "<request>")]
pub fn web_replace_transcript(request: Json<ReplaceTranscriptRequest>) -> Json<ApiResponse<String>> {
    match set(&request.video_path, &request.new_content) {
        Ok(_) => Json(ApiResponse::success("Transcript replaced successfully".to_string())),
        Err(e) => Json(ApiResponse::error(format!("Failed to replace transcript: {}", e))),
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
        assert!(result.unwrap_err().to_string().contains("Transcript file does not exist"));
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
        assert_eq!(result.unwrap_err().to_string(), "Line number must be greater than 0");
    }

    #[test]
    fn test_set_line_beyond_file_length() {
        let temp_dir = TempDir::new().unwrap();
        let video_path = temp_dir.path().join("test_video.mp4");
        
        create_test_file(temp_dir.path(), "test_video.txt", "Line 1\nLine 2");
        
        let result = set_line(video_path.to_str().unwrap(), 5, "New content");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Line number 5 is beyond the end of the file"));
    }

    #[test]
    fn test_set_line_file_not_exists() {
        let temp_dir = TempDir::new().unwrap();
        let video_path = temp_dir.path().join("nonexistent_video.mp4");
        
        let result = set_line(video_path.to_str().unwrap(), 1, "New content");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Transcript file does not exist"));
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
        
        // Mock config by setting environment variable
        std::env::set_var("ATCI_CONFIG_PATH", temp_dir.path().join("config.toml"));
        let config = crate::config::AtciConfig {
            watch_directories: vec![temp_dir.path().to_string_lossy().to_string()],
            ..Default::default()
        };
        crate::config::store_config(&config).unwrap();
        
        let result = set(video_path.to_str().unwrap(), new_content);
        assert!(result.is_ok());
        
        let saved_content = get_transcript(video_path.to_str().unwrap()).unwrap();
        assert_eq!(saved_content, new_content);
        
        // Cleanup
        std::env::remove_var("ATCI_CONFIG_PATH");
    }

    #[test]
    fn test_set_overwrites_existing_file() {
        let temp_dir = TempDir::new().unwrap();
        let video_path = temp_dir.path().join("test_video.mp4");
        let original_content = "Original content";
        let new_content = "New content";
        
        create_test_file(temp_dir.path(), "test_video.txt", original_content);
        create_test_file(temp_dir.path(), "test_video.mp4", "fake video content");
        
        // Mock config by setting environment variable
        std::env::set_var("ATCI_CONFIG_PATH", temp_dir.path().join("config.toml"));
        let config = crate::config::AtciConfig {
            watch_directories: vec![temp_dir.path().to_string_lossy().to_string()],
            ..Default::default()
        };
        crate::config::store_config(&config).unwrap();
        
        let result = set(video_path.to_str().unwrap(), new_content);
        assert!(result.is_ok());
        
        let saved_content = get_transcript(video_path.to_str().unwrap()).unwrap();
        assert_eq!(saved_content, new_content);
        
        // Cleanup
        std::env::remove_var("ATCI_CONFIG_PATH");
    }

    #[test]
    fn test_set_video_file_not_exists() {
        let temp_dir = TempDir::new().unwrap();
        let video_path = temp_dir.path().join("nonexistent_video.mp4");
        let new_content = "New content";
        
        let result = set(video_path.to_str().unwrap(), new_content);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Video file does not exist"));
    }

    #[test]
    fn test_set_path_not_in_watch_directory() {
        let temp_dir = TempDir::new().unwrap();
        let other_dir = TempDir::new().unwrap();
        let video_path = other_dir.path().join("test_video.mp4");
        let new_content = "New content";
        
        create_test_file(other_dir.path(), "test_video.mp4", "fake video content");
        
        // Mock config with different watch directory
        std::env::set_var("ATCI_CONFIG_PATH", temp_dir.path().join("config.toml"));
        let config = crate::config::AtciConfig {
            watch_directories: vec![temp_dir.path().to_string_lossy().to_string()],
            ..Default::default()
        };
        crate::config::store_config(&config).unwrap();
        
        let result = set(video_path.to_str().unwrap(), new_content);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("is not within any watch directory"));
        
        // Cleanup
        std::env::remove_var("ATCI_CONFIG_PATH");
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
        assert_eq!(result.unwrap_err().to_string(), "No transcript files found to delete");
    }
}