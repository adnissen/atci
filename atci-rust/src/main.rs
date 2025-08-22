use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::thread;
use std::time::Duration;
use std::collections::HashSet;
use globset::{Glob, GlobSetBuilder};
use walkdir::WalkDir;
use chrono::{DateTime, Local};
use dialoguer::{Input, Select};
//use rust_embed::Embed;

mod clipper;
mod queue;
mod video_processor;
mod tools_manager;
mod model_manager;
mod search;

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
    #[command(about = "Manage video information cache")]
    VideoInfo {
        #[command(subcommand)]
        video_info_command: Option<VideoInfoCommands>,
    },
    #[command(about = "Manage video processing queue")]
    Queue {
        #[command(subcommand)]
        queue_command: Option<QueueCommands>,
    },
    #[command(about = "Create video clips with optional text overlay")]
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
    #[command(about = "Manage external tools and dependencies")]
    Tools {
        #[command(subcommand)]
        tools_command: Option<ToolsCommands>,
    },
    #[command(about = "Manage Whisper models")]
    Models {
        #[command(subcommand)]
        models_command: Option<ModelsCommands>,
    },
    #[command(about = "Watch directories for new videos and process them automatically")]
    Watch,
    #[command(about = "Display current configuration settings")]
    Config {
        #[command(subcommand)]
        config_command: Option<ConfigCommands>,
    },
    #[command(about = "Search for content in video transcripts")]
    Search {
        #[arg(help = "Search query", num_args = 1.., value_delimiter = ' ')]
        query: Vec<String>,
        #[arg(long, help = "Show formatted output instead of JSON", default_value = "false")]
        pretty: bool,
    },
}


#[derive(Subcommand, Debug)]
#[command(arg_required_else_help = true)]
enum VideoInfoCommands {
    #[command(about = "Get video information from cache")]
    Get,
    #[command(about = "Update video information cache by scanning watch directories")]
    Update,
}

#[derive(Subcommand, Debug)]
#[command(arg_required_else_help = true)]
enum QueueCommands {
    #[command(about = "Get all items in the processing queue")]
    Get,
    #[command(about = "Add a video path to the processing queue")]
    Add {
        #[arg(help = "Path to add to the queue")]
        path: String,
    },
    #[command(about = "Get current queue processing status")]
    Status,
}

#[derive(Subcommand, Debug)]
#[command(arg_required_else_help = true)]
enum ToolsCommands {
    #[command(about = "List all available tools and their status")]
    List {
        #[arg(long, help = "Show formatted output instead of JSON", default_value = "false")]
        pretty: bool,
    },
    #[command(about = "Download and install a specific tool")]
    Download {
        #[arg(help = "Name of the tool to download")]
        tool: String,
    },
}

#[derive(Subcommand, Debug)]
#[command(arg_required_else_help = true)]
enum ModelsCommands {
    #[command(about = "List all available models and their status")]
    List {
        #[arg(long, help = "Show formatted output instead of JSON", default_value = "false")]
        pretty: bool,
    },
    #[command(about = "Download and install a specific model")]
    Download {
        #[arg(help = "Name of the model to download")]
        model: String,
    },
}

