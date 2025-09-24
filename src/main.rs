// atci (andrew's transcript and clipping interface)
// Copyright (C) 2025 Andrew Nissen

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.

// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use crate::config::AtciConfig;
use clap::{Parser, Subcommand};
use dialoguer::{Input, Select};
use rust_embed::Embed;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::time::Duration;

mod auth;
mod clipper;
mod config;
mod db;
mod files;
mod metadata;
mod model_manager;
mod queue;
mod search;
mod tools_manager;
mod transcripts;
mod tui;
mod video_parts;
mod video_processor;
mod web;

#[derive(Embed)]
#[folder = "assets/"]
pub struct Asset;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None, arg_required_else_help = true)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(about = "Manage file information cache")]
    Files {
        #[command(subcommand)]
        files_command: Option<FilesCommands>,
    },
    #[command(about = "Manage video processing queue")]
    Queue {
        #[command(subcommand)]
        queue_command: Option<QueueCommands>,
    },
    #[command(about = "Create video clips with optional text overlay")]
    #[command(arg_required_else_help = true)]
    Clip {
        #[arg(help = "Path to the video file")]
        path: String,
        #[arg(help = "Start time (seconds: 455.5, frames: 300f, timestamp: 01:30:15.5)")]
        start: String,
        #[arg(help = "End time (seconds: 520.5, frames: 600f, timestamp: 01:35:20.0)")]
        end: String,
        #[arg(help = "Optional text to overlay")]
        text: Option<String>,
        #[arg(long, help = "Display text overlay", default_value = "true")]
        display_text: bool,
        #[arg(long, help = "Output format: gif, mp3, or mp4", value_parser = ["gif", "mp3", "mp4"], default_value = "mp4")]
        format: String,
        #[arg(long, help = "Font size for text overlay")]
        font_size: Option<u32>,
    },
    #[command(about = "Extract a frame from a video with optional text overlay")]
    #[command(arg_required_else_help = true)]
    Frame {
        #[arg(help = "Path to the video file")]
        path: String,
        #[arg(help = "Time (seconds: 455.5, frames: 300f, timestamp: 01:30:15.5)")]
        time: String,
        #[arg(help = "Optional text to overlay")]
        text: Option<String>,
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
    #[command(arg_required_else_help = true)]
    Search {
        #[arg(help = "Search query", num_args = 1.., value_delimiter = ' ')]
        query: Vec<String>,
        #[arg(
            long,
            help = "Show JSON output instead of formatted",
            default_value = "false"
        )]
        json: bool,
        #[arg(
            short = 'f',
            long,
            help = "Comma-separated list of strings to filter results by path",
            value_delimiter = ','
        )]
        filter: Option<Vec<String>>,
        #[arg(
            long,
            help = "Generate clips for each search result and show clip commands",
            default_value = "false"
        )]
        clip: bool,
        #[arg(
            long,
            help = "Generate GIF clips for each search result and show clip commands",
            default_value = "false"
        )]
        gif: bool,
    },
    #[command(about = "Manage video transcripts")]
    Transcripts {
        #[command(subcommand)]
        transcripts_command: Option<TranscriptsCommands>,
    },
    #[command(about = "Launch the web server and watcher")]
    Web {
        #[command(subcommand)]
        web_command: Option<WebCommands>,
    },
    #[command(about = "Update atci to the latest version from GitHub releases")]
    Update,
    #[command(about = "Display version information and check for updates")]
    Version {
        #[arg(
            long,
            help = "Show JSON output instead of formatted",
            default_value = "false"
        )]
        json: bool,
    },
    #[command(about = "Download m3u8 stream in configurable parts for processing")]
    #[command(arg_required_else_help = true)]
    Streamdl {
        #[arg(help = "Name for the stream (used in filename)")]
        stream_name: String,
        #[arg(help = "URL to the m3u8 stream")]
        url: String,
    },
    #[command(about = "Launch TUI interface for browsing videos and transcripts")]
    Tui,
}

#[derive(Subcommand, Debug)]
#[command(arg_required_else_help = true)]
enum FilesCommands {
    #[command(about = "Get file information from cache")]
    Get {
        #[arg(
            short = 'f',
            long,
            help = "Comma-separated list of strings to filter results by path",
            value_delimiter = ','
        )]
        filter: Option<Vec<String>>,
    },
    #[command(about = "Update file information cache by scanning watch directories")]
    Update,
}

#[derive(Subcommand, Debug)]
#[command(arg_required_else_help = true)]
enum QueueCommands {
    #[command(about = "Get all items in the processing queue")]
    Get,
    #[command(about = "Get current queue processing status")]
    Status,
    #[command(about = "Add a path to the blocklist")]
    Block {
        #[arg(help = "Path to add to the blocklist")]
        path: String,
    },
    #[command(about = "Set the queue with new paths")]
    Set {
        #[arg(help = "Paths in desired order", num_args = 1..)]
        paths: Vec<String>,
    },
    #[command(about = "Cancel queue processing")]
    Cancel,
}

#[derive(Subcommand, Debug)]
#[command(arg_required_else_help = true)]
enum ToolsCommands {
    #[command(about = "List all available tools and their status")]
    List {
        #[arg(
            long,
            help = "Show JSON output instead of formatted",
            default_value = "false"
        )]
        json: bool,
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
        #[arg(
            long,
            help = "Show JSON output instead of formatted",
            default_value = "false"
        )]
        json: bool,
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

