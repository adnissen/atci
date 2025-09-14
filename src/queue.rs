// atci (andrew's transcript and clipping interface)
// Copyright (C) 2025 Andrew Nissen

use crate::config::AtciConfig;
use crate::video_processor;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::Duration;
use tokio::time::sleep;
use walkdir::WalkDir;

use crate::auth::AuthGuard;
use crate::config;
use crate::db;
use crate::files;
use crate::web::ApiResponse;
use rocket::serde::Deserialize;
use rocket::serde::json::Json;
use rocket::{get, post};
use rusqlite::Connection;

#[get("/api/queue")]
pub fn web_get_queue(_auth: AuthGuard) -> Json<ApiResponse<serde_json::Value>> {
    match get_queue(None) {
        Ok(queue_data) => Json(ApiResponse::success(queue_data.into())),
        Err(e) => Json(ApiResponse::error(format!("Failed to get queue: {}", e))),
    }
}

pub fn get_queue(conn: Option<&Connection>) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let owned_conn;
    let conn = match conn {
        Some(c) => c,
        None => {
            owned_conn = db::get_connection()?;
            &owned_conn
        }
    };

    let mut stmt = conn.prepare("SELECT path FROM queue ORDER BY position")?;
    let queue_iter = stmt.query_map([], |row| row.get::<_, String>(0))?;

    let mut queue = Vec::new();
    for path in queue_iter {
        queue.push(path?);
    }

    Ok(queue)
}

pub fn set_queue(paths: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let conn = db::get_connection()?;
    let tx = conn.unchecked_transaction()?;

    // Get all existing paths from the queue table
    let mut existing_paths = std::collections::HashMap::new();
    {
        let mut stmt = tx.prepare("SELECT path, model, subtitle_stream_index FROM queue")?;
        let existing_iter = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<i64>>(2)?,
            ))
        })?;

        for item in existing_iter {
            let (path, model, subtitle_stream_index) = item?;
            existing_paths.insert(path, (model, subtitle_stream_index));
        }
    } // stmt is dropped here

    // Clear the queue table
    tx.execute("DELETE FROM queue", [])?;

    let mut position = 0i64;

    // Add paths in the specified order
    for path in &paths {
        let (model, subtitle_stream_index) = existing_paths.remove(path).unwrap_or((None, None));

        tx.execute(
            "INSERT INTO queue (position, path, model, subtitle_stream_index) VALUES (?1, ?2, ?3, ?4)",
            (position, path, model, subtitle_stream_index),
        )?;
        position += 1;
    }

    // Add any remaining paths that weren't in the input at the end
    for (path, (model, subtitle_stream_index)) in existing_paths {
        tx.execute(
            "INSERT INTO queue (position, path, model, subtitle_stream_index) VALUES (?1, ?2, ?3, ?4)",
            (position, path, model, subtitle_stream_index),
        )?;
        position += 1;
    }

    tx.commit()?;
    Ok(())
}

pub fn add_to_queue(
    path: &str,
    model: Option<String>,
    subtitle_stream_index: Option<i32>,
) -> Result<(), Box<dyn std::error::Error>> {
    let conn = db::get_connection()?;

    // Check if this path is currently being processed
    let is_currently_processing: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM currently_processing WHERE path = ?1",
            [path],
            |row| row.get(0),
        )
        .unwrap_or(false);

    if is_currently_processing {
        return Ok(());
    }

    let existing_queue = get_queue(Some(&conn))?;

    if existing_queue.contains(&path.to_string()) {
        return Ok(());
    }

    // Get the next position (max position + 1, or 0 if empty)
    let next_position: i64 = conn.query_row(
        "SELECT COALESCE(MAX(position), -1) + 1 FROM queue",
        [],
        |row| row.get(0),
    )?;

    conn.execute(
        "INSERT INTO queue (position, path, model, subtitle_stream_index) VALUES (?1, ?2, ?3, ?4)",
        (
            next_position,
            path,
            model,
            subtitle_stream_index.map(|i| i as i64),
        ),
    )?;

    Ok(())
}

