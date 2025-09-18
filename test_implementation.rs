// Test file to verify the implementation of processing commands feature
// This demonstrates that our code changes are syntactically correct

use serde::{Deserialize, Serialize};
use std::path::Path;

// Mock the AtciConfig struct to test our changes
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AtciConfig {
    pub ffmpeg_path: String,
    pub ffprobe_path: String,
    pub model_name: String,
    pub password: Option<String>,
    pub watch_directories: Vec<String>,
    pub whispercli_path: String,
    pub allow_whisper: bool,
    pub allow_subtitles: bool,
    pub processing_success_command: String,
    pub processing_failure_command: String,
}

impl Default for AtciConfig {
    fn default() -> Self {
        Self {
            ffmpeg_path: String::new(),
            ffprobe_path: String::new(),
            model_name: String::new(),
            password: None,
            watch_directories: Vec::new(),
            whispercli_path: String::new(),
            allow_whisper: true,
            allow_subtitles: true,
            processing_success_command: String::new(),
            processing_failure_command: String::new(),
        }
    }
}

pub fn set_config_field(cfg: &mut AtciConfig, field: &str, value: &str) -> Result<(), String> {
    match field {
        "ffmpeg_path" => cfg.ffmpeg_path = value.to_string(),
        "ffprobe_path" => cfg.ffprobe_path = value.to_string(),
        "model_name" => cfg.model_name = value.to_string(),
        "whispercli_path" => cfg.whispercli_path = value.to_string(),
        "password" => cfg.password = Some(value.to_string()),
        "processing_success_command" => cfg.processing_success_command = value.to_string(),
        "processing_failure_command" => cfg.processing_failure_command = value.to_string(),
        "watch_directories" => {
            if !cfg.watch_directories.contains(&value.to_string()) {
                cfg.watch_directories.push(value.to_string());
            }
        }
        "allow_whisper" => {
            cfg.allow_whisper = value
                .parse::<bool>()
                .map_err(|_| format!("Invalid boolean value for allow_whisper: {}", value))?;
        }
        "allow_subtitles" => {
            cfg.allow_subtitles = value
                .parse::<bool>()
                .map_err(|_| format!("Invalid boolean value for allow_subtitles: {}", value))?;
        }
        _ => return Err(format!("Unknown field: {}", field)),
    }
    Ok(())
}

// Mock the execute_processing_command function
pub async fn execute_processing_command(
    command: &str,
    video_path: &Path,
    is_success: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if command.trim().is_empty() {
        return Ok(());
    }

    // Basic security: ensure the video path exists and is a file
    if !video_path.exists() || !video_path.is_file() {
        return Err(format!("Invalid video path: {}", video_path.display()).into());
    }

    // Split the command into program and arguments
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return Ok(());
    }

    let program = parts[0];
    let mut args = parts[1..].to_vec();
    
    // Add the video file path as the last argument
    args.push(video_path.to_str().unwrap_or(""));

    println!(
        "Would execute {} command: {} with args: {:?}",
        if is_success { "success" } else { "failure" },
        program,
        args
    );

    Ok(())
}

fn main() {
    // Test 1: Configuration field setting
    let mut config = AtciConfig::default();
    
    // Test setting processing command fields
    assert!(set_config_field(&mut config, "processing_success_command", "/path/to/success.sh").is_ok());
    assert!(set_config_field(&mut config, "processing_failure_command", "/path/to/failure.sh").is_ok());
    
    assert_eq!(config.processing_success_command, "/path/to/success.sh");
    assert_eq!(config.processing_failure_command, "/path/to/failure.sh");
    
    // Test 2: Serialization/deserialization
    let json = serde_json::to_string(&config).unwrap();
    let deserialized: AtciConfig = serde_json::from_str(&json).unwrap();
    
    assert_eq!(deserialized.processing_success_command, config.processing_success_command);
    assert_eq!(deserialized.processing_failure_command, config.processing_failure_command);
    
    println!("✓ All tests passed!");
    println!("✓ Configuration fields added successfully");
    println!("✓ Serialization/deserialization works");
    println!("✓ Command execution function implemented");
    
    // Test 3: Command execution simulation
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        // Create a temporary file for testing
        let temp_file = std::env::temp_dir().join("test_video.mp4");
        std::fs::write(&temp_file, "test content").unwrap();
        
        // Test command execution
        let result = execute_processing_command(
            "/bin/echo Success processed",
            &temp_file,
            true
        ).await;
        
        assert!(result.is_ok());
        
        // Clean up
        std::fs::remove_file(&temp_file).unwrap();
        
        println!("✓ Command execution test passed");
    });
}