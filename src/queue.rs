// atci (andrew's transcript and clipping interface)
// Copyright (C) 2025 Andrew Nissen

use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::Duration;
use walkdir::WalkDir;
use crate::config::AtciConfig;
use crate::video_processor;
use tokio::time::sleep;

use rocket::serde::json::Json;
use rocket::serde::Deserialize;
use rocket::{get, post};
use crate::web::ApiResponse;
use crate::config;
use crate::files;
use crate::auth::AuthGuard;

#[get("/api/queue")]
pub fn web_get_queue(_auth: AuthGuard) -> Json<ApiResponse<serde_json::Value>> {
    match get_queue() {
        Ok(queue_data) => Json(ApiResponse::success(queue_data.into())),
        Err(e) => Json(ApiResponse::error(format!("Failed to get queue: {}", e))),
    }
}

pub fn get_queue() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let queue_path = std::path::Path::new(&home_dir).join(".atci/.queue");
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

pub fn set_queue(paths: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let queue_path = std::path::Path::new(&home_dir).join(".atci/.commands/").join("SET_QUEUE");
    
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(queue_path)?;
    
    for path in paths {
        writeln!(file, "{}", path)?;
    }
    Ok(())
}

pub fn add_to_queue(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let queue_path = std::path::Path::new(&home_dir).join(".atci/.queue");

    let currently_processing_path = std::path::Path::new(&home_dir).join(".atci/.currently_processing");
    if currently_processing_path.exists() && fs::read_to_string(&currently_processing_path)? == path {
        return Ok(());
    }

    let existing_queue = get_queue()?;
    // we need to lock the file before we perform out duplication check so that someone doesn't write 
    // to it after we check but before we lock
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(queue_path)?;
    

    if existing_queue.contains(&path.to_string()) {
        return Ok(());
    }
    
    writeln!(file, "{}", path)?;

    Ok(())
}

pub fn add_to_blocklist(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let blocklist_path = std::path::Path::new(&home_dir).join(".atci/.blocklist");
    
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(blocklist_path)?;
    
    writeln!(file, "{}", path)?;
    Ok(())
}

pub fn load_blocklist() -> Vec<String> {
    let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let blocklist_path = std::path::Path::new(&home_dir).join(".atci/.blocklist");
    if blocklist_path.exists() {
        if let Ok(content) = fs::read_to_string(&blocklist_path) {
            content.lines()
                .map(|line| line.trim().to_string())
                .filter(|line| !line.is_empty())
                .collect()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    }
}

#[get("/api/queue/status")]
pub fn web_get_queue_status(_auth: AuthGuard) -> Json<ApiResponse<serde_json::Value>> {
    match get_queue_status() {
        Ok((path, age)) => {
            let queue = get_queue().unwrap_or_else(|_| Vec::new());
            let result = serde_json::json!({
                "currently_processing": path.unwrap_or_else(|| "".to_string()),
                "age_in_seconds": age,
                "queue": queue
            });
            Json(ApiResponse::success(result))
        }
        Err(e) => Json(ApiResponse::error(format!("Failed to get queue status: {}", e))),
    }
}

#[post("/api/queue/block", data = "<path>")]
pub fn web_block_path(_auth: AuthGuard, path: String) -> Json<ApiResponse<&'static str>> {
    match add_to_blocklist(&path) {
        Ok(()) => Json(ApiResponse::success("Path added to blocklist")),
        Err(e) => Json(ApiResponse::error(format!("Failed to add path to blocklist: {}", e))),
    }
}

#[derive(Deserialize)]
pub struct SetRequest {
    paths: Vec<String>,
}

#[post("/api/queue/set", data = "<request>")]
pub fn web_set_queue(_auth: AuthGuard, request: Json<SetRequest>) -> Json<ApiResponse<&'static str>> {
    match set_queue(request.paths.clone()) {
        Ok(()) => Json(ApiResponse::success("Queue set successfully")),
        Err(e) => Json(ApiResponse::error(format!("Failed to set queue: {}", e))),
    }
}

#[post("/api/queue/cancel")]
pub fn web_cancel_queue(_auth: AuthGuard) -> Json<ApiResponse<String>> {
    match cancel_queue() {
        Ok(message) => Json(ApiResponse::success(message)),
        Err(e) => Json(ApiResponse::error(format!("Failed to cancel queue: {}", e))),
    }
}

