// atci (andrew's transcript and clipping interface)
// Copyright (C) 2025 Andrew Nissen

use crate::auth::AuthGuard;
use crate::clipper;
use crate::files::VideoInfo;
use crate::metadata;
use crate::web::ApiResponse;
use crate::{config, config::AtciConfig};
use chrono::{DateTime, Local};
use rayon::prelude::*;
use rocket::get;
use rocket::serde::json::Json;
use serde::Serialize;
use std::fs;
use walkdir::WalkDir;

#[derive(Debug, Serialize)]
pub struct SearchMatch {
    pub line_number: usize,
    pub line_text: String,
    pub timestamp: Option<String>,
    pub video_info: VideoInfo,
    pub clip_path: Option<String>,
    pub clip_command: Option<String>,
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

fn normalize_apostrophes(text: &str) -> String {
    text
        // Replace right single quotation mark (U+2019) with regular apostrophe
        .replace(['\u{2019}', '\u{2018}', '\u{00B4}', '`'], "'")
}

fn generate_clip_for_match(
    file_path: &std::path::Path,
    timestamp_line: &str,
    format: &str,
    text: Option<&str>,
) -> (Option<String>, Option<String>) {
    // Parse timestamp from line like "126: 00:05:25.920 --> 00:05:46.060"
    if let Some(timestamp_range) = parse_timestamp_range(timestamp_line) {
        let (start_time, end_time) = timestamp_range;
        
        // Generate clip using the clipper module
        let display_text = text.is_some();
        match clipper::clip(
            file_path,
            &start_time,
            &end_time,
            text,      // Pass text for GIF overlay
            display_text, // Display text for GIFs
            format,    // Use specified format (mp4 or gif)
            None,      // No custom font size
        ) {
            Ok(clip_path) => {
                let clip_command = if let Some(text_content) = text {
                    format!(
                        "atci clip \"{}\" {} {} \"{}\" --format {}",
                        file_path.display(),
                        start_time,
                        end_time,
                        text_content.replace('"', "\\\""),
                        format
                    )
                } else {
                    format!(
                        "atci clip \"{}\" {} {} --format {}",
                        file_path.display(),
                        start_time,
                        end_time,
                        format
                    )
                };
                (
                    Some(clip_path.to_string_lossy().to_string()),
                    Some(clip_command),
                )
            }
            Err(e) => {
                eprintln!("Warning: Failed to generate {} for {}: {}", format, file_path.display(), e);
                (None, None)
            }
        }
    } else {
        (None, None)
    }
}

fn parse_timestamp_range(timestamp_line: &str) -> Option<(String, String)> {
    // Parse lines like "51: 00:01:07.220 --> 00:01:10.680" or "00:01:07.220 --> 00:01:10.680"

    // First check if line contains the arrow separator
    if let Some(_arrow_pos) = timestamp_line.find(" --> ") {
        // Check if it has a number prefix (subtitle format): "51: 00:01:07.220 --> 00:01:10.680"
        if let Some(colon_pos) = timestamp_line.find(": ") {
            let timestamp_part = &timestamp_line[colon_pos + 2..];
            let start_end: Vec<&str> = timestamp_part.split(" --> ").collect();
            if start_end.len() == 2 {
                return Some((start_end[0].to_string(), start_end[1].to_string()));
            }
        } else {
            // Direct format: "00:01:07.220 --> 00:01:10.680"
            let start_end: Vec<&str> = timestamp_line.split(" --> ").collect();
            if start_end.len() == 2 {
                return Some((start_end[0].to_string(), start_end[1].to_string()));
            }
        }
    }
    None
}

pub fn search(
    query: &str,
    filter: Option<&Vec<String>>,
    generate_clips: bool,
    generate_gifs: bool,
) -> Result<Vec<SearchResult>, Box<dyn std::error::Error>> {
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
            if let Some(filters) = filter
                && !filters.is_empty()
            {
                let file_path_str = file_path.to_string_lossy().to_lowercase();
                if !filters
                    .iter()
                    .any(|f| file_path_str.contains(&f.trim().to_lowercase()))
                {
                    return None;
                }
            }

            let txt_path = file_path.with_extension("txt");

            if !txt_path.exists() {
                return None;
            }

            let content = fs::read_to_string(&txt_path).ok()?;
            let lines: Vec<&str> = content.lines().collect();

            // Create VideoInfo for this file
            let metadata = fs::metadata(file_path).ok()?;
            let filename = file_path
                .file_stem()
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

            let created_at = metadata
                .created()
                .or_else(|_| metadata.modified())
                .map(format_datetime)
                .unwrap_or_else(|_| "Unknown".to_string());

            let video_info = VideoInfo {
                name: file_path
                    .file_name()
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

            let normalized_query = normalize_apostrophes(&query.to_lowercase());

            let matches: Vec<SearchMatch> = lines
                .iter()
                .enumerate()
                .filter_map(|(line_num, line)| {
                    let normalized_line = normalize_apostrophes(&line.to_lowercase());
                    if normalized_line.contains(&normalized_query) {
                        // Check if the previous line contains a timestamp
                        let timestamp = if line_num > 0 {
                            let prev_line = lines[line_num - 1];
                            // Check if the previous line looks like a timestamp (contains digits and colons)
                            if prev_line.contains(':')
                                && prev_line.chars().any(|c| c.is_ascii_digit())
                            {
                                Some(prev_line.to_string())
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        // Generate clip if requested and timestamp is available
                        let (clip_path, clip_command) = if (generate_clips || generate_gifs) && timestamp.is_some() {
                            let format = if generate_gifs { "gif" } else { "mp4" };
                            let text_for_clip = if generate_gifs { Some(*line) } else { None };
                            generate_clip_for_match(file_path, &timestamp.as_ref().unwrap(), format, text_for_clip)
                        } else {
                            (None, None)
                        };

                        Some(SearchMatch {
                            line_number: line_num + 1,
                            line_text: line.to_string(),
                            timestamp,
                            video_info: video_info.clone(),
                            clip_path,
                            clip_command,
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
pub fn web_search_transcripts(
    _auth: AuthGuard,
    query: String,
    filter: Option<Vec<String>>,
) -> Json<ApiResponse<serde_json::Value>> {
    // URL decode the filter strings to handle %2C -> ,
    let decoded_filter = filter.map(|filters| {
        filters
            .iter()
            .flat_map(|f| {
                urlencoding::decode(f)
                    .unwrap_or_else(|_| f.into())
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
            })
            .collect()
    });
    println!("decoded_filter: {:?}", decoded_filter);
    match search(&query, decoded_filter.as_ref(), false, false) {
        Ok(results) => Json(ApiResponse::success(
            serde_json::to_value(results).unwrap_or_default(),
        )),
        Err(e) => Json(ApiResponse::error(format!("Search failed: {}", e))),
    }
}
