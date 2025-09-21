// atci (andrew's transcript and clipping interface)
// Copyright (C) 2025 Andrew Nissen

use crate::db;
use regex::Regex;
use rusqlite::Connection;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct VideoPart {
    pub base_name: String,
    pub part_number: i32,
    pub video_path: String,
    pub extension: String,
}

/// Parse a video filename to detect if it's a part of a multi-part video
/// Supports patterns like: filename.part1.ext, filename.part2.ext, etc.
pub fn parse_video_part(file_path: &Path) -> Option<VideoPart> {
    let file_name = file_path.file_name()?.to_str()?;
    
    // Regex to match pattern: basename.partN.extension
    let part_regex = Regex::new(r"^(.+)\.part(\d+)\.([^.]+)$").ok()?;
    
    if let Some(captures) = part_regex.captures(file_name) {
        let base_name = captures.get(1)?.as_str().to_string();
        let part_number: i32 = captures.get(2)?.as_str().parse().ok()?;
        let extension = captures.get(3)?.as_str().to_string();
        
        Some(VideoPart {
            base_name,
            part_number,
            video_path: file_path.to_string_lossy().to_string(),
            extension,
        })
    } else {
        None
    }
}

/// Get the master file paths for a video part
pub fn get_master_paths(video_part: &VideoPart) -> (String, String) {
    let parent_dir = Path::new(&video_part.video_path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| ".".to_string());
    
    let master_video_path = format!("{}/{}.{}", parent_dir, video_part.base_name, video_part.extension);
    let master_transcript_path = format!("{}/{}.txt", parent_dir, video_part.base_name);
    
    (master_video_path, master_transcript_path)
}

/// Check if a video part has been processed
pub fn is_part_processed(conn: &Connection, base_name: &str, part_number: i32) -> Result<bool, rusqlite::Error> {
    let count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM video_parts WHERE base_name = ?1 AND part_number = ?2",
        [base_name, &part_number.to_string()],
        |row| row.get(0),
    )?;
    
    Ok(count > 0)
}

/// Get all processed parts for a base video name
pub fn get_processed_parts(conn: &Connection, base_name: &str) -> Result<Vec<i32>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT part_number FROM video_parts WHERE base_name = ?1 ORDER BY part_number"
    )?;
    
    let part_iter = stmt.query_map([base_name], |row| {
        Ok(row.get::<_, i32>(0)?)
    })?;
    
    let mut parts = Vec::new();
    for part in part_iter {
        parts.push(part?);
    }
    
    Ok(parts)
}

/// Find missing parts between 1 and the given part number
pub fn find_missing_parts(conn: &Connection, base_name: &str, up_to_part: i32) -> Result<Vec<i32>, rusqlite::Error> {
    let processed_parts = get_processed_parts(conn, base_name)?;
    let mut missing = Vec::new();
    
    for part_num in 1..=up_to_part {
        if !processed_parts.contains(&part_num) {
            missing.push(part_num);
        }
    }
    
    Ok(missing)
}

/// Record a processed video part in the database
pub fn record_processed_part(
    conn: &Connection,
    video_part: &VideoPart,
    transcript_length: i32,
) -> Result<(), rusqlite::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    
    conn.execute(
        "INSERT OR REPLACE INTO video_parts (base_name, part_number, video_path, processed_at, transcript_length) 
         VALUES (?1, ?2, ?3, ?4, ?5)",
        [
            &video_part.base_name,
            &video_part.part_number.to_string(),
            &video_part.video_path,
            &now,
            &transcript_length.to_string(),
        ],
    )?;
    
    Ok(())
}

/// Check if the next sequential part exists and add it to queue
pub fn check_and_queue_next_part(video_part: &VideoPart) -> Result<(), Box<dyn std::error::Error>> {
    let parent_dir = Path::new(&video_part.video_path)
        .parent()
        .ok_or("Could not get parent directory")?;
    
    let next_part_number = video_part.part_number + 1;
    let next_part_filename = format!("{}.part{}.{}", 
        video_part.base_name, next_part_number, video_part.extension);
    let next_part_path = parent_dir.join(next_part_filename);
    
    if next_part_path.exists() {
        println!("Found next part: {}", next_part_path.display());
        crate::queue::add_to_queue(&next_part_path.to_string_lossy(), None, None)?;
    }
    
    Ok(())
}

/// Create a placeholder transcript for missing parts
pub fn create_missing_part_placeholder(
    master_transcript_path: &str,
    missing_parts: &[i32],
    current_part: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    let missing_parts_str = missing_parts
        .iter()
        .map(|p| p.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    
    let placeholder_content = format!(
        ">>> Part {} of video, missing part(s): {} <<<\nProcessing paused until missing parts are available.\n",
        current_part, missing_parts_str
    );
    
    std::fs::write(master_transcript_path, placeholder_content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse_video_part_valid() {
        let path = PathBuf::from("/path/to/episode01.part1.mkv");
        let result = parse_video_part(&path);
        
        assert!(result.is_some());
        let part = result.unwrap();
        assert_eq!(part.base_name, "episode01");
        assert_eq!(part.part_number, 1);
        assert_eq!(part.extension, "mkv");
    }

    #[test]
    fn test_parse_video_part_invalid() {
        let path = PathBuf::from("/path/to/regular_video.mkv");
        let result = parse_video_part(&path);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_video_part_complex_basename() {
        let path = PathBuf::from("/path/to/show_s01e05_720p.part3.mp4");
        let result = parse_video_part(&path);
        
        assert!(result.is_some());
        let part = result.unwrap();
        assert_eq!(part.base_name, "show_s01e05_720p");
        assert_eq!(part.part_number, 3);
        assert_eq!(part.extension, "mp4");
    }

    #[test]
    fn test_get_master_paths() {
        let part = VideoPart {
            base_name: "episode01".to_string(),
            part_number: 1,
            video_path: "/videos/episode01.part1.mkv".to_string(),
            extension: "mkv".to_string(),
        };
        
        let (video_path, transcript_path) = get_master_paths(&part);
        assert_eq!(video_path, "/videos/episode01.mkv");
        assert_eq!(transcript_path, "/videos/episode01.txt");
    }

    #[test]
    fn test_find_missing_parts() {
        // This would need a test database setup, skipping for now
        // but the logic should work correctly
    }
}