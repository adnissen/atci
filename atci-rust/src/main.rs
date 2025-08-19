use clap::Parser;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use globset::{Glob, GlobSet, GlobSetBuilder};
use walkdir::WalkDir;
use chrono::{DateTime, Local};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct AtciConfig {
    pub ffmpeg_path: String,
    pub ffprobe_path: String,
    pub model_name: String,
    pub nonlocal_password: Option<String>,
    pub watch_directories: Vec<String>,
    pub whispercli_path: String,
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

#[derive(Serialize, Debug)]
struct VideoInfo {
    name: String,
    base_name: String,
    created_at: String,
    line_count: usize,
    full_path: String,
    transcript: bool,
    last_generated: Option<String>,
    length: Option<String>,
    model: Option<String>,
}

fn format_datetime(timestamp: std::time::SystemTime) -> String {
    let datetime: DateTime<Local> = timestamp.into();
    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
}

fn get_video_extensions() -> Vec<&'static str> {
    vec!["mp4", "avi", "mov", "mkv", "wmv", "flv", "webm", "m4v"]
}

fn get_meta_fields(meta_path: &Path, fields: &[&str]) -> Vec<Option<String>> {
    let mut results = vec![None; fields.len()];
    if let Ok(content) = fs::read_to_string(meta_path) {
        for line in content.lines() {
            for (i, field) in fields.iter().enumerate() {
                if results[i].is_none() && line.starts_with(&format!("{}:", field)) {
                    // Split only on the first colon, then trim whitespace
                    if let Some(value) = line.splitn(2, ':').nth(1) {
                        results[i] = Some(value.trim().to_string());
                    }
                }
            }
            // Early exit if all fields found
            if results.iter().all(|r| r.is_some()) {
                break;
            }
        }
    }
    results
}

fn get_video_info_from_disk(cfg: &AtciConfig) -> Result<Vec<VideoInfo>, Box<dyn std::error::Error>> {
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
        println!("Processing watch directory: {}", watch_directory);
        
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
                        
                        // get all of the meta fields at once so we don't have to keep reading the file
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

fn process_watch_directories() -> Result<(), Box<dyn std::error::Error>> {
    let cfg: AtciConfig = confy::load("atci", "config")?;
    
    let video_infos = get_video_info_from_disk(&cfg)?;
    let json_output = serde_json::to_string_pretty(&video_infos)?;
    println!("{}", json_output);
    
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _args = Args::parse();
    let cfg: AtciConfig = confy::load("atci", "config")?;
    println!("Loaded config path: {:?}", confy::get_configuration_file_path("atci", "config")?);
    
    process_watch_directories()?;
    
    Ok(())
}