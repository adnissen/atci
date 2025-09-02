// atci (andrew's transcript and clipping interface)
// Copyright (C) 2025 Andrew Nissen

use serde::{Deserialize, Serialize};
use rocket::serde::json::Json;
use rocket::{get, post};
use crate::web::ApiResponse;
use crate::auth::AuthGuard;

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
    if let Ok(config_path) = std::env::var("ATCI_CONFIG_PATH") {
        confy::store_path(&config_path, config)
    } else {
        confy::store("atci", "config", config)
    }
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
        "watch_directories" => {
            // For watch_directories, treat the value as a single directory to add
            if !cfg.watch_directories.contains(&value.to_string()) {
                cfg.watch_directories.push(value.to_string());
            }
        },
        _ => return Err(format!("Unknown field: {}", field)),
    }
    Ok(())
}

#[post("/api/config", data = "<config>")]
pub fn web_set_config(_auth: AuthGuard, config: Json<AtciConfig>) -> Json<ApiResponse<String>> {
    match store_config(&config) {
        Ok(()) => Json(ApiResponse::success("Config updated successfully".to_string())),
        Err(e) => Json(ApiResponse::error(format!("Error saving config: {}", e))),
    }
}