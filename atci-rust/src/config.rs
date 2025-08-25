use serde::{Deserialize, Serialize};
use rocket::serde::json::Json;
use rocket::{get, State};
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