pub fn get_queue_status() -> Result<(Option<String>, u64), Box<dyn std::error::Error>> {
    let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let currently_processing_path = std::path::Path::new(&home_dir).join(".atci/.currently_processing");
    
    if !currently_processing_path.exists() {
        return Ok((None, 0));
    }
    
    let content = fs::read_to_string(&currently_processing_path)?;
    let path = content.trim();
    
    if path.is_empty() {
        return Ok((None, 0));
    }
    
    // Get the modification time of the .currently_processing file
    let metadata = fs::metadata(&currently_processing_path)?;
    let modified = metadata.modified()?;
    let now = std::time::SystemTime::now();
    let age = now.duration_since(modified)?.as_secs();
    
    Ok((Some(path.to_string()), age))
}

fn remove_first_line_from_queue() -> Result<(), Box<dyn std::error::Error>> {
    let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let queue_path = std::path::Path::new(&home_dir).join(".atci/.queue");
    
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

pub async fn process_queue() -> Result<(), Box<dyn std::error::Error>> {
    tokio::spawn(async {
        loop {
           let _ = process_queue_iteration().await;

           let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
           let currently_processing_path = std::path::Path::new(&home_dir).join(".atci/.currently_processing");
           if currently_processing_path.exists() {
               fs::remove_file(&currently_processing_path).unwrap();
           }

           sleep(Duration::from_secs(2)).await;
        }
    });
    Ok(())
}

pub async fn process_queue_iteration() -> Result<bool, Box<dyn std::error::Error>> {
    let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let currently_processing_path = std::path::Path::new(&home_dir).join(".atci/.currently_processing");
    if !currently_processing_path.exists() {
        return Ok(false);
    }
        
    let content = fs::read_to_string(&currently_processing_path)?;
    let first_line = content.lines().next();
    if let Some(video_path_str) = first_line {
        println!("Processing queue item: {}", video_path_str);
        let video_path_str = video_path_str.trim();
        if video_path_str.is_empty() {
            return Ok(false);
        }
        
        let video_path = Path::new(video_path_str);
        
        if !video_path.exists() {
            eprintln!("Video file does not exist: {}", video_path_str);
            return Ok(true);
        }
        
        let video_extensions = crate::files::get_video_extensions();
        let has_valid_extension = video_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| video_extensions.contains(&ext.to_lowercase().as_str()))
            .unwrap_or(false);
        
        if !has_valid_extension {
            eprintln!("File does not have a valid video extension: {}", video_path_str);
            return Ok(true);
        }
        
        let txt_path = video_path.with_extension("txt");
        
        // Create transcript with cancellation support
        if !txt_path.exists() {
            match video_processor::cancellable_create_transcript(video_path).await {
                Ok(true) => {
                    // Successfully created transcript, continue
                }
                Ok(false) => {
                    // Cancelled, exit early
                    println!("Processing cancelled for: {}", video_path_str);
                    return Ok(true);
                }
                Err(e) => {
                    eprintln!("Error creating transcript for {}: {}", video_path_str, e);
                    return Ok(true);
                }
            }
        }
        
        // Update metadata with length and cancellation support
        match video_processor::cancellable_add_length_to_metadata(video_path).await {
            Ok(true) => {
                // Successfully added metadata, continue
            }
            Ok(false) => {
                // Cancelled, exit early
                println!("Processing cancelled for: {}", video_path_str);
                return Ok(true);
            }
            Err(e) => {
                eprintln!("Error adding length metadata for {}: {}", video_path_str, e);
                return Ok(true);
            }
        }

        println!("Processed queue item: {}", video_path_str);
        let new_cache = files::get_video_info_from_disk()?;
        files::save_video_info_to_cache(&new_cache)?;
        return Ok(true);
    }
    
    Ok(false)
}

pub fn cancel_queue() -> Result<String, Box<dyn std::error::Error>> {
    let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let commands_dir = std::path::Path::new(&home_dir).join(".atci").join(".commands");
    let cancel_file = commands_dir.join("CANCEL");
    
    if cancel_file.exists() {
        let metadata = fs::metadata(&cancel_file)?;
        let created = metadata.created()
            .or_else(|_| metadata.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        
        let duration = created.duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0));
        let datetime = chrono::DateTime::from_timestamp(duration.as_secs() as i64, 0)
            .unwrap_or_default();
        
        Ok(format!("CANCEL file already exists, created at: {}", datetime.format("%Y-%m-%d %H:%M:%S UTC")))
    } else {
        fs::create_dir_all(&commands_dir)?;
        fs::write(&cancel_file, "")?;
        Ok("Created CANCEL file".to_string())
    }
}

