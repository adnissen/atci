use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::Duration;
use walkdir::WalkDir;
use crate::config::AtciConfig;
use crate::video_processor;
use tokio::time::sleep;

use rocket::serde::json::Json;
use rocket::get;
use crate::web::ApiResponse;

#[get("/api/queue")]
pub fn web_get_queue() -> Json<ApiResponse<serde_json::Value>> {
    match get_queue() {
        Ok(queue_data) => Json(ApiResponse::success(queue_data.into())),
        Err(e) => Json(ApiResponse::error(format!("Failed to get queue: {}", e))),
    }
}

pub fn get_queue() -> Result<Vec<String>, Box<dyn std::error::Error>> {
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

pub fn add_to_queue(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let existing_queue = get_queue()?;
    let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let currently_processing_path = std::path::Path::new(&home_dir).join(".currently_processing");
    if currently_processing_path.exists() && fs::read_to_string(&currently_processing_path)? == path {
        return Ok(());
    }
    
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

#[get("/api/queue/status")]
pub fn web_get_queue_status() -> Json<ApiResponse<serde_json::Value>> {
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

pub fn get_queue_status() -> Result<(Option<String>, u64), Box<dyn std::error::Error>> {
    let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let currently_processing_path = std::path::Path::new(&home_dir).join(".currently_processing");
    
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
    println!("Removed first line from queue");
    Ok(())
}

pub async fn process_queue() -> Result<(), Box<dyn std::error::Error>> {
    tokio::spawn(async {
        loop {
           let _ = process_queue_iteration();
           sleep(Duration::from_secs(2)).await;
        }
    });
    Ok(())
}

pub fn process_queue_iteration() -> Result<bool, Box<dyn std::error::Error>> {
    let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let currently_processing_path = std::path::Path::new(&home_dir).join(".currently_processing");
    if currently_processing_path.exists() {
        let _ = fs::remove_file(&currently_processing_path);
    }
    
    let queue_path = std::path::Path::new(&home_dir).join(".queue");
    if !queue_path.exists() {
        return Ok(false);
    }
    
    let content = fs::read_to_string(&queue_path)?;
    let first_line = content.lines().next();
    if let Some(video_path_str) = first_line {
        println!("Processing queue item: {}", video_path_str);
        let video_path_str = video_path_str.trim();
        if video_path_str.is_empty() {
            remove_first_line_from_queue()?;
            return Ok(false);
        }
        
        remove_first_line_from_queue()?;
        
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
        
        fs::write(&currently_processing_path, video_path_str)?;
        
        if !txt_path.exists() {
            video_processor::create_transcript(video_path)?;
            if currently_processing_path.exists() {
                let _ = fs::remove_file(&currently_processing_path);
            }
        }
        
        // we always update the meta file with the latest length
        video_processor::create_metafile(video_path)?;

        if currently_processing_path.exists() {
            let _ = fs::remove_file(&currently_processing_path);
        }
        
        println!("Processed queue item: {}", video_path_str);
        return Ok(true);
    }
    
    Ok(false)
}

pub async fn watch_for_missing_metadata(cfg: &AtciConfig) -> Result<(), Box<dyn std::error::Error>> {
    let cfg_clone = cfg.clone();
    tokio::spawn(async move {
        let video_extensions = crate::files::get_video_extensions();
        
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
                                        
                                        if !txt_path.exists() || !meta_path.exists() {
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
            
            sleep(Duration::from_secs(2)).await;
        }
    });
    Ok(())
}