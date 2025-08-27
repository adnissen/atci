use serde::{Deserialize, Serialize};
use rocket::serde::json::Json;
use rocket::{get, post, State};
use std::sync::Arc;
use crate::web::ApiResponse;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AtciConfig {
    #[serde(default)]
    pub ffmpeg_path: String,
    #[serde(default)]
    pub ffprobe_path: String,
    #[serde(default)]
    pub model_name: String,
    pub nonlocal_password: Option<String>,
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
            nonlocal_password: None,
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
pub fn web_get_config(config_state: &State<Arc<AtciConfig>>) -> Json<ApiResponse<ConfigResponse>> {
    let config = config_state.inner().as_ref().clone();
    
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

fn is_valid_config_field(field: &str) -> bool {
    matches!(field, "ffmpeg_path" | "ffprobe_path" | "model_name" | "whispercli_path" | "watch_directories" | "nonlocal_password")
}

pub fn set_config_field(cfg: &mut AtciConfig, field: &str, value: &str) -> Result<(), String> {
    match field {
        "ffmpeg_path" => cfg.ffmpeg_path = value.to_string(),
        "ffprobe_path" => cfg.ffprobe_path = value.to_string(),
        "model_name" => cfg.model_name = value.to_string(),
        "whispercli_path" => cfg.whispercli_path = value.to_string(),
        "nonlocal_password" => cfg.nonlocal_password = Some(value.to_string()),
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

#[derive(Deserialize)]
pub struct SetConfigRequest {
    field: String,
    value: String,
}

#[post("/api/config/set", data = "<request>")]
pub fn web_set_config(request: Json<SetConfigRequest>) -> Json<ApiResponse<String>> {
    if !is_valid_config_field(&request.field) {
        return Json(ApiResponse::error(format!(
            "Unknown field '{}'. Valid fields are: ffmpeg_path, ffprobe_path, model_name, whispercli_path, watch_directories, nonlocal_password", 
            request.field
        )));
    }
    
    match load_config() {
        Ok(mut cfg) => {
            if let Err(e) = set_config_field(&mut cfg, &request.field, &request.value) {
                return Json(ApiResponse::error(format!("Error setting field: {}", e)));
            }
            
            if let Err(e) = store_config(&cfg) {
                return Json(ApiResponse::error(format!("Error saving config: {}", e)));
            }
            
            Json(ApiResponse::success(format!("Set {} = {}", request.field, request.value)))
        }
        Err(e) => Json(ApiResponse::error(format!("Error loading config: {}", e))),
    }
}