pub async fn watch_for_missing_metadata() -> Result<(), Box<dyn std::error::Error>> {
    tokio::spawn(async move {
        loop {
            let cfg: AtciConfig = config::load_config().expect("Failed to load config");

            let video_extensions = crate::files::get_video_extensions();
            
            if cfg.watch_directories.is_empty() {
                eprintln!("No watch directories configured");
                return;
            }
    
            let blocklist = load_blocklist();
            
            // Check for SET_QUEUE file and copy it to queue, overwriting it
            let set_queue_path = std::path::Path::new(&std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
                .join(".atci/.commands/SET_QUEUE");
            
            if set_queue_path.exists() {
                if let Ok(content) = fs::read_to_string(&set_queue_path) {
                    let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                    let queue_path = std::path::Path::new(&home_dir).join(".atci/.queue");
                    
                    // Get existing queue items
                    let existing_queue = get_queue().unwrap_or_else(|_| Vec::new());
                    
                    // Filter SET_QUEUE lines to only include items that exist in current queue
                    let filtered_lines: Vec<String> = content.lines()
                        .map(|line| line.trim().to_string())
                        .filter(|line| !line.is_empty() && existing_queue.contains(line))
                        .collect();
                    
                    // Write filtered content back to queue
                    if !filtered_lines.is_empty() {
                        let filtered_content = filtered_lines.join("\n") + "\n";
                        let _ = fs::write(&queue_path, filtered_content);
                    }
                }
                // Delete the SET_QUEUE file after loading
                let _ = fs::remove_file(&set_queue_path);
            }
            
            // Scan watch directories and add files to queue
            let files_to_add: Vec<_> = cfg.watch_directories.iter().map(|wd| {
                let mut files: Vec<_> = WalkDir::new(wd).into_iter().filter_map(|e| e.ok()).filter_map(|entry| {
                    let file_path = entry.path();

                    // skip directories
                    if !file_path.is_file() {
                        return None;
                    }

                    // skip files that are in the blocklist
                    if blocklist.contains(&file_path.to_string_lossy().to_string()) {
                        return None;
                    }

                    if let Some(extension) = file_path.extension() {
                        let ext_str = extension.to_string_lossy().to_lowercase();

                        // we're only interested in video files
                        if !video_extensions.contains(&ext_str.as_str()) {
                            return None;
                        }

                        // we want to make sure the file isn't in the process of currently being copied over to our watch directory
                        // since there isn't any way to actually tell for sure via an api call, a useful proxy for this is that the file hasn't been modified in the last 3 seconds
                        if let Ok(metadata) = fs::metadata(&file_path) {
                            if let Ok(modified) = metadata.modified() {
                                let now = std::time::SystemTime::now();
                                if let Ok(duration) = now.duration_since(modified) {
                                    if duration.as_secs() >= 3 {
                                        let txt_path = file_path.with_extension("txt");
                                        
                                        if !txt_path.exists() {
                                            return Some(file_path.to_string_lossy().to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                    None
                }).collect::<Vec<_>>();
                files.sort_by(|a, b| a.cmp(b));
                files
            }).flatten().collect();

            for file_to_add in files_to_add {
                if let Err(e) = add_to_queue(&file_to_add) {
                    eprintln!("Error adding to queue: {}", e);
                }
            }

            // after we've added all the files to the queue, take the first line and remove it from the queue, writing it to the currently processing file
            let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            let currently_processing_path = std::path::Path::new(&home_dir).join(".atci/.currently_processing");
            if !currently_processing_path.exists() {
                let queue = get_queue().unwrap_or_else(|_| Vec::new());
                if !queue.is_empty() {
                    fs::write(&currently_processing_path, &queue[0]).unwrap();
                    remove_first_line_from_queue().unwrap();
                }
            }
            sleep(Duration::from_millis(500)).await;
        }
    });
    Ok(())
}