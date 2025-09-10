// atci (andrew's transcript and clipping interface)
// Copyright (C) 2025 Andrew Nissen

use std::fs;
use walkdir::WalkDir;
use serde::Serialize;
use crate::{config, config::AtciConfig};
use rayon::prelude::*;
use rocket::serde::json::Json;
use rocket::get;
use crate::web::ApiResponse;
use crate::auth::AuthGuard;
use crate::files::VideoInfo;
use crate::metadata;
use chrono::{DateTime, Local};

#[derive(Debug, Serialize)]
pub struct SearchMatch {
    pub line_number: usize,
    pub line_text: String,
    pub timestamp: Option<String>,
    pub video_info: VideoInfo,
}

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub file_path: String,
    pub matches: Vec<SearchMatch>,
}

fn format_datetime(timestamp: std::time::SystemTime) -> String {
    let datetime: DateTime<Local> = timestamp.into();
    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
}

pub fn search(query: &str, filter: Option<&Vec<String>>) -> Result<Vec<SearchResult>, Box<dyn std::error::Error>> {
    let cfg: AtciConfig = config::load_config()?;
    let video_extensions = crate::files::get_video_extensions();
    
    let filtered_directories = cfg.watch_directories.clone();
    
    let all_entries: Vec<_> = filtered_directories
        .iter()
        .flat_map(|watch_directory| {
            WalkDir::new(watch_directory)
                .into_iter()
                .filter_map(|e| e.ok())
                .collect::<Vec<_>>()
        })
        .collect();

    let mut results: Vec<SearchResult> = all_entries
        .par_iter()
        .filter_map(|entry| {
            let file_path = entry.path();
            
            if !file_path.is_file() {
                return None;
            }
            
            let extension = file_path.extension()?;
            let ext_str = extension.to_str()?;
            
            if !video_extensions.contains(&ext_str.to_lowercase().as_str()) {
                return None;
            }
            
            // Apply file path filter if provided
            if let Some(filters) = filter {
                if !filters.is_empty() {
                    let file_path_str = file_path.to_string_lossy().to_lowercase();
                    if !filters.iter().any(|f| file_path_str.contains(&f.trim().to_lowercase())) {
                        return None;
                    }
                }
            }
            
            let txt_path = file_path.with_extension("txt");
            
            if !txt_path.exists() {
                return None;
            }
            
            let content = fs::read_to_string(&txt_path).ok()?;
            let lines: Vec<&str> = content.lines().collect();
            
            // Create VideoInfo for this file
            let metadata = fs::metadata(&file_path).ok()?;
            let filename = file_path.file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            
            let line_count = lines.len();
            let transcript_exists = true; // We know it exists since we're reading it
            
            let last_generated = fs::metadata(&txt_path)
                .ok()
                .and_then(|meta| meta.modified().ok())
                .map(format_datetime);
            
            let (length, model) = {
                let metadata_fields = metadata::get_metadata_fields(file_path).unwrap_or_default();
                (metadata_fields.length, metadata_fields.source)
            };
            
            let created_at = metadata.created()
                .or_else(|_| metadata.modified())
                .map(format_datetime)
                .unwrap_or_else(|_| "Unknown".to_string());
            
            let video_info = VideoInfo {
                name: file_path.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                base_name: filename,
                created_at,
                line_count,
                full_path: file_path.to_string_lossy().to_string(),
                transcript: transcript_exists,
                last_generated,
                length,
                source: model,
            };
            
            let matches: Vec<SearchMatch> = lines
                .iter()
                .enumerate()
                .filter_map(|(line_num, line)| {
                    if line.to_lowercase().contains(&query.to_lowercase()) {
                        // Check if the previous line contains a timestamp
                        let timestamp = if line_num > 0 {
                            let prev_line = lines[line_num - 1];
                            // Check if the previous line looks like a timestamp (contains digits and colons)
                            if prev_line.contains(':') && prev_line.chars().any(|c| c.is_ascii_digit()) {
                                Some(prev_line.to_string())
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        
                        Some(SearchMatch {
                            line_number: line_num + 1,
                            line_text: line.to_string(),
                            timestamp,
                            video_info: video_info.clone(),
                        })
                    } else {
                        None
                    }
                })
                .collect();
            
            if matches.is_empty() {
                None
            } else {
                Some(SearchResult {
                    file_path: file_path.to_string_lossy().to_string(),
                    matches,
                })
            }
        })
        .collect();

    results.sort_by(|a, b| a.file_path.cmp(&b.file_path));

    Ok(results)
}

#[get("/api/search?<query>&<filter>")]
pub fn web_search_transcripts(_auth: AuthGuard, query: String, filter: Option<Vec<String>>) -> Json<ApiResponse<serde_json::Value>> {
    match search(&query, filter.as_ref()) {
        Ok(results) => Json(ApiResponse::success(serde_json::to_value(results).unwrap_or_default())),
        Err(e) => Json(ApiResponse::error(format!("Search failed: {}", e))),
    }
}