pub fn add_to_blocklist(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let home_dir = dirs::home_dir().ok_or("Could not find home directory")?;
    let blocklist_path = home_dir.join(".atci/.blocklist");

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(blocklist_path)?;

    writeln!(file, "{}", path)?;
    Ok(())
}

#[get("/api/queue/status")]
pub fn web_get_queue_status(_auth: AuthGuard) -> Json<ApiResponse<serde_json::Value>> {
    let conn = match db::get_connection() {
        Ok(conn) => conn,
        Err(e) => {
            return Json(ApiResponse::error(format!(
                "Database connection failed: {}",
                e
            )));
        }
    };
    match get_queue_status(Some(&conn)) {
        Ok((path, age)) => {
            let queue = get_queue(Some(&conn)).unwrap_or_else(|_| Vec::new());
            let result = serde_json::json!({
                "currently_processing": path.unwrap_or_else(|| "".to_string()),
                "age_in_seconds": age,
                "queue": queue
            });
            Json(ApiResponse::success(result))
        }
        Err(e) => Json(ApiResponse::error(format!(
            "Failed to get queue status: {}",
            e
        ))),
    }
}

#[post("/api/queue/block", data = "<path>")]
pub fn web_block_path(_auth: AuthGuard, path: String) -> Json<ApiResponse<&'static str>> {
    match add_to_blocklist(&path) {
        Ok(()) => Json(ApiResponse::success("Path added to blocklist")),
        Err(e) => Json(ApiResponse::error(format!(
            "Failed to add path to blocklist: {}",
            e
        ))),
    }
}

#[derive(Deserialize)]
pub struct SetRequest {
    paths: Vec<String>,
}

#[post("/api/queue/set", data = "<request>")]
pub fn web_set_queue(
    _auth: AuthGuard,
    request: Json<SetRequest>,
) -> Json<ApiResponse<&'static str>> {
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

pub fn get_queue_status(
    conn: Option<&Connection>,
) -> Result<(Option<String>, u64), Box<dyn std::error::Error>> {
    let owned_conn;
    let conn = match conn {
        Some(c) => c,
        None => {
            owned_conn = db::get_connection()?;
            &owned_conn
        }
    };

    // Get the currently processing item with its starting time
    let result: Option<(String, Option<String>)> = conn
        .query_row(
            "SELECT path, starting_time FROM currently_processing LIMIT 1",
            [],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
        )
        .ok();

    match result {
        Some((path, starting_time_str)) => {
            let age = if let Some(starting_time_str) = starting_time_str {
                // Parse the RFC3339 timestamp and calculate age
                if let Ok(starting_time) = chrono::DateTime::parse_from_rfc3339(&starting_time_str)
                {
                    let now = chrono::Utc::now();
                    let duration =
                        now.signed_duration_since(starting_time.with_timezone(&chrono::Utc));
                    duration.num_seconds().max(0) as u64
                } else {
                    0
                }
            } else {
                0
            };

            Ok((Some(path), age))
        }
        None => Ok((None, 0)),
    }
}

fn remove_first_line_from_queue(
    conn: Option<&Connection>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Remove the first item from the database queue table and reposition remaining items
    let owned_conn;
    let conn = match conn {
        Some(c) => c,
        None => {
            owned_conn = db::get_connection()?;
            &owned_conn
        }
    };
    let tx = conn.unchecked_transaction()?;

    tx.execute(
        "DELETE FROM queue WHERE position = (SELECT MIN(position) FROM queue)",
        [],
    )?;

    tx.execute("UPDATE queue SET position = position - 1", [])?;

    tx.commit()?;

    Ok(())
}

pub async fn process_queue() -> Result<(), Box<dyn std::error::Error>> {
    tokio::spawn(async {
        loop {
            let _ = process_queue_iteration().await;

            // Clear the currently_processing table after each iteration
            if let Ok(conn) = db::get_connection() {
                let _ = conn.execute("DELETE FROM currently_processing", []);
            }

            sleep(Duration::from_secs(2)).await;
        }
    });
    Ok(())
}