#[derive(Subcommand, Debug)]
#[command(arg_required_else_help = true)]
enum WebCommands {
    #[command(about = "Launch web server with UI and API")]
    All {
        #[arg(
            short = 'p',
            long,
            help = "Port to run the web server on",
            default_value = "4620"
        )]
        port: u16,
        #[arg(
            long,
            help = "Host to bind the web server to",
            default_value = "127.0.0.1"
        )]
        host: String,
    },
    #[command(about = "Launch API-only server")]
    Api {
        #[arg(
            short = 'p',
            long,
            help = "Port to run the API server on",
            default_value = "4620"
        )]
        port: u16,
        #[arg(
            long,
            help = "Host to bind the API server to",
            default_value = "127.0.0.1"
        )]
        host: String,
    },
}

#[derive(Subcommand, Debug)]
#[command(arg_required_else_help = true)]
enum TranscriptsCommands {
    #[command(about = "Get transcript content for a video file")]
    Get {
        #[arg(help = "Path to the video file")]
        path: String,
    },
    #[command(about = "Set content of a specific line in a transcript file")]
    SetLine {
        #[arg(help = "Path to the video file")]
        video_path: String,
        #[arg(help = "Line number to modify (1-based)")]
        line_number: usize,
        #[arg(help = "New content for the line")]
        content: String,
    },
    #[command(about = "Replace entire content of a transcript file")]
    Set {
        #[arg(help = "Path to the video file")]
        video_path: String,
        #[arg(help = "New content for the entire transcript file")]
        content: String,
    },
    #[command(about = "Delete transcript and meta files to force regeneration")]
    Regenerate {
        #[arg(help = "Path to the video file")]
        video_path: String,
        #[arg(short = 'i', long, help = "Interactive mode with processing options")]
        interactive: bool,
    },
    #[command(about = "Rename both video file and its corresponding transcript file")]
    Rename {
        #[arg(help = "Path to the video file")]
        video_path: String,
        #[arg(help = "New path for the video file")]
        new_path: String,
    },
}

fn is_valid_config_field(field: &str) -> bool {
    matches!(
        field,
        "ffmpeg_path"
            | "ffprobe_path"
            | "model_name"
            | "whispercli_path"
            | "watch_directories"
            | "password"
            | "allow_whisper"
            | "allow_subtitles"
            | "processing_success_command"
            | "processing_failure_command"
            | "stream_chunk_size"
    )
}

fn set_config_field(cfg: &mut AtciConfig, field: &str, value: &str) -> Result<(), String> {
    match field {
        "ffmpeg_path" => cfg.ffmpeg_path = value.to_string(),
        "ffprobe_path" => cfg.ffprobe_path = value.to_string(),
        "model_name" => cfg.model_name = value.to_string(),
        "whispercli_path" => cfg.whispercli_path = value.to_string(),
        "password" => cfg.password = Some(value.to_string()),
        "watch_directories" => {
            // For watch_directories, treat the value as a single directory to add
            if !cfg.watch_directories.contains(&value.to_string()) {
                cfg.watch_directories.push(value.to_string());
            }
        }
        "allow_whisper" => {
            cfg.allow_whisper = value
                .parse::<bool>()
                .map_err(|_| format!("Invalid boolean value for allow_whisper: {}", value))?;
        }
        "allow_subtitles" => {
            cfg.allow_subtitles = value
                .parse::<bool>()
                .map_err(|_| format!("Invalid boolean value for allow_subtitles: {}", value))?;
        }
        "processing_success_command" => cfg.processing_success_command = value.to_string(),
        "processing_failure_command" => cfg.processing_failure_command = value.to_string(),
        "stream_chunk_size" => {
            cfg.stream_chunk_size = value
                .parse::<u32>()
                .map_err(|_| format!("Invalid number value for stream_chunk_size: {}", value))?;
        }
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
        "password" => cfg.password = None,
        "watch_directories" => cfg.watch_directories.clear(),
        "allow_whisper" => cfg.allow_whisper = true,
        "allow_subtitles" => cfg.allow_subtitles = true,
        "processing_success_command" => cfg.processing_success_command = String::new(),
        "processing_failure_command" => cfg.processing_failure_command = String::new(),
        "stream_chunk_size" => cfg.stream_chunk_size = 60,
        _ => return Err(format!("Unknown field: {}", field)),
    }
    Ok(())
}

