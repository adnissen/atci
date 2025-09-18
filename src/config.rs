// atci (andrew's transcript and clipping interface)
// Copyright (C) 2025 Andrew Nissen

use crate::auth::AuthGuard;
use crate::files;
use crate::web::ApiResponse;
use rocket::serde::json::Json;
use rocket::{get, post};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::process::Command;

fn default_true() -> bool {
    true
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AtciConfig {
    #[serde(default)]
    pub ffmpeg_path: String,
    #[serde(default)]
    pub ffprobe_path: String,
    #[serde(default)]
    pub model_name: String,
    pub password: Option<String>,
    #[serde(default)]
    pub watch_directories: Vec<String>,
    #[serde(default)]
    pub whispercli_path: String,
    #[serde(default = "default_true")]
    pub allow_whisper: bool,
    #[serde(default = "default_true")]
    pub allow_subtitles: bool,
    #[serde(default)]
    pub processing_success_command: String,
    #[serde(default)]
    pub processing_failure_command: String,
}

#[derive(Serialize)]
pub struct ConfigResponse {
    pub config: AtciConfig,
    pub is_complete: bool,
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

pub fn load_config() -> Result<AtciConfig, confy::ConfyError> {
    if let Ok(config_path) = std::env::var("ATCI_CONFIG_PATH") {
        confy::load_path(&config_path)
    } else {
        confy::load("atci", "config")
    }
}

pub fn load_config_or_default() -> AtciConfig {
    load_config().unwrap_or_default()
}

pub fn store_config(config: &AtciConfig) -> Result<(), confy::ConfyError> {
    let result;
    if let Ok(config_path) = std::env::var("ATCI_CONFIG_PATH") {
        result = confy::store_path(&config_path, config)
    } else {
        result = confy::store("atci", "config", config)
    }

    let _ = files::get_and_save_video_info_from_disk();

    result
}

#[get("/api/config")]
pub fn web_get_config(_auth: AuthGuard) -> Json<ApiResponse<ConfigResponse>> {
    let config = load_config().unwrap_or_default();

    let is_complete = !config.ffmpeg_path.is_empty()
        && !config.ffprobe_path.is_empty()
        && !config.model_name.is_empty()
        && !config.whispercli_path.is_empty();

    let response = ConfigResponse {
        config,
        is_complete,
    };

    Json(ApiResponse::success(response))
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
            // For watch_directories, treat the value as a single directory to add
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

#[post("/api/config", data = "<config>")]
pub fn web_set_config(_auth: AuthGuard, config: Json<AtciConfig>) -> Json<ApiResponse<String>> {
    match store_config(&config) {
        Ok(()) => Json(ApiResponse::success(
            "Config updated successfully".to_string(),
        )),
        Err(e) => Json(ApiResponse::error(format!("Error saving config: {}", e))),
    }
}

/// Execute a command with the video file path as an argument
/// This function provides basic security by validating the command and path
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
        "Executing {} command: {} with args: {:?}",
        if is_success { "success" } else { "failure" },
        program,
        args
    );

    // Execute the command with a timeout to prevent hanging
    let mut child = Command::new(program)
        .args(&args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;

    // Wait for completion with a reasonable timeout
    let timeout_duration = std::time::Duration::from_secs(30);
    match tokio::time::timeout(timeout_duration, child.wait()).await {
        Ok(Ok(status)) => {
            if status.success() {
                println!(
                    "{} command completed successfully",
                    if is_success { "Success" } else { "Failure" }
                );
            } else {
                eprintln!(
                    "{} command failed with exit code: {:?}",
                    if is_success { "Success" } else { "Failure" },
                    status.code()
                );
            }
        }
        Ok(Err(e)) => {
            eprintln!(
                "Error executing {} command: {}",
                if is_success { "success" } else { "failure" },
                e
            );
        }
        Err(_) => {
            // Timeout occurred, kill the process
            let _ = child.kill().await;
            eprintln!(
                "{} command timed out after {} seconds",
                if is_success { "Success" } else { "Failure" },
                timeout_duration.as_secs()
            );
        }
    }

    Ok(())
}