pub async fn process_queue_iteration() -> Result<bool, Box<dyn std::error::Error>> {
    let conn = db::get_connection()?;

    // Get the currently processing item with model and subtitle stream info
    let current_item: Option<(String, Option<String>, Option<i64>)> = conn
        .query_row(
            "SELECT path, model, subtitle_stream_index FROM currently_processing LIMIT 1",
            [],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                ))
            },
        )
        .ok();

    if let Some((video_path_str, model, subtitle_stream_index)) = current_item {
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
            eprintln!(
                "File does not have a valid video extension: {}",
                video_path_str
            );
            return Ok(true);
        }

        // Create transcript with cancellation support
        let subtitle_index_i32 = subtitle_stream_index.map(|i| i as i32);
        match video_processor::cancellable_create_transcript(video_path, model, subtitle_index_i32)
            .await
        {
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
        files::get_and_save_video_info_from_disk()?;
        return Ok(true);
    }

    Ok(false)
}

pub fn cancel_queue() -> Result<String, Box<dyn std::error::Error>> {
    let home_dir = dirs::home_dir().ok_or("Could not find home directory")?;
    let commands_dir = home_dir.join(".atci").join(".commands");
    let cancel_file = commands_dir.join("CANCEL");

    if cancel_file.exists() {
        let metadata = fs::metadata(&cancel_file)?;
        let created = metadata
            .created()
            .or_else(|_| metadata.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

        let duration = created
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0));
        let datetime =
            chrono::DateTime::from_timestamp(duration.as_secs() as i64, 0).unwrap_or_default();

        Ok(format!(
            "CANCEL file already exists, created at: {}",
            datetime.format("%Y-%m-%d %H:%M:%S UTC")
        ))
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

            // Scan watch directories and add files to queue
            let files_to_add: Vec<_> = cfg
                .watch_directories
                .iter()
                .flat_map(|wd| {
                    let mut files: Vec<_> = WalkDir::new(wd)
                        .into_iter()
                        .filter_map(|e| e.ok())
                        .filter_map(|entry| {
                            let file_path = entry.path();

                            // skip directories
                            if !file_path.is_file() {
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
                                if let Ok(metadata) = fs::metadata(file_path)
                                    && let Ok(modified) = metadata.modified()
                                {
                                    let now = std::time::SystemTime::now();
                                    if let Ok(duration) = now.duration_since(modified)
                                        && duration.as_secs() >= 3
                                    {
                                        let txt_path = file_path.with_extension("txt");

                                        if !txt_path.exists() {
                                            return Some(file_path.to_string_lossy().to_string());
                                        }
                                    }
                                }
                            }
                            None
                        })
                        .collect::<Vec<_>>();
                    files.sort();
                    files
                })
                .collect();

            for file_to_add in files_to_add {
                if let Err(e) = add_to_queue(&file_to_add, None, None) {
                    eprintln!("Error adding to queue: {}", e);
                }
            }

            // after we've added all the files to the queue, check if there's anything currently processing
            // if not, take the first item from the queue and add it to currently_processing table
            if let Ok(conn) = db::get_connection() {
                let has_current_processing: bool = conn
                    .query_row("SELECT COUNT(*) > 0 FROM currently_processing", [], |row| {
                        row.get(0)
                    })
                    .unwrap_or(false);

                if !has_current_processing {
                    // Get the first item from queue with all its data
                    if let Ok(mut stmt) = conn.prepare("SELECT path, model, subtitle_stream_index FROM queue ORDER BY position LIMIT 1")
                        && let Ok(mut rows) = stmt.query_map([], |row| {
                            Ok((
                                row.get::<_, String>(0)?,
                                row.get::<_, Option<String>>(1)?,
                                row.get::<_, Option<i64>>(2)?
                            ))
                        })
                            && let Some(Ok((path, model, subtitle_stream_index))) = rows.next() {
                                // Add to currently_processing table with current timestamp
                                let now = chrono::Utc::now().to_rfc3339();
                                let _ = conn.execute(
                                    "INSERT INTO currently_processing (starting_time, path, model, subtitle_stream_index) VALUES (?1, ?2, ?3, ?4)",
                                    (now, path, model, subtitle_stream_index),
                                );

                                // Remove from queue
                                let _ = remove_first_line_from_queue(Some(&conn));
                            }
                }
            }
            sleep(Duration::from_millis(500)).await;
        }
    });
    Ok(())
}
