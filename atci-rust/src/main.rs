use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::thread;
use std::time::Duration;
use std::io::Write;
use globset::{Glob, GlobSetBuilder};
use walkdir::WalkDir;
use chrono::{DateTime, Local};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None, arg_required_else_help = true)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Api {
        #[command(subcommand)]
        api_command: Option<ApiCommands>,
    },
    Watch,
}

#[derive(Subcommand, Debug)]
#[command(arg_required_else_help = true)]
enum ApiCommands {
    VideoInfo {
        #[command(subcommand)]
        video_info_command: Option<VideoInfoCommands>,
    },
    Queue {
        #[command(subcommand)]
        queue_command: Option<QueueCommands>,
    },
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

fn get_queue() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let queue_path = std::path::Path::new(&home_dir).join(".queue");
    if !queue_path.exists() {
        return Ok(Vec::new());
    }
    
    let content = fs::read_to_string(queue_path)?;
    let queue: Vec<String> = content.lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect();
    Ok(queue)
}

fn add_to_queue(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let existing_queue = get_queue()?;
    
    if existing_queue.contains(&path.to_string()) {
        return Ok(());
    }
    
    let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let queue_path = std::path::Path::new(&home_dir).join(".queue");
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(queue_path)?;
    writeln!(file, "{}", path)?;
    Ok(())
}

fn remove_first_line_from_queue() -> Result<(), Box<dyn std::error::Error>> {
    let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let queue_path = std::path::Path::new(&home_dir).join(".queue");
    
    if !queue_path.exists() {
        return Ok(());
    }
    
    let content = fs::read_to_string(&queue_path)?;
    let lines: Vec<&str> = content.lines().collect();
    
    if lines.len() <= 1 {
        fs::write(&queue_path, "")?;
    } else {
        let remaining_lines = lines[1..].join("\n");
        if !remaining_lines.is_empty() {
            fs::write(&queue_path, format!("{}\n", remaining_lines))?;
        } else {
            fs::write(&queue_path, "")?;
        }
    }
    
    Ok(())
}

fn process_queue() -> Result<(), Box<dyn std::error::Error>> {
    thread::spawn(|| {
        loop {
           let _ = process_queue_iteration();
           thread::sleep(Duration::from_secs(2));
        }
    });
    Ok(())
}

fn process_queue_iteration() -> Result<bool, Box<dyn std::error::Error>> {
    let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let queue_path = std::path::Path::new(&home_dir).join(".queue");
    if !queue_path.exists() {
        return Ok(false);
    }
    
    let content = fs::read_to_string(&queue_path)?;
    let first_line = content.lines().next();
    if let Some(video_path_str) = first_line {
        let video_path_str = video_path_str.trim();
        if video_path_str.is_empty() {
            remove_first_line_from_queue()?;
            return Ok(false);
        }
        
        let video_path = Path::new(video_path_str);
        
        if !video_path.exists() {
            eprintln!("Video file does not exist: {}", video_path_str);
            remove_first_line_from_queue()?;
            return Ok(true);
        }
        
        let video_extensions = get_video_extensions();
        let has_valid_extension = video_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| video_extensions.contains(&ext.to_lowercase().as_str()))
            .unwrap_or(false);
        
        if !has_valid_extension {
            eprintln!("File does not have a valid video extension: {}", video_path_str);
            remove_first_line_from_queue()?;
            return Ok(true);
        }
        
        let txt_path = video_path.with_extension("txt");
        let meta_path = video_path.with_extension("meta");
        
        if !txt_path.exists() {
            fs::write(&txt_path, "")?;
            println!("Created empty .txt file: {}", txt_path.display());
        }
        
        if !meta_path.exists() {
            fs::write(&meta_path, "")?;
            println!("Created empty .meta file: {}", meta_path.display());
        }
        
        remove_first_line_from_queue()?;
        println!("Processed queue item: {}", video_path_str);
        return Ok(true);
    }
    
    Ok(false)
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

fn watch_for_missing_metadata(cfg: &AtciConfig) -> Result<(), Box<dyn std::error::Error>> {
    let cfg_clone = cfg.clone();
    thread::spawn(move || {
        let video_extensions = get_video_extensions();
        
        if cfg_clone.watch_directories.is_empty() {
            eprintln!("No watch directories configured");
            return;
        }
        
        loop {
            for watch_directory in &cfg_clone.watch_directories {
                for entry in WalkDir::new(watch_directory).into_iter().filter_map(|e| e.ok()) {
                    let file_path = entry.path();

                    //skip directories
                    if !file_path.is_file() {
                        continue;
                    }

                    if let Some(extension) = file_path.extension() {
                        let ext_str = extension.to_string_lossy().to_lowercase();

                        // we're only interested in video files
                        if !video_extensions.contains(&ext_str.as_str()) {
                            continue;
                        }

                        // we want to make sure the file isn't in the process of currently being copied over to our watch directory
                        // since there isn't any way to actually tell for sure via an api call, a useful proxy for this is that the file hasn't been modified in the last 3 seconds
                        if let Ok(metadata) = fs::metadata(&file_path) {
                            if let Ok(modified) = metadata.modified() {
                                let now = std::time::SystemTime::now();
                                if let Ok(duration) = now.duration_since(modified) {
                                    if duration.as_secs() >= 3 {
                                        let txt_path = file_path.with_extension("txt");
                                        let meta_path = file_path.with_extension("meta");
                                        
                                        if !txt_path.exists() && !meta_path.exists() {
                                            if let Err(e) = add_to_queue(&file_path.to_string_lossy()) {
                                                eprintln!("Error adding to queue: {}", e);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            thread::sleep(Duration::from_secs(2));
        }
    });
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    match args.command {
        Some(Commands::Api { api_command }) => {
            match api_command {
                Some(ApiCommands::VideoInfo { video_info_command }) => {
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
                Some(ApiCommands::Queue { queue_command }) => {
                    match queue_command {
                        Some(QueueCommands::Get) => {
                            match get_queue() {
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
                            match add_to_queue(&path) {
                                Ok(()) => {
                                    println!("Added to queue: {}", path);
                                }
                                Err(e) => {
                                    eprintln!("Error adding to queue: {}", e);
                                    std::process::exit(1);
                                }
                            }
                        }
                        None => {}
                    }
                }
                None => {}
            }
        }
        Some(Commands::Watch) => {
            let cfg: AtciConfig = confy::load("atci", "config")?;
            watch_for_missing_metadata(&cfg)?;
            process_queue()?;
            
            // Keep the main thread alive while the background threads run
            loop {
                thread::sleep(Duration::from_secs(60));
            }
        }
        None => {}
    }
    
    Ok(())
}