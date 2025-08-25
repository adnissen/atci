use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use globset::{Glob, GlobSetBuilder};
use walkdir::WalkDir;
use chrono::{DateTime, Local};
use rocket::serde::json::Json;
use rocket::get;
use crate::web::ApiResponse;
use crate::config::AtciConfig;

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

fn get_meta_fields(meta_path: &Path, fields: &[&str]) -> Vec<Option<String>> {
    let mut results = vec![None; fields.len()];
    if let Ok(content) = fs::read_to_string(meta_path) {
        for line in content.lines() {
            for (i, field) in fields.iter().enumerate() {
                if results[i].is_none() && line.starts_with(&format!("{}:", field)) {
                    if let Some(value) = line.splitn(2, ':').nth(1) {
                        results[i] = Some(value.trim().to_string());
                    }
                }
            }
            if results.iter().all(|r| r.is_some()) {
                break;
            }
        }
    }
    results
}

pub fn get_cache_file_path() -> std::path::PathBuf {
    let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    std::path::Path::new(&home_dir).join(".atci_video_info_cache.json")
}

pub fn save_video_info_to_cache(video_infos: &[VideoInfo]) -> Result<(), Box<dyn std::error::Error>> {
    let cache_path = get_cache_file_path();
    let json_data = serde_json::to_string_pretty(video_infos)?;
    fs::write(cache_path, json_data)?;
    Ok(())
}

pub fn load_video_info_from_cache(filter: Option<&Vec<String>>) -> Result<Vec<VideoInfo>, Box<dyn std::error::Error>> {
    let cache_path = get_cache_file_path();
    let json_data = fs::read_to_string(cache_path)?;
    let mut video_infos: Vec<VideoInfo> = serde_json::from_str(&json_data)?;
    
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
    let mut video_infos = Vec::new();

    for watch_directory in &cfg.watch_directories {
        for entry in WalkDir::new(watch_directory).into_iter().filter_map(|e| e.ok()) {
            let file_path = entry.path();
            
            if file_path.is_file() {
                let relative_path = file_path.strip_prefix(watch_directory)
                    .unwrap_or(file_path)
                    .to_string_lossy()
                    .to_string();
                
                if globset.is_match(&relative_path) {
                    if let Ok(metadata) = fs::metadata(&file_path) {
                        let filename = file_path.file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        
                        let txt_path = file_path.with_extension("txt");
                        let meta_path = file_path.with_extension("meta");
                        
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
                            let fields = ["length", "source"];
                            let results = get_meta_fields(&meta_path, &fields);
                            (results[0].clone(), results[1].clone())
                        } else {
                            (None, None)
                        };
                        
                        let created_at = metadata.created()
                            .or_else(|_| metadata.modified())
                            .map(format_datetime)
                            .unwrap_or_else(|_| "Unknown".to_string());
                        
                        video_infos.push(VideoInfo {
                            name: relative_path,
                            base_name: filename,
                            created_at,
                            line_count,
                            full_path: file_path.to_string_lossy().to_string(),
                            transcript: transcript_exists,
                            last_generated,
                            length,
                            model,
                        });
                    }
                }
            }
        }
    }
    
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