fn prompt_for_executable_path(
    tool: &str,
    current_path: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    // Get tool info to check what options are available
    let tools = tools_manager::list_tools();
    let tool_info = tools.iter().find(|t| t.name == tool);

    if let Some(info) = tool_info {
        let mut options = Vec::new();
        let mut paths = Vec::new();

        // Option 1: Use downloaded version (if available)
        if info.downloaded {
            options.push(format!(
                "Use downloaded {} ({})",
                tool, info.downloaded_path
            ));
            paths.push(info.downloaded_path.clone());
        }

        // Option 2: Use system version (if available)
        if info.system_available
            && let Some(system_path) = &info.system_path
        {
            options.push(format!("Use system {} ({})", tool, system_path));
            paths.push(system_path.clone());
        }

        // Option 3: Download and use
        options.push(format!("Download and use {}", tool));
        paths.push("__download__".to_string());

        // Option 4: Enter custom path
        options.push("Enter custom path".to_string());
        paths.push("__custom__".to_string());

        if options.is_empty() {
            return Err("No options available for this tool".into());
        }

        let selection = Select::new()
            .with_prompt(format!("Select {} configuration", tool))
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
                    .with_prompt(format!("Enter path to {}", tool))
                    .default(current_path.to_string())
                    .validate_with(|input: &String| validate_executable_path(input))
                    .interact()?;
                Ok(custom_path)
            }
            path => Ok(path.to_string()),
        }
    } else {
        // Fallback to simple input if tool info not found
        let custom_path: String = Input::new()
            .with_prompt(format!("Enter path to {}", tool))
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

fn validate_model_path(path: &str) -> Result<(), String> {
    if path.is_empty() {
        return Err("Path cannot be empty".to_string());
    }

    let path_obj = Path::new(path);
    if !path_obj.exists() {
        return Err("Model file does not exist".to_string());
    }

    if !path_obj.is_file() {
        return Err("Path is not a file".to_string());
    }

    Ok(())
}

fn prompt_for_model_name(current_model: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Get model info to check what options are available
    let models = model_manager::list_models();

    let mut options = Vec::new();
    let mut values = Vec::new();

    // Group models: downloaded first, then available
    let (downloaded, available): (Vec<_>, Vec<_>) =
        models.iter().partition(|model| model.downloaded);

    // Add downloaded models first
    if !downloaded.is_empty() {
        for model in downloaded {
            let status = if model.configured {
                " (currently configured)"
            } else {
                ""
            };
            options.push(format!(
                "Use downloaded {} ({}){}",
                model.name, model.path, status
            ));
            values.push(model.name.clone());
        }
    }

    // Add available models for download
    if !available.is_empty() {
        for model in available {
            options.push(format!("Download and use {}", model.name));
            values.push(format!("__download__{}", model.name));
        }
    }

    // Option to enter custom path
    options.push("Enter custom model file path".to_string());
    values.push("__custom__".to_string());

    if options.is_empty() {
        return Err("No model options available".into());
    }

    let selection = Select::new()
        .with_prompt("Select model configuration")
        .items(&options)
        .default(0)
        .interact()?;

    match values[selection].as_str() {
        value if value.starts_with("__download__") => {
            let model_name = &value["__download__".len()..];
            println!("Downloading model {}...", model_name);
            let downloaded_path = model_manager::download_model(model_name)?;
            println!(
                "Successfully downloaded {} to: {}",
                model_name, downloaded_path
            );
            Ok(model_name.to_string())
        }
        "__custom__" => {
            let custom_path: String = Input::new()
                .with_prompt("Enter path to model file")
                .default(current_model.to_string())
                .validate_with(|input: &String| validate_model_path(input))
                .interact()?;
            Ok(custom_path)
        }
        model_name => Ok(model_name.to_string()),
    }
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

fn get_atci_dir() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let home_dir = dirs::home_dir().ok_or("Could not find home directory")?;
    let atci_dir = home_dir.join(".atci");

    if !atci_dir.exists() {
        fs::create_dir_all(&atci_dir)?;
    }

    Ok(atci_dir)
}

fn get_pid_file_path(pid: u32) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let atci_dir = get_atci_dir()?;
    let config_sha = config::get_config_path_sha();
    Ok(atci_dir.join(format!("atci.{}.{}.pid", config_sha, pid)))
}

fn find_existing_pid_files() -> Result<Vec<u32>, Box<dyn std::error::Error>> {
    let atci_dir = get_atci_dir()?;
    let config_sha = config::get_config_path_sha();
    let mut pids = Vec::new();

    if atci_dir.exists() {
        for entry in fs::read_dir(atci_dir)? {
            let entry = entry?;
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();

            let expected_prefix = format!("atci.{}.", config_sha);
            if file_name_str.starts_with(&expected_prefix) && file_name_str.ends_with(".pid") {
                let pid_str = &file_name_str[expected_prefix.len()..file_name_str.len() - 4]; // Remove prefix and ".pid" suffix
                if let Ok(pid) = pid_str.parse::<u32>() {
                    pids.push(pid);
                }
            }
        }
    }

    Ok(pids)
}

fn is_process_running(pid: u32) -> bool {
    #[cfg(unix)]
    {
        use std::process::Command;
        let output = Command::new("ps").arg("-p").arg(pid.to_string()).output();

        match output {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }

    #[cfg(windows)]
    {
        use std::process::Command;
        let output = Command::new("tasklist")
            .arg("/FI")
            .arg(format!("PID eq {}", pid))
            .output();

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                stdout.contains(&pid.to_string())
            }
            Err(_) => false,
        }
    }
}

