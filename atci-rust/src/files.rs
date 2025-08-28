use serde::{Deserialize, Serialize};
use std::fs;
use globset::{Glob, GlobSetBuilder};
use walkdir::WalkDir;
use chrono::{DateTime, Local};
use rocket::serde::json::Json;
use rocket::get;
use crate::web::ApiResponse;
use crate::config::AtciConfig;
use rayon::prelude::*;
use crate::metadata;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VideoInfo {
    pub name: String,
    pub base_name: String,
    pub created_at: String,
    pub line_count: usize,
    pub full_path: String,
    pub transcript: bool,
    pub last_generated: Option<String>,
    pub length: Option<String>,
    pub model: Option<String>,
}

fn format_datetime(timestamp: std::time::SystemTime) -> String {
    let datetime: DateTime<Local> = timestamp.into();
    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
}

pub fn get_video_extensions() -> Vec<&'static str> {
    vec!["mp4", "avi", "mov", "mkv", "wmv", "flv", "webm", "m4v"]
}

pub fn get_cache_file_path() -> std::path::PathBuf {
    let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    std::path::Path::new(&home_dir).join(".atci_video_info_cache.msgpack")
}

pub fn save_video_info_to_cache(video_infos: &[VideoInfo]) -> Result<(), Box<dyn std::error::Error>> {
    let cache_path = get_cache_file_path();
    let msgpack_data = rmp_serde::to_vec(video_infos)?;
    fs::write(cache_path, msgpack_data)?;
    Ok(())
}

pub fn load_video_info_from_cache(filter: Option<&Vec<String>>) -> Result<Vec<VideoInfo>, Box<dyn std::error::Error>> {
    let cache_path = get_cache_file_path();
    
    let msgpack_data = match fs::read(&cache_path) {
        Ok(data) => data,
        Err(_) => {
            // File doesn't exist, create it with empty array
            let empty_array: Vec<VideoInfo> = Vec::new();
            let empty_data = rmp_serde::to_vec(&empty_array)?;
            fs::write(&cache_path, &empty_data)?;
            empty_data
        }
    };
    
    let mut video_infos: Vec<VideoInfo> = rmp_serde::from_slice(&msgpack_data)?;
    
    if let Some(filters) = filter {
        if !filters.is_empty() {
            video_infos.retain(|info| {
                let full_path_lower = info.full_path.to_lowercase();
                filters.iter().any(|f| full_path_lower.contains(&f.to_lowercase()))
            });
        }
    }
    
    Ok(video_infos)
}

pub fn get_video_info_from_disk(cfg: &AtciConfig) -> Result<Vec<VideoInfo>, Box<dyn std::error::Error>> {
    if cfg.watch_directories.is_empty() {
        return Ok(Vec::new());
    }

    let mut builder = GlobSetBuilder::new();
    let video_extensions = get_video_extensions();
    
    for ext in &video_extensions {
        let pattern = format!("**/*.{}", ext);
        builder.add(Glob::new(&pattern)?);
    }
    
    let globset = builder.build()?;

    let all_entries: Vec<_> = cfg.watch_directories
        .iter()
        .flat_map(|watch_directory| {
            WalkDir::new(watch_directory)
                .into_iter()
                .filter_map(|e| e.ok())
                .map(|entry| (entry, watch_directory.clone()))
                .collect::<Vec<_>>()
        })
        .collect();

    let mut video_infos: Vec<VideoInfo> = all_entries
        .par_iter()
        .filter_map(|(entry, watch_directory)| {
            let file_path = entry.path();
            
            if !file_path.is_file() {
                return None;
            }
            
            let relative_path = file_path.strip_prefix(watch_directory)
                .unwrap_or(file_path)
                .to_string_lossy()
                .to_string();
            
            if !globset.is_match(&relative_path) {
                return None;
            }
            
            let metadata = fs::metadata(&file_path).ok()?;
            let filename = file_path.file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            
            let txt_path = file_path.with_extension("txt");
            
            let transcript_exists = txt_path.exists();
            
            let (line_count, last_generated) = if transcript_exists {
                let line_count = fs::read_to_string(&txt_path)
                    .map(|content| content.lines().count())
                    .unwrap_or(0);
                
                let last_generated = fs::metadata(&txt_path)
                    .ok()
                    .and_then(|meta| meta.modified().ok())
                    .map(format_datetime);
                
                (line_count, last_generated)
            } else {
                (0, None)
            };
            
            let (length, model) = if transcript_exists {
                let metadata = metadata::get_metadata_fields(file_path);
                (metadata.clone().unwrap().length.clone(), metadata.clone().unwrap().source.clone())
            } else {
                (None, None)
            };
            
            let created_at = metadata.created()
                .or_else(|_| metadata.modified())
                .map(format_datetime)
                .unwrap_or_else(|_| "Unknown".to_string());
            
            Some(VideoInfo {
                name: relative_path,
                base_name: filename,
                created_at,
                line_count,
                full_path: file_path.to_string_lossy().to_string(),
                transcript: transcript_exists,
                last_generated,
                length,
                model,
            })
        })
        .collect();
    
    video_infos.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(video_infos)
}

#[get("/api/files?<filter>")]
pub fn web_get_files(filter: Option<String>) -> Json<ApiResponse<serde_json::Value>> {
    let filter_vec = filter.map(|f| f.split(',').map(|s| s.trim().to_string()).collect::<Vec<String>>());
    match load_video_info_from_cache(filter_vec.as_ref()) {
        Ok(video_infos) => Json(ApiResponse::success(serde_json::to_value(video_infos).unwrap_or_default())),
        Err(e) => Json(ApiResponse::error(format!("Failed to load video info cache: {}", e))),
    }
}

#[get("/api/sources")]
pub fn web_get_sources() -> Json<ApiResponse<serde_json::Value>> {
    let cfg = match crate::config::load_config() {
        Ok(config) => config,
        Err(e) => return Json(ApiResponse::error(format!("Failed to load config: {}", e))),
    };
    
    match get_video_info_from_disk(&cfg) {
        Ok(video_infos) => {
            let mut sources: std::collections::HashSet<String> = std::collections::HashSet::new();
            
            for info in video_infos {
                if let Some(model) = info.model {
                    if !model.is_empty() {
                        sources.insert(model);
                    }
                }
            }
            
            let mut unique_sources: Vec<String> = sources.into_iter().collect();
            unique_sources.sort();
            
            Json(ApiResponse::success(serde_json::to_value(unique_sources).unwrap_or_default()))
        },
        Err(e) => Json(ApiResponse::error(format!("Failed to get video info from disk: {}", e))),
    }
}