#[derive(Subcommand, Debug)]
enum ConfigCommands {
    #[command(about = "Display current configuration settings")]
    Show,
    #[command(about = "Display path to configuration file")]
    Path,
    #[command(about = "Set a configuration field")]
    Set {
        #[arg(help = "Field name to set")]
        field: String,
        #[arg(help = "Value to set")]
        value: String,
    },
    #[command(about = "Unset/clear a configuration field")]
    Unset {
        #[arg(help = "Field name to unset")]
        field: String,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct AtciConfig {
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

fn is_valid_config_field(field: &str) -> bool {
    matches!(field, "ffmpeg_path" | "ffprobe_path" | "model_name" | "whispercli_path" | "watch_directories" | "nonlocal_password")
}

fn set_config_field(cfg: &mut AtciConfig, field: &str, value: &str) -> Result<(), String> {
    match field {
        "ffmpeg_path" => cfg.ffmpeg_path = value.to_string(),
        "ffprobe_path" => cfg.ffprobe_path = value.to_string(),
        "model_name" => cfg.model_name = value.to_string(),
        "whispercli_path" => cfg.whispercli_path = value.to_string(),
        "nonlocal_password" => cfg.nonlocal_password = Some(value.to_string()),
        "watch_directories" => {
            // For watch_directories, treat the value as a single directory to add
            if !cfg.watch_directories.contains(&value.to_string()) {
                cfg.watch_directories.push(value.to_string());
            }
        },
        _ => return Err(format!("Unknown field: {}", field)),
    }
    Ok(())
}

fn unset_config_field(cfg: &mut AtciConfig, field: &str) -> Result<(), String> {
    match field {
        "ffmpeg_path" => cfg.ffmpeg_path = String::new(),
        "ffprobe_path" => cfg.ffprobe_path = String::new(),
        "model_name" => cfg.model_name = String::new(),
        "whispercli_path" => cfg.whispercli_path = String::new(),
        "nonlocal_password" => cfg.nonlocal_password = None,
        "watch_directories" => cfg.watch_directories.clear(),
        _ => return Err(format!("Unknown field: {}", field)),
    }
    Ok(())
}

fn prompt_for_executable_path(tool: &str, current_path: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Get tool info to check what options are available
    let tools = tools_manager::list_tools();
    let tool_info = tools.iter().find(|t| t.name == tool);
    
    if let Some(info) = tool_info {
        let mut options = Vec::new();
        let mut paths = Vec::new();
        
        // Option 1: Use downloaded version (if available)
        if info.downloaded {
            options.push(format!("Use downloaded {} ({})", tool, info.downloaded_path));
            paths.push(info.downloaded_path.clone());
        }
        
        // Option 2: Use system version (if available)
        if info.system_available {
            if let Some(system_path) = &info.system_path {
                options.push(format!("Use system {} ({})", tool, system_path));
                paths.push(system_path.clone());
            }
        }
        
        // Option 3: Download and use
        options.push(format!("Download {} and use that", tool));
        paths.push("__download__".to_string());
        
        // Option 4: Enter custom path
        options.push("Enter custom path".to_string());
        paths.push("__custom__".to_string());
        
        if options.is_empty() {
            return Err("No options available for this tool".into());
        }
        
        let selection = Select::new()
            .with_prompt(&format!("Select {} configuration", tool))
            .items(&options)
            .default(0)
            .interact()?;
            
        match paths[selection].as_str() {
            "__download__" => {
                println!("Downloading {}...", tool);
                let downloaded_path = tools_manager::download_tool(tool)?;
                println!("Successfully downloaded {} to: {}", tool, downloaded_path);
                Ok(downloaded_path)
            }
            "__custom__" => {
                let custom_path: String = Input::new()
                    .with_prompt(&format!("Enter path to {}", tool))
                    .default(current_path.to_string())
                    .validate_with(|input: &String| validate_executable_path(input))
                    .interact()?;
                Ok(custom_path)
            }
            path => Ok(path.to_string())
        }
    } else {
        // Fallback to simple input if tool info not found
        let custom_path: String = Input::new()
            .with_prompt(&format!("Enter path to {}", tool))
            .default(current_path.to_string())
            .validate_with(|input: &String| validate_executable_path(input))
            .interact()?;
        Ok(custom_path)
    }
}

fn validate_executable_path(path: &str) -> Result<(), String> {
    if path.is_empty() {
        return Err("Path cannot be empty".to_string());
    }
    
    let path_obj = Path::new(path);
    if !path_obj.exists() {
        return Err("Path does not exist".to_string());
    }
    
    if !path_obj.is_file() {
        return Err("Path is not a file".to_string());
    }
    
    // Check if file is executable (Unix-like systems)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = fs::metadata(path) {
            let permissions = metadata.permissions();
            if permissions.mode() & 0o111 == 0 {
                return Err("File is not executable".to_string());
            }
        }
    }
    
    Ok(())
}

fn validate_directory_path(path: &str) -> Result<(), String> {
    if path.is_empty() {
        return Ok(()); // Empty is acceptable for optional directories
    }
    
    let path_obj = Path::new(path);
    if !path_obj.exists() {
        return Err("Directory does not exist".to_string());
    }
    
    if !path_obj.is_dir() {
        return Err("Path is not a directory".to_string());
    }
    
    Ok(())
}

fn validate_and_prompt_config(cfg: &mut AtciConfig, fields_to_verify: &HashSet<String>) -> Result<(), Box<dyn std::error::Error>> {
    let mut config_changed = false;

    if fields_to_verify.contains("ffmpeg_path") && cfg.ffmpeg_path.is_empty() {
        let ffmpeg_path = prompt_for_executable_path("ffmpeg", &cfg.ffmpeg_path)?;
        cfg.ffmpeg_path = ffmpeg_path;
        config_changed = true;
    }

    if fields_to_verify.contains("ffprobe_path") && cfg.ffprobe_path.is_empty() {
        let ffprobe_path = prompt_for_executable_path("ffprobe", &cfg.ffprobe_path)?;
        cfg.ffprobe_path = ffprobe_path;
        config_changed = true;
    }

    if fields_to_verify.contains("whispercli_path") && cfg.whispercli_path.is_empty() {
        let whispercli_path: String = Input::new()
            .with_prompt("Whisper CLI path")
            .validate_with(|input: &String| validate_executable_path(input))
            .interact()?;
        cfg.whispercli_path = whispercli_path;
        config_changed = true;
    }

    if fields_to_verify.contains("model_name") && cfg.model_name.is_empty() {
        let model_name: String = Input::new()
            .with_prompt("Model name")
            .validate_with(|input: &String| {
                if input.is_empty() {
                    Err("Model name cannot be empty")
                } else {
                    Ok(())
                }
            })
            .interact()?;
        cfg.model_name = model_name;
        config_changed = true;
    }

    if fields_to_verify.contains("watch_directories") && cfg.watch_directories.is_empty() {
        let watch_dir: String = Input::new()
            .with_prompt("Watch directory (press Enter to skip)")
            .allow_empty(true)
            .validate_with(|input: &String| validate_directory_path(input))
            .interact()?;
        
        if !watch_dir.is_empty() {
            cfg.watch_directories.push(watch_dir);
            config_changed = true;
        }
    }

    if config_changed {
        confy::store("atci", "config", cfg)?;
        println!("Configuration updated and saved.");
    }

    Ok(())
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
            let mut cfg: AtciConfig = confy::load("atci", "config")?;
            
            // Define required fields for clip command
            let mut required_fields = HashSet::new();
            required_fields.insert("ffmpeg_path".to_string());
            required_fields.insert("ffprobe_path".to_string());
            
            // Validate and prompt for missing configuration
            validate_and_prompt_config(&mut cfg, &required_fields)?;
            
            clipper::clip(Path::new(&path), start, end, text.as_deref(), display_text, &format, font_size)?;
        }
        Some(Commands::Tools { tools_command }) => {
            match tools_command {
                Some(ToolsCommands::List { pretty }) => {
                    let tools = tools_manager::list_tools();
                    if pretty {
                        println!("Tools Status:");
                        println!("{}", "=".repeat(50));
                        for tool in tools {
                            println!("\n🔧 {}", tool.name.to_uppercase());
                            println!("   Platform: {}", tool.platform);
                            println!("   Downloaded: {}", if tool.downloaded { "✅ Yes" } else { "❌ No" });
                            if tool.downloaded {
                                println!("   Downloaded Path: {}", tool.downloaded_path);
                            }
                            println!("   System Available: {}", if tool.system_available { "✅ Yes" } else { "❌ No" });
                            if let Some(system_path) = &tool.system_path {
                                println!("   System Path: {}", system_path);
                            }
                            println!("   Configured Path: {}", tool.current_path);
                        }
                    } else {
                        let json_output = serde_json::to_string_pretty(&tools)?;
                        println!("{}", json_output);
                    }
                }
                Some(ToolsCommands::Download { tool }) => {
                    match tools_manager::download_tool(&tool) {
                        Ok(path) => {
                            println!("Successfully downloaded {} to: {}", tool, path);
                        }
                        Err(e) => {
                            eprintln!("Error downloading {}: {}", tool, e);
                            std::process::exit(1);
                        }
                    }
                }
                None => {}
            }
        }
        Some(Commands::Models { models_command }) => {
            match models_command {
                Some(ModelsCommands::List { pretty }) => {
                    let models = model_manager::list_models();
                    if pretty {
                        let (downloaded, available): (Vec<_>, Vec<_>) = models.iter()
                            .partition(|model| model.downloaded);
                        
                        if !downloaded.is_empty() {
                            println!("📦 INSTALLED MODELS");
                            println!("{}", "=".repeat(50));
                            for model in downloaded {
                                let status = if model.configured { "⭐ " } else { "✅ " };
                                println!("{}{}", status, model.name);
                                if model.configured {
                                    println!("   Status: Currently configured");
                                }
                                println!("   Path: {}", model.path);
                                println!();
                            }
                        }
                        
                        if !available.is_empty() {
                            println!("🔍 AVAILABLE MODELS");
                            println!("{}", "=".repeat(50));
                            for model in available {
                                println!("⬇️  {}", model.name);
                            }
                        }
                    } else {
                        let json_output = serde_json::to_string_pretty(&models)?;
                        println!("{}", json_output);
                    }
                }
                Some(ModelsCommands::Download { model }) => {
                    match model_manager::download_model(&model) {
                        Ok(path) => {
                            println!("Successfully downloaded model {} to: {}", model, path);
                        }
                        Err(e) => {
                            eprintln!("Error downloading model {}: {}", model, e);
                            std::process::exit(1);
                        }
                    }
                }
                None => {}
            }
        }
        Some(Commands::Watch) => {
            let mut cfg: AtciConfig = confy::load("atci", "config")?;
            
            // Define required fields for watch command
            let mut required_fields = HashSet::new();
            required_fields.insert("ffmpeg_path".to_string());
            required_fields.insert("ffprobe_path".to_string());
            required_fields.insert("whispercli_path".to_string());
            required_fields.insert("model_name".to_string());
            required_fields.insert("watch_directories".to_string());
            
            // Validate and prompt for missing configuration
            validate_and_prompt_config(&mut cfg, &required_fields)?;
            
            queue::watch_for_missing_metadata(&cfg)?;
            queue::process_queue()?;
            
            // Keep the main thread alive while the background threads run
            loop {
                thread::sleep(Duration::from_secs(60));
            }
        }
        Some(Commands::Config { config_command }) => {
            match config_command {
                Some(ConfigCommands::Show) => {
                    let cfg: AtciConfig = confy::load("atci", "config")?;
                    let json_output = serde_json::to_string_pretty(&cfg)?;
                    println!("{}", json_output);
                }
                Some(ConfigCommands::Path) => {
                    let config_path = confy::get_configuration_file_path("atci", "config")?;
                    println!("{}", config_path.display());
                }
                Some(ConfigCommands::Set { field, value }) => {
                    if !is_valid_config_field(&field) {
                        eprintln!("Error: Unknown field '{}'. Valid fields are: ffmpeg_path, ffprobe_path, model_name, whispercli_path, watch_directories, nonlocal_password", field);
                        std::process::exit(1);
                    }
                    
                    let mut cfg: AtciConfig = confy::load("atci", "config")?;
                    
                    if let Err(e) = set_config_field(&mut cfg, &field, &value) {
                        eprintln!("Error setting field: {}", e);
                        std::process::exit(1);
                    }
                    
                    confy::store("atci", "config", &cfg)?;
                    println!("Set {} = {}", field, value);
                }
                Some(ConfigCommands::Unset { field }) => {
                    if !is_valid_config_field(&field) {
                        eprintln!("Error: Unknown field '{}'. Valid fields are: ffmpeg_path, ffprobe_path, model_name, whispercli_path, watch_directories, nonlocal_password", field);
                        std::process::exit(1);
                    }
                    
                    let mut cfg: AtciConfig = confy::load("atci", "config")?;
                    
                    if let Err(e) = unset_config_field(&mut cfg, &field) {
                        eprintln!("Error unsetting field: {}", e);
                        std::process::exit(1);
                    }
                    
                    confy::store("atci", "config", &cfg)?;
                    println!("Unset {}", field);
                }
                None => {
                    let cfg: AtciConfig = confy::load("atci", "config")?;
                    let json_output = serde_json::to_string_pretty(&cfg)?;
                    println!("{}", json_output);
                }
            }
        }
        Some(Commands::Search { query, pretty }) => {
            let search_query = query.join(" ");
            let cfg: AtciConfig = confy::load("atci", "config")?;
            
            match search::search(&search_query, &cfg) {
                Ok(results) => {
                    if pretty {
                        for result in results {
                            println!("File: {}", result.file_path);
                            for search_match in result.matches {
                                if let Some(timestamp) = search_match.timestamp {
                                    println!("{}: {}", search_match.line_number, timestamp);
                                    println!("{}:\t{}", search_match.line_number + 1, search_match.line_text);
                                } else {
                                    println!("{}: \"{}\"", search_match.line_number, search_match.line_text);
                                }
                                println!();
                            }
                            println!();
                        }
                    } else {
                        let json_output = serde_json::to_string_pretty(&results)?;
                        println!("{}", json_output);
                    }
                }
                Err(e) => {
                    eprintln!("Error searching: {}", e);
                    std::process::exit(1);
                }
            }
        }
        None => {}
    }
    
    Ok(())
}