fn handle_existing_pid_files() -> Result<(), Box<dyn std::error::Error>> {
    let existing_pids = find_existing_pid_files()?;

    if existing_pids.is_empty() {
        return Ok(());
    }

    let (running_pids, stale_pids): (Vec<&u32>, Vec<&u32>) = existing_pids
        .iter()
        .partition(|&&pid| is_process_running(pid));

    if !running_pids.is_empty() {
        if running_pids.len() == 1 {
            println!(
                "Another atci process is already running (PID: {})",
                running_pids[0]
            );
        } else {
            println!(
                "Multiple atci processes are already running (PIDs: {:?})",
                running_pids
            );
        }
        println!();

        let options = vec![
            if running_pids.len() == 1 {
                "Kill the existing process and continue"
            } else {
                "Kill all existing processes and continue"
            },
            "Start anyway (WARNING: may cause undefined behavior)",
            "Quit",
        ];

        let selection = Select::new()
            .with_prompt("What would you like to do?")
            .items(&options)
            .default(2)
            .interact()?;

        match selection {
            0 => {
                let mut all_killed = true;
                for &pid in &running_pids {
                    #[cfg(unix)]
                    {
                        use std::process::Command;
                        let result = Command::new("kill").arg(pid.to_string()).output();

                        match result {
                            Ok(output) if output.status.success() => {
                                println!("Successfully killed process {}", pid);
                                if let Ok(pid_file_path) = get_pid_file_path(*pid) {
                                    let _ = fs::remove_file(&pid_file_path);
                                }
                            }
                            _ => {
                                eprintln!("Failed to kill process {}", pid);
                                all_killed = false;
                            }
                        }
                    }

                    #[cfg(windows)]
                    {
                        use std::process::Command;
                        let result = Command::new("taskkill")
                            .arg("/F")
                            .arg("/PID")
                            .arg(pid.to_string())
                            .output();

                        match result {
                            Ok(output) if output.status.success() => {
                                println!("Successfully killed process {}", pid);
                                if let Ok(pid_file_path) = get_pid_file_path(*pid) {
                                    let _ = fs::remove_file(&pid_file_path);
                                }
                            }
                            _ => {
                                eprintln!("Failed to kill process {}", pid);
                                all_killed = false;
                            }
                        }
                    }
                }

                if !all_killed {
                    std::process::exit(1);
                }
            }
            1 => {
                println!(
                    "   WARNING: Starting with existing PID files may cause undefined behavior!"
                );
                println!("   Multiple instances may conflict with each other.");
                println!();
            }
            _ => {
                println!("Exiting...");
                std::process::exit(0);
            }
        }
    }

    if !stale_pids.is_empty() {
        if stale_pids.len() == 1 {
            println!(
                "Found stale PID file (process {} is not running)",
                stale_pids[0]
            );
        } else {
            println!(
                "Found {} stale PID files (processes not running: {:?})",
                stale_pids.len(),
                stale_pids
            );
        }
        println!();

        let options = vec![
            if stale_pids.len() == 1 {
                "Delete the stale PID file and continue"
            } else {
                "Delete all stale PID files and continue"
            },
            "Start anyway with our own PID file",
            "Quit",
        ];

        let selection = Select::new()
            .with_prompt("What would you like to do?")
            .items(&options)
            .default(0)
            .interact()?;

        match selection {
            0 => {
                for &pid in &stale_pids {
                    if let Ok(pid_file_path) = get_pid_file_path(*pid) {
                        let _ = fs::remove_file(&pid_file_path);
                    }
                }
                println!(
                    "Deleted stale PID file{}",
                    if stale_pids.len() == 1 { "" } else { "s" }
                );
            }
            1 => {
                println!(
                    "Continuing with existing PID file{} present",
                    if stale_pids.len() == 1 { "" } else { "s" }
                );
            }
            _ => {
                println!("Exiting...");
                std::process::exit(0);
            }
        }
    }

    Ok(())
}

fn create_pid_file() -> Result<(), Box<dyn std::error::Error>> {
    let current_pid = std::process::id();
    let pid_file_path = get_pid_file_path(current_pid)?;

    // Create empty file (PID is in filename)
    fs::File::create(&pid_file_path)?;

    println!(
        "Created PID file: {} (PID: {})",
        pid_file_path.display(),
        current_pid
    );
    Ok(())
}

fn cleanup_pid_file() {
    let current_pid = std::process::id();
    if let Ok(pid_file_path) = get_pid_file_path(current_pid)
        && pid_file_path.exists()
        && let Err(e) = fs::remove_file(&pid_file_path)
    {
        eprintln!("Warning: Failed to remove PID file: {}", e);
    }
}

fn setup_pid_file_management() -> Result<(), Box<dyn std::error::Error>> {
    handle_existing_pid_files()?;
    create_pid_file()?;

    // Set up cleanup handler
    ctrlc::set_handler(move || {
        println!("\nReceived interrupt signal, cleaning up pid file");
        cleanup_pid_file();
        std::process::exit(0);
    })?;

    Ok(())
}

fn update() -> Result<(), Box<dyn std::error::Error>> {
    use self_update::cargo_crate_version;

    // Determine the target and binary name based on the current platform
    let (target, bin_name) = if cfg!(target_os = "windows") && cfg!(target_arch = "x86_64") {
        ("windows-x86_64", "atci.exe")
    } else if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        ("macos-aarch64", "atci")
    } else {
        return Err(format!(
            "Self-update is only supported for Windows x86_64 and macOS aarch64. Current platform: {}-{}",
            std::env::consts::OS,
            std::env::consts::ARCH
        ).into());
    };

    let status = self_update::backends::github::Update::configure()
        .repo_owner("adnissen")
        .repo_name("atci")
        .bin_name(bin_name)
        .target(target)
        .show_download_progress(true)
        .current_version(cargo_crate_version!())
        .build()?
        .update()?;

    println!("Update status: `{}`", status.version());
    Ok(())
}

