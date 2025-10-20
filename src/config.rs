// atci (andrew's transcript and clipping interface)
// Copyright (C) 2025 Andrew Nissen

use crate::auth::AuthGuard;
use crate::files;
use crate::web::ApiResponse;
use rocket::serde::json::Json;
use rocket::{get, post};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::process::Command;

fn default_true() -> bool {
    true
}

fn default_stream_chunk_size() -> u32 {
    60
}

fn default_hostname() -> String {
    "http://localhost:4620".to_string()
}

// Color defaults
fn default_color_buffer_bg() -> String {
    "#020617".to_string()
}

fn default_color_header_bg() -> String {
    "#1e3a8a".to_string()
}

fn default_color_text_primary() -> String {
    "#e2e8f0".to_string()
}

fn default_color_border_primary() -> String {
    "#60a5fa".to_string()
}

fn default_color_selection() -> String {
    "#ffff00".to_string()
}

fn default_color_success() -> String {
    "#00ff00".to_string()
}

fn default_color_disabled() -> String {
    "#808080".to_string()
}

fn default_color_info() -> String {
    "#00ffff".to_string()
}

fn default_color_error() -> String {
    "#ff0000".to_string()
}

fn default_color_text_highlight() -> String {
    "#ffffff".to_string()
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
    #[serde(default = "default_stream_chunk_size")]
    pub stream_chunk_size: u32,
    #[serde(default = "default_hostname")]
    pub hostname: String,
    // Color configuration
    #[serde(default = "default_color_buffer_bg")]
    pub color_buffer_bg: String,
    #[serde(default = "default_color_header_bg")]
    pub color_header_bg: String,
    #[serde(default = "default_color_text_primary")]
    pub color_text_primary: String,
    #[serde(default = "default_color_border_primary")]
    pub color_border_primary: String,
    #[serde(default = "default_color_selection")]
    pub color_selection: String,
    #[serde(default = "default_color_success")]
    pub color_success: String,
    #[serde(default = "default_color_disabled")]
    pub color_disabled: String,
    #[serde(default = "default_color_info")]
    pub color_info: String,
    #[serde(default = "default_color_error")]
    pub color_error: String,
    #[serde(default = "default_color_text_highlight")]
    pub color_text_highlight: String,
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
            stream_chunk_size: 60,
            hostname: "http://localhost:4620".to_string(),
            color_buffer_bg: default_color_buffer_bg(),
            color_header_bg: default_color_header_bg(),
            color_text_primary: default_color_text_primary(),
            color_border_primary: default_color_border_primary(),
            color_selection: default_color_selection(),
            color_success: default_color_success(),
            color_disabled: default_color_disabled(),
            color_info: default_color_info(),
            color_error: default_color_error(),
            color_text_highlight: default_color_text_highlight(),
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

pub fn get_config_path_sha() -> String {
    let config_path = if let Ok(config_path) = std::env::var("ATCI_CONFIG_PATH") {
        std::path::PathBuf::from(config_path)
    } else {
        confy::get_configuration_file_path("atci", "config")
            .unwrap_or_else(|_| std::path::PathBuf::from("default"))
    };

    let mut hasher = Sha256::new();
    hasher.update(config_path.to_string_lossy().as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)[..8].to_string()
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
        "stream_chunk_size" => {
            cfg.stream_chunk_size = value
                .parse::<u32>()
                .map_err(|_| format!("Invalid number value for stream_chunk_size: {}", value))?;
        }
        "hostname" => cfg.hostname = value.to_string(),
        "color_buffer_bg" => cfg.color_buffer_bg = validate_hex_color(value)?,
        "color_header_bg" => cfg.color_header_bg = validate_hex_color(value)?,
        "color_text_primary" => cfg.color_text_primary = validate_hex_color(value)?,
        "color_border_primary" => cfg.color_border_primary = validate_hex_color(value)?,
        "color_selection" => cfg.color_selection = validate_hex_color(value)?,
        "color_success" => cfg.color_success = validate_hex_color(value)?,
        "color_disabled" => cfg.color_disabled = validate_hex_color(value)?,
        "color_info" => cfg.color_info = validate_hex_color(value)?,
        "color_error" => cfg.color_error = validate_hex_color(value)?,
        "color_text_highlight" => cfg.color_text_highlight = validate_hex_color(value)?,
        _ => return Err(format!("Unknown field: {}", field)),
    }
    Ok(())
}

fn validate_hex_color(hex: &str) -> Result<String, String> {
    let hex = hex.trim();
    if !hex.starts_with('#') || (hex.len() != 7 && hex.len() != 4) {
        return Err(format!("Invalid hex color format: {}. Expected #RRGGBB or #RGB", hex));
    }

    // Validate hex characters
    for c in hex.chars().skip(1) {
        if !c.is_ascii_hexdigit() {
            return Err(format!("Invalid hex color: {} contains non-hex character", hex));
        }
    }

    Ok(hex.to_string())
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

/// Execute a command with the video file path piped as input in detached mode
/// The command will continue running after atci exits
pub fn execute_processing_command(
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

    println!("Running command: {}", command);

    // Create the command with detached process configuration
    // Parse the command string to run it as a full shell command
    let mut cmd = if cfg!(target_os = "windows") {
        let mut c = Command::new("cmd");
        c.args(["/C", command]);
        c
    } else {
        let mut c = Command::new("sh");
        c.args(["-c", command]);
        c
    };
    cmd.stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .stdin(std::process::Stdio::piped()); // Enable piped input

    // Configure for detached execution
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x00000008); // DETACHED_PROCESS
    }

    // Spawn the command with piped stdin
    let mut child = cmd.spawn()?;

    // Get and output the process ID
    let pid = child.id();
    println!(
        "{} command spawned successfully with PID {} and running detached with piped input",
        if is_success { "Success" } else { "Failure" },
        pid
    );

    // Write the video path to stdin and close it
    if let Some(stdin) = child.stdin.take() {
        use std::io::Write;
        let video_path_str = format!("{}\n", video_path.display());
        let _ = std::thread::spawn(move || {
            let mut stdin = stdin;
            let _ = stdin.write_all(video_path_str.as_bytes());
            let _ = stdin.flush();
            // stdin is automatically closed when it goes out of scope
        });
    }

    // Don't call child.wait() - let it run independently

    Ok(())
}
