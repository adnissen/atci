use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::thread;
use std::time::Duration;
use globset::{Glob, GlobSetBuilder};
use walkdir::WalkDir;
use chrono::{DateTime, Local};
//use rust_embed::Embed;

mod clipper;
mod queue;
mod video_processor;

//#[derive(Embed)]
//#[folder = "assets/"]
//struct Asset;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None, arg_required_else_help = true)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    VideoInfo {
        #[command(subcommand)]
        video_info_command: Option<VideoInfoCommands>,
    },
    Queue {
        #[command(subcommand)]
        queue_command: Option<QueueCommands>,
    },
    Clip {
        #[arg(help = "Path to the video file")]
        path: String,
        #[arg(help = "Start time in seconds (e.g., 455.00)")]
        start: f64,
        #[arg(help = "End time in seconds (e.g., 520.50)")]
        end: f64,
        #[arg(help = "Optional text to overlay")]
        text: Option<String>,
        #[arg(long, help = "Display text overlay", default_value = "true")]
        display_text: bool,
        #[arg(long, help = "Output format: gif, mp3, or mp4", value_parser = ["gif", "mp3", "mp4"], default_value = "mp4")]
        format: String,
        #[arg(long, help = "Font size for text overlay")]
        font_size: Option<u32>,
    },
    Watch,
    Config,
}


#[derive(Subcommand, Debug)]
#[command(arg_required_else_help = true)]
enum VideoInfoCommands {
    Get,
    Update,
}

#[derive(Subcommand, Debug)]
#[command(arg_required_else_help = true)]
enum QueueCommands {
    Get,
    Add {
        #[arg(help = "Path to add to the queue")]
        path: String,
    },
    Status,
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

#[derive(Serialize, Deserialize, Debug)]
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

fn get_cache_file_path() -> std::path::PathBuf {
    let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    std::path::Path::new(&home_dir).join(".atci_video_info_cache.json")
}

fn save_video_info_to_cache(video_infos: &[VideoInfo]) -> Result<(), Box<dyn std::error::Error>> {
    let cache_path = get_cache_file_path();
    let json_data = serde_json::to_string_pretty(video_infos)?;
    fs::write(cache_path, json_data)?;
    Ok(())
}

fn load_video_info_from_cache() -> Result<Vec<VideoInfo>, Box<dyn std::error::Error>> {
    let cache_path = get_cache_file_path();
    let json_data = fs::read_to_string(cache_path)?;
    let video_infos: Vec<VideoInfo> = serde_json::from_str(&json_data)?;
    Ok(video_infos)
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


fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    match args.command {
        Some(Commands::VideoInfo { video_info_command }) => {
            match video_info_command {
                Some(VideoInfoCommands::Get) => {
                    match load_video_info_from_cache() {
                        Ok(video_infos) => {
                            let json_output = serde_json::to_string_pretty(&video_infos)?;
                            println!("{}", json_output);
                        }
                        Err(e) => {
                            eprintln!("Error reading cache file: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                Some(VideoInfoCommands::Update) => {
                    let cfg: AtciConfig = confy::load("atci", "config")?;
                    let video_infos = get_video_info_from_disk(&cfg)?;
                    save_video_info_to_cache(&video_infos)?;
                    let json_output = serde_json::to_string_pretty(&video_infos)?;
                    println!("{}", json_output);
                }
                None => {}
            }
        }
        Some(Commands::Queue { queue_command }) => {
            match queue_command {
                Some(QueueCommands::Get) => {
                    match queue::get_queue() {
                        Ok(queue) => {
                            let json_output = serde_json::to_string_pretty(&queue)?;
                            println!("{}", json_output);
                        }
                        Err(e) => {
                            eprintln!("Error reading queue: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                Some(QueueCommands::Add { path }) => {
                    match queue::add_to_queue(&path) {
                        Ok(()) => {
                            println!("Added to queue: {}", path);
                        }
                        Err(e) => {
                            eprintln!("Error adding to queue: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                Some(QueueCommands::Status) => {
                    match queue::get_queue_status() {
                        Ok((path, age)) => {
                            let result = serde_json::json!({
                                "currently_processing": path.unwrap_or_else(|| "".to_string()),
                                "age_in_seconds": age
                            });
                            println!("{}", result);
                        }
                        Err(e) => {
                            eprintln!("Error reading queue status: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                None => {}
            }
        }
        Some(Commands::Clip { path, start, end, text, display_text, format, font_size }) => {
            clipper::clip(Path::new(&path), start, end, text.as_deref(), display_text, &format, font_size)?;
        }
        Some(Commands::Watch) => {
            let cfg: AtciConfig = confy::load("atci", "config")?;
            queue::watch_for_missing_metadata(&cfg)?;
            queue::process_queue()?;
            
            // Keep the main thread alive while the background threads run
            loop {
                thread::sleep(Duration::from_secs(60));
            }
        }
        Some(Commands::Config) => {
            let cfg: AtciConfig = confy::load("atci", "config")?;
            let json_output = serde_json::to_string_pretty(&cfg)?;
            println!("{}", json_output);
        }
        None => {}
    }
    
    Ok(())
}