pub fn get_version_info() -> Result<(String, String, bool), Box<dyn std::error::Error>> {
    use self_update::cargo_crate_version;

    let current_version = cargo_crate_version!();

    // Check for latest release
    let (latest_version, update_available) =
        match self_update::backends::github::ReleaseList::configure()
            .repo_owner("adnissen")
            .repo_name("atci")
            .build()
            .and_then(|r| r.fetch())
        {
            Ok(releases) => {
                let latest_release = releases.first();
                let latest_version = latest_release
                    .map(|r| r.version.as_str())
                    .unwrap_or("unknown");
                let update_available = latest_release
                    .map(|r| r.version.as_str() != current_version)
                    .unwrap_or(false);
                (latest_version.to_string(), update_available)
            }
            Err(_) => {
                // If we can't fetch releases (repository doesn't exist, network issues, etc.)
                ("unknown".to_string(), false)
            }
        };

    Ok((
        current_version.to_string(),
        latest_version,
        update_available,
    ))
}

pub fn check_version(json: bool) -> Result<(), Box<dyn std::error::Error>> {
    use serde_json::json;

    let (current_version, latest_version, update_available) = get_version_info()?;

    if json {
        let version_info = json!({
            "current_version": current_version,
            "latest_version": latest_version,
            "update_available": update_available
        });
        println!("{}", serde_json::to_string_pretty(&version_info)?);
    } else {
        println!("github: {}", latest_version);
        println!("local: {}", current_version);
    }

    Ok(())
}

fn validate_and_prompt_config(
    cfg: &mut AtciConfig,
    fields_to_verify: &HashSet<String>,
) -> Result<(), Box<dyn std::error::Error>> {
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
        let whispercli_path = prompt_for_executable_path("whisper-cli", &cfg.whispercli_path)?;
        cfg.whispercli_path = whispercli_path;
        config_changed = true;
    }

    if fields_to_verify.contains("model_name") && cfg.model_name.is_empty() {
        let model_name = prompt_for_model_name(&cfg.model_name)?;
        cfg.model_name = model_name;
        config_changed = true;
    }

    if fields_to_verify.contains("watch_directories") && cfg.watch_directories.is_empty() {
        let watch_dir: String = Input::new()
            .with_prompt("Watch directory (press Enter to create and use ~/atci_videos)")
            .allow_empty(true)
            .validate_with(|input: &String| validate_directory_path(input))
            .interact()?;

        if !watch_dir.is_empty() {
            cfg.watch_directories.push(watch_dir);
            config_changed = true;
        } else {
            // Create atci_videos directory in home directory and use it
            let home_dir = dirs::home_dir().ok_or("Could not find home directory")?;
            let atci_videos_dir = home_dir.join("atci_videos");

            if !atci_videos_dir.exists() {
                fs::create_dir_all(&atci_videos_dir)?;
                println!("Created directory: {}", atci_videos_dir.display());
            }

            cfg.watch_directories
                .push(atci_videos_dir.to_string_lossy().to_string());
            config_changed = true;
        }
    }

    if config_changed {
        config::store_config(cfg)?;
        println!("Configuration updated and saved.");
    }

    Ok(())
}

