// atci (andrew's transcript and clipping interface)
// Copyright (C) 2025 Andrew Nissen

use serde::{Deserialize, Serialize};
use std::fs;
use globset::{Glob, GlobSetBuilder};
use walkdir::WalkDir;
use chrono::{DateTime, Local};
use rocket::serde::json::Json;
use rocket::get;
use crate::web::ApiResponse;
use crate::config;
use rayon::prelude::*;
use crate::metadata;
use crate::auth::AuthGuard;
use rusqlite::{Connection, Result as SqliteResult};

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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CacheData {
    pub files: Vec<VideoInfo>,
    pub sources: Vec<String>,
}

fn format_datetime(timestamp: std::time::SystemTime) -> String {
    let datetime: DateTime<Local> = timestamp.into();
    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
}

pub fn get_video_extensions() -> Vec<&'static str> {
    vec!["mp4", "avi", "mov", "mkv", "wmv", "flv", "webm", "m4v"]
}

pub fn get_db_path() -> std::path::PathBuf {
    let home_dir = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    home_dir.join(".atci/video_info.db")
}

fn init_database(conn: &Connection) -> SqliteResult<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS video_info (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            base_name TEXT NOT NULL,
            created_at TEXT NOT NULL,
            line_count INTEGER NOT NULL,
            full_path TEXT NOT NULL UNIQUE,
            transcript BOOLEAN NOT NULL,
            last_generated TEXT,
            length TEXT,
            model TEXT
        )",
        [],
    )?;
    Ok(())
}

fn get_connection() -> SqliteResult<Connection> {
    let db_path = get_db_path();
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let conn = Connection::open(db_path)?;
    init_database(&conn)?;
    Ok(conn)
}

pub fn load_cache_data() -> Result<CacheData, Box<dyn std::error::Error>> {
    let conn = get_connection()?;
    
    let mut stmt = conn.prepare("SELECT name, base_name, created_at, line_count, full_path, transcript, last_generated, length, model FROM video_info ORDER BY created_at DESC")?;
    let video_iter = stmt.query_map([], |row| {
        Ok(VideoInfo {
            name: row.get(0)?,
            base_name: row.get(1)?,
            created_at: row.get(2)?,
            line_count: row.get(3)?,
            full_path: row.get(4)?,
            transcript: row.get(5)?,
            last_generated: row.get(6)?,
            length: row.get(7)?,
            model: row.get(8)?,
        })
    })?;
    
    let mut files = Vec::new();
    for video in video_iter {
        files.push(video?);
    }
    
    let mut sources_stmt = conn.prepare("SELECT DISTINCT model FROM video_info WHERE model IS NOT NULL AND model != '' ORDER BY model")?;
    let sources_iter = sources_stmt.query_map([], |row| {
        Ok(row.get::<_, String>(0)?)
    })?;
    
    let mut sources = Vec::new();
    for source in sources_iter {
        sources.push(source?);
    }
    
    Ok(CacheData { files, sources })
}

pub fn load_video_info_from_cache(filter: Option<&Vec<String>>) -> Result<Vec<VideoInfo>, Box<dyn std::error::Error>> {
    let cache_data = load_cache_data()?;
    let mut video_infos = cache_data.files;
    
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

pub fn get_and_save_video_info_from_disk() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = config::load_config_or_default();
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

    let video_infos: Vec<VideoInfo> = all_entries
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
    
    // Save to database in a transaction
    let conn = get_connection()?;
    let tx = conn.unchecked_transaction()?;
    
    // Clear existing data
    tx.execute("DELETE FROM video_info", [])?;
    
    // Insert new data
    {
        let mut stmt = tx.prepare("INSERT INTO video_info (name, base_name, created_at, line_count, full_path, transcript, last_generated, length, model) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)")?;
        
        for video in &video_infos {
            stmt.execute((
                &video.name,
                &video.base_name,
                &video.created_at,
                &video.line_count,
                &video.full_path,
                &video.transcript,
                &video.last_generated,
                &video.length,
                &video.model,
            ))?;
        }
    }
    
    tx.commit()?;
    Ok(())
}

#[get("/api/files?<filter>")]
pub fn web_get_files(_auth: AuthGuard, filter: Option<String>) -> Json<ApiResponse<serde_json::Value>> {
    let filter_vec = filter.map(|f| f.split(',').map(|s| s.trim().to_string()).collect::<Vec<String>>());
    match load_video_info_from_cache(filter_vec.as_ref()) {
        Ok(video_infos) => Json(ApiResponse::success(serde_json::to_value(video_infos).unwrap_or_default())),
        Err(e) => Json(ApiResponse::error(format!("Failed to load video info cache: {}", e))),
    }
}

#[get("/api/sources")]
pub fn web_get_sources(_auth: AuthGuard) -> Json<ApiResponse<serde_json::Value>> {
    match load_cache_data() {
        Ok(cache_data) => Json(ApiResponse::success(serde_json::to_value(cache_data.sources).unwrap_or_default())),
        Err(e) => Json(ApiResponse::error(format!("Failed to load cache data: {}", e))),
    }
}