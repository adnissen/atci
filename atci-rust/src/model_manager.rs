const MODEL_NAMES: &[&str] = &[
    "ggml-base-q5_1",
    "ggml-base-q8_0",
    "ggml-base",
    "ggml-base.en-q5_1",
    "ggml-base.en-q8_0",
    "ggml-base.en",
    "ggml-large-v1",
    "ggml-large-v2-q5_0",
    "ggml-large-v2-q8_0",
    "ggml-large-v2",
    "ggml-large-v3-q5_0",
    "ggml-large-v3-turbo-q5_0",
    "ggml-large-v3-turbo-q8_0",
    "ggml-large-v3-turbo",
    "ggml-large-v3",
    "ggml-medium-q5_0",
    "ggml-medium-q8_0",
    "ggml-medium",
    "ggml-medium.en-q5_0",
    "ggml-medium.en-q8_0",
    "ggml-medium.en",
    "ggml-small-q5_1",
    "ggml-small-q8_0",
    "ggml-small",
    "ggml-small.en-q5_1",
    "ggml-small.en-q8_0",
    "ggml-small.en",
    "ggml-tiny-q5_1",
    "ggml-tiny-q8_0",
    "ggml-tiny",
    "ggml-tiny.en-q5_1",
    "ggml-tiny.en-q8_0",
    "ggml-tiny.en",
];

const HUGGINGFACE_BASE_URL: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main";

#[derive(Debug, serde::Serialize)]
pub struct ModelInfo {
    pub name: String,
    pub downloaded: bool,
    pub path: String,
    pub configured: bool,
}

pub fn models_directory() -> std::path::PathBuf {
    let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    std::path::Path::new(&home_dir).join(".atci").join("models")
}

pub fn list_models() -> Vec<ModelInfo> {
    let models_dir = models_directory();
    std::fs::create_dir_all(&models_dir).ok();

    let cfg: crate::AtciConfig = crate::config::load_config_or_default();
    let configured_model = &cfg.model_name;

    MODEL_NAMES.iter().map(|&model_name| {
        let model_path = models_dir.join(format!("{}.bin", model_name));
        
        ModelInfo {
            name: model_name.to_string(),
            downloaded: model_path.exists(),
            path: model_path.to_string_lossy().to_string(),
            configured: model_name == configured_model,
        }
    }).collect()
}

pub fn download_model(model_name: &str) -> Result<String, Box<dyn std::error::Error>> {
    if !MODEL_NAMES.contains(&model_name) {
        return Err("Invalid model name".into());
    }

    let models_dir = models_directory();
    std::fs::create_dir_all(&models_dir)?;

    let model_path = models_dir.join(format!("{}.bin", model_name));
    let url = format!("{}/{}.bin", HUGGINGFACE_BASE_URL, model_name);

    println!("Downloading model {} from {}", model_name, url);

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()?;

    let response = client.get(&url)
        .send()?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()).into());
    }

    let bytes = response.bytes()?;
    std::fs::write(&model_path, bytes)?;

    Ok(model_path.to_string_lossy().to_string())
}

use rocket::serde::json::Json;
use rocket::{get, post};
use crate::web::ApiResponse;

#[derive(serde::Deserialize)]
pub struct DownloadModelRequest {
    model: String,
}

#[get("/api/models/list")]
pub fn web_list_models() -> Json<ApiResponse<Vec<ModelInfo>>> {
    let models = list_models();
    Json(ApiResponse::success(models))
}

#[post("/api/models/download", data = "<request>")]
pub fn web_download_model(request: Json<DownloadModelRequest>) -> Json<ApiResponse<String>> {
    match download_model(&request.model) {
        Ok(path) => Json(ApiResponse::success(path)),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}