async fn download_stream(url: &str, stream_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    use chrono::Utc;
    use std::fs;

    println!("Starting stream download: {} -> {}", url, stream_name);

    // Get or create streams directory structure
    let home_dir = dirs::home_dir().ok_or("Could not find home directory")?;
    let base_streams_dir = home_dir.join("atci_streams");
    let stream_dir = base_streams_dir.join(stream_name);

    if !stream_dir.exists() {
        fs::create_dir_all(&stream_dir)?;
        println!("Created stream directory: {}", stream_dir.display());
    }

    // Load and update config to include this specific stream directory in watch directories
    let mut cfg: AtciConfig = config::load_config()?;
    let stream_dir_str = stream_dir.to_string_lossy().to_string();

    if !cfg.watch_directories.contains(&stream_dir_str) {
        cfg.watch_directories.push(stream_dir_str.clone());
        config::store_config(&cfg)?;
        println!(
            "Added stream directory to watch directories: {}",
            stream_dir_str
        );
    }

    // Validate required tools
    if cfg.ffmpeg_path.is_empty() {
        return Err(
            "FFmpeg path not configured. Please run 'atci config set ffmpeg_path /path/to/ffmpeg'"
                .into(),
        );
    }

    // Generate timestamp for this stream session
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");

    // Determine file extension from stream (default to ts for m3u8)
    let extension = if url.contains(".m3u8") { "ts" } else { "mp4" };

    println!(
        "Downloading stream in {}-second parts...",
        cfg.stream_chunk_size
    );
    println!("Output directory: {}", stream_dir.display());
    println!(
        "File pattern: {}.{}.partX.{}",
        stream_name, timestamp, extension
    );

    // Use FFmpeg's segment muxer to automatically split the stream into configurable-second parts
    let output_pattern = stream_dir.join(format!(
        "{}.{}.part%d.{}",
        stream_name, timestamp, extension
    ));

    println!("Starting continuous stream download with automatic segmentation...");
    println!("Output pattern: {}", output_pattern.display());
    println!("Press Ctrl+C to stop the download");

    let mut cmd = tokio::process::Command::new(&cfg.ffmpeg_path);
    cmd.args([
        "-i",
        url,
        "-c",
        "copy", // Copy streams without re-encoding
        "-avoid_negative_ts",
        "make_zero", // Handle timestamp issues
        "-f",
        "segment", // Use segment muxer
        "-segment_time",
        &cfg.stream_chunk_size.to_string(), // configurable second segments
        "-segment_format",
        "mpegts", // Output format for segments
        "-segment_start_number",
        "1", // Start numbering from 1
        "-reset_timestamps",
        "1",  // Reset timestamps for each segment
        "-y", // Overwrite output files
        output_pattern.to_str().unwrap(),
    ]);

    // For live streams, we want to run FFmpeg in a way that we can monitor it
    let mut child = cmd.spawn()?;

    // Wait for the process to complete or be interrupted
    let status = child.wait().await?;

    if !status.success() {
        return Err("FFmpeg process failed".into());
    }

    println!("Stream download completed!");
    println!("Files saved to: {}", stream_dir.display());
    println!("The video parts will be automatically processed by the queue system.");

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    match args.command {
        Some(Commands::Files { files_command }) => match files_command {
            Some(FilesCommands::Get { filter }) => {
                match files::load_video_info_from_cache(filter.as_ref()) {
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
            Some(FilesCommands::Update) => {
                files::get_and_save_video_info_from_disk()?;
                let cache_data = files::load_cache_data()?;
                let json_output = serde_json::to_string_pretty(&cache_data.files)?;
                println!("{}", json_output);
            }
            None => {}
        },
        Some(Commands::Queue { queue_command }) => match queue_command {
            Some(QueueCommands::Get) => match queue::get_queue(None) {
                Ok(queue) => {
                    let json_output = serde_json::to_string_pretty(&queue)?;
                    println!("{}", json_output);
                }
                Err(e) => {
                    eprintln!("Error reading queue: {}", e);
                    std::process::exit(1);
                }
            },
            Some(QueueCommands::Status) => match queue::get_queue_status(None) {
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
            },
            Some(QueueCommands::Block { path }) => match queue::add_to_blocklist(&path) {
                Ok(()) => {
                    println!("Added to blocklist: {}", path);
                }
                Err(e) => {
                    eprintln!("Error adding to blocklist: {}", e);
                    std::process::exit(1);
                }
            },
            Some(QueueCommands::Set { paths }) => match queue::set_queue(paths) {
                Ok(()) => {
                    println!("Queue set successfully");
                }
                Err(e) => {
                    eprintln!("Error setting queue: {}", e);
                    std::process::exit(1);
                }
            },
            Some(QueueCommands::Cancel) => match queue::cancel_queue() {
                Ok(message) => {
                    println!("{}", message);
                }
                Err(e) => {
                    eprintln!("Error canceling queue: {}", e);
                    std::process::exit(1);
                }
            },
            None => {}
        },
        Some(Commands::Clip {
            path,
            start,
            end,
            text,
            display_text,
            format,
            font_size,
        }) => {
            let mut cfg: AtciConfig = config::load_config()?;

            // Define required fields for clip command
            let mut required_fields = HashSet::new();
            required_fields.insert("ffmpeg_path".to_string());
            required_fields.insert("ffprobe_path".to_string());

            // Validate and prompt for missing configuration
            validate_and_prompt_config(&mut cfg, &required_fields)?;

            let output_path = clipper::clip(
                Path::new(&path),
                &start,
                &end,
                text.as_deref(),
                display_text,
                &format,
                font_size,
            )?;
            println!("{}", output_path.display());
        }
        Some(Commands::Frame {
            path,
            time,
            text,
            font_size,
        }) => {
            let mut cfg: AtciConfig = config::load_config()?;

            // Define required fields for frame command
            let mut required_fields = HashSet::new();
            required_fields.insert("ffmpeg_path".to_string());
            required_fields.insert("ffprobe_path".to_string());

            // Validate and prompt for missing configuration
            validate_and_prompt_config(&mut cfg, &required_fields)?;

            let output_path =
                clipper::grab_frame(Path::new(&path), &time, text.as_deref(), font_size)?;
            println!("{}", output_path.display());
        }
        Some(Commands::Tools { tools_command }) => match tools_command {
            Some(ToolsCommands::List { json }) => {
                let tools = tools_manager::list_tools();
                if json {
                    let json_output = serde_json::to_string_pretty(&tools)?;
                    println!("{}", json_output);
                } else {
                    println!("Tools Status:");
                    println!("{}", "=".repeat(50));
                    for tool in tools {
                        println!("\n{}", tool.name.to_uppercase());
                        println!("   Platform: {}", tool.platform);
                        println!(
                            "   Downloaded: {}",
                            if tool.downloaded { "Yes" } else { "No" }
                        );
                        if tool.downloaded {
                            println!("   Downloaded Path: {}", tool.downloaded_path);
                        }
                        println!(
                            "   System Available: {}",
                            if tool.system_available { "Yes" } else { "No" }
                        );
                        if let Some(system_path) = &tool.system_path {
                            println!("   System Path: {}", system_path);
                        }
                        println!("   Configured Path: {}", tool.current_path);
                    }
                }
            }
            Some(ToolsCommands::Download { tool }) => match tools_manager::download_tool(&tool) {
                Ok(path) => {
                    println!("Successfully downloaded {} to: {}", tool, path);
                }
                Err(e) => {
                    eprintln!("Error downloading {}: {}", tool, e);
                    std::process::exit(1);
                }
            },
            None => {}
        },
        Some(Commands::Models { models_command }) => match models_command {
            Some(ModelsCommands::List { json }) => {
                let models = model_manager::list_models();
                if json {
                    let json_output = serde_json::to_string_pretty(&models)?;
                    println!("{}", json_output);
                } else {
                    let (downloaded, available): (Vec<_>, Vec<_>) =
                        models.iter().partition(|model| model.downloaded);

                    if !downloaded.is_empty() {
                        println!("INSTALLED MODELS");
                        println!("{}", "=".repeat(50));
                        for model in downloaded {
                            let status = if model.configured { "* " } else { " " };
                            println!("{}{}", status, model.name);
                            if model.configured {
                                println!("   Status: Currently configured");
                            }
                            println!("   Path: {}", model.path);
                            println!();
                        }
                    }

                    if !available.is_empty() {
                        println!("AVAILABLE MODELS");
                        println!("{}", "=".repeat(50));
                        for model in available {
                            println!("  {}", model.name);
                        }
                    }
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
        },
        Some(Commands::Watch) => {
            let mut cfg: AtciConfig = config::load_config()?;

            // Define required fields for watch command
            let mut required_fields = HashSet::new();
            required_fields.insert("ffmpeg_path".to_string());
            required_fields.insert("ffprobe_path".to_string());
            required_fields.insert("whispercli_path".to_string());
            required_fields.insert("model_name".to_string());
            required_fields.insert("watch_directories".to_string());

            // Validate and prompt for missing configuration
            validate_and_prompt_config(&mut cfg, &required_fields)?;

            // Setup PID file management
            setup_pid_file_management()?;

            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async {
                if let Err(e) = queue::watch_for_missing_metadata().await {
                    eprintln!("Error starting metadata watcher: {}", e);
                    std::process::exit(1);
                }

                if let Err(e) = queue::process_queue().await {
                    eprintln!("Error starting queue processor: {}", e);
                    std::process::exit(1);
                }

                // Keep the main thread alive while the background tasks run
                loop {
                    tokio::time::sleep(Duration::from_secs(60)).await;
                }
            });
        }
        Some(Commands::Config { config_command }) => match config_command {
            Some(ConfigCommands::Show) => {
                let cfg: AtciConfig = config::load_config()?;
                let json_output = serde_json::to_string_pretty(&cfg)?;
                println!("{}", json_output);
            }
            Some(ConfigCommands::Path) => {
                let config_path = confy::get_configuration_file_path("atci", "config")?;
                println!("{}", config_path.display());
            }
            Some(ConfigCommands::Set { field, value }) => {
                if !is_valid_config_field(&field) {
                    eprintln!(
                        "Error: Unknown field '{}'. Valid fields are: ffmpeg_path, ffprobe_path, model_name, whispercli_path, watch_directories, password, allow_whisper, allow_subtitles, processing_success_command, processing_failure_command, stream_chunk_size",
                        field
                    );
                    std::process::exit(1);
                }

                let mut cfg: AtciConfig = config::load_config()?;

                if let Err(e) = set_config_field(&mut cfg, &field, &value) {
                    eprintln!("Error setting field: {}", e);
                    std::process::exit(1);
                }

                config::store_config(&cfg)?;
                println!("Set {} = {}", field, value);
            }
            Some(ConfigCommands::Unset { field }) => {
                if !is_valid_config_field(&field) {
                    eprintln!(
                        "Error: Unknown field '{}'. Valid fields are: ffmpeg_path, ffprobe_path, model_name, whispercli_path, watch_directories, password, allow_whisper, allow_subtitles, processing_success_command, processing_failure_command, stream_chunk_size",
                        field
                    );
                    std::process::exit(1);
                }

                let mut cfg: AtciConfig = config::load_config()?;

                if let Err(e) = unset_config_field(&mut cfg, &field) {
                    eprintln!("Error unsetting field: {}", e);
                    std::process::exit(1);
                }

                config::store_config(&cfg)?;
                println!("Unset {}", field);
            }
            None => {
                let cfg: AtciConfig = config::load_config()?;
                let json_output = serde_json::to_string_pretty(&cfg)?;
                println!("{}", json_output);
            }
        },
        Some(Commands::Search {
            query,
            json,
            filter,
            clip,
            gif,
        }) => {
            let search_query = query.join(" ");

            match search::search(&search_query, filter.as_ref(), clip, gif) {
                Ok(results) => {
                    if json {
                        let json_output = serde_json::to_string_pretty(&results)?;
                        println!("{}", json_output);
                    } else {
                        for result in results {
                            println!("File: {}", result.file_path);
                            for search_match in result.matches {
                                if let Some(timestamp) = search_match.timestamp {
                                    println!("  {}: {}", search_match.line_number, timestamp);
                                    println!(
                                        "  {}:\t{}",
                                        search_match.line_number + 1,
                                        search_match.line_text
                                    );
                                } else {
                                    println!(
                                        "  {}: \"{}\"",
                                        search_match.line_number, search_match.line_text
                                    );
                                }

                                // Display clip information if available
                                if let Some(clip_path) = &search_match.clip_path {
                                    println!("Clip: {}", clip_path);
                                }
                                if let Some(clip_command) = &search_match.clip_command {
                                    println!("Command: {}", clip_command);
                                }

                                println!();
                            }
                            println!();
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error searching: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Transcripts {
            transcripts_command,
        }) => match transcripts_command {
            Some(TranscriptsCommands::Get { path }) => match transcripts::get_transcript(&path) {
                Ok(content) => {
                    println!("{}", content);
                }
                Err(e) => {
                    eprintln!("Error reading transcript: {}", e);
                    std::process::exit(1);
                }
            },
            Some(TranscriptsCommands::SetLine {
                video_path,
                line_number,
                content,
            }) => match transcripts::set_line(&video_path, line_number, &content) {
                Ok(()) => {
                    println!(
                        "Successfully updated line {} in transcript for {}",
                        line_number, video_path
                    );
                }
                Err(e) => {
                    eprintln!("Error setting line: {}", e);
                    std::process::exit(1);
                }
            },
            Some(TranscriptsCommands::Set {
                video_path,
                content,
            }) => match transcripts::set(&video_path, &content) {
                Ok(()) => {
                    println!(
                        "Successfully replaced transcript content for {}",
                        video_path
                    );
                }
                Err(e) => {
                    eprintln!("Error setting file content: {}", e);
                    std::process::exit(1);
                }
            },
            Some(TranscriptsCommands::Regenerate {
                video_path,
                interactive,
            }) => {
                if interactive {
                    let rt = tokio::runtime::Runtime::new()?;
                    rt.block_on(async {
                        match transcripts::regenerate_interactive(&video_path).await {
                            Ok(()) => {
                                println!("Successfully processed {}", video_path);
                            }
                            Err(e) => {
                                eprintln!("Error processing file: {}", e);
                                std::process::exit(1);
                            }
                        }
                    });
                } else {
                    match transcripts::regenerate(&video_path, None, None) {
                        Ok(()) => {
                            println!("Successfully deleted transcript files for {}", video_path);
                        }
                        Err(e) => {
                            eprintln!("Error deleting files: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
            }
            Some(TranscriptsCommands::Rename {
                video_path,
                new_path,
            }) => match transcripts::rename(&video_path, &new_path) {
                Ok(()) => {
                    println!("Successfully renamed {} to {}", video_path, new_path);
                }
                Err(e) => {
                    eprintln!("Error renaming files: {}", e);
                    std::process::exit(1);
                }
            },
            None => {}
        },
        Some(Commands::Web { web_command }) => {
            match web_command {
                Some(WebCommands::All { host, port }) => {
                    let mut cfg: AtciConfig = config::load_config()?;
                    let mut required_fields = HashSet::new();
                    required_fields.insert("ffmpeg_path".to_string());
                    required_fields.insert("ffprobe_path".to_string());
                    required_fields.insert("whispercli_path".to_string());
                    required_fields.insert("model_name".to_string());
                    required_fields.insert("watch_directories".to_string());

                    // Validate and prompt for missing configuration
                    validate_and_prompt_config(&mut cfg, &required_fields)?;

                    // Setup PID file management
                    setup_pid_file_management()?;

                    files::get_and_save_video_info_from_disk()?;

                    println!(
                        "Starting atci \x1b[1mAPI\x1b[0m and \x1b[1mWeb UI\x1b[0m server on \x1b]8;;http://{}:{}\x1b\\http://{}:{}\x1b]8;;\x1b\\",
                        host, port, host, port
                    );

                    let rt = tokio::runtime::Runtime::new()?;
                    rt.block_on(async {
                        if let Err(e) = queue::watch_for_missing_metadata().await {
                            eprintln!("Error starting metadata watcher: {}", e);
                            std::process::exit(1);
                        }

                        if let Err(e) = queue::process_queue().await {
                            eprintln!("Error starting queue processor: {}", e);
                            std::process::exit(1);
                        }

                        if let Err(e) = web::launch_server(&host, port).await {
                            eprintln!("Error starting web server: {}", e);
                            std::process::exit(1);
                        }
                    });
                }
                Some(WebCommands::Api { host, port }) => {
                    // Setup PID file management
                    setup_pid_file_management()?;

                    println!(
                        "Starting atci \x1b[1mAPI-only\x1b[0m server on \x1b]8;;http://{}:{}\x1b\\http://{}:{}\x1b]8;;\x1b\\",
                        host, port, host, port
                    );

                    let rt = tokio::runtime::Runtime::new()?;
                    rt.block_on(async {
                        if let Err(e) = web::launch_api_server(&host, port).await {
                            eprintln!("Error starting API server: {}", e);
                            std::process::exit(1);
                        }
                    });
                }
                None => {}
            }
        }
        Some(Commands::Update) => {
            if let Err(e) = update() {
                eprintln!("Error updating: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Version { json }) => {
            if let Err(e) = check_version(json) {
                eprintln!("Error checking version: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Streamdl { stream_name, url }) => {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                if let Err(e) = download_stream(&url, &stream_name).await {
                    eprintln!("Error downloading stream: {}", e);
                    std::process::exit(1);
                }
            });
        }
        Some(Commands::Tui) => {
            if let Err(e) = tui::run() {
                eprintln!("Error running TUI: {}", e);
                std::process::exit(1);
            }
        }
        None => {}
    }

    // Clean up PID file on normal exit
    cleanup_pid_file();

    Ok(())
}
