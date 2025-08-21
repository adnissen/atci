use std::fs;
use std::path::Path;
use walkdir::WalkDir;
use serde::Serialize;
use crate::AtciConfig;
use rayon::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
pub struct SearchMatch {
    pub line_number: usize,
    pub line_text: String,
    pub timestamp: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub file_path: String,
    pub matches: Vec<SearchMatch>,
}

pub fn search(query: &str, cfg: &AtciConfig) -> Result<Vec<SearchResult>, Box<dyn std::error::Error>> {
    let video_extensions = crate::get_video_extensions();
    
    let all_entries: Vec<_> = cfg.watch_directories
        .iter()
        .flat_map(|watch_directory| {
            WalkDir::new(watch_directory)
                .into_iter()
                .filter_map(|e| e.ok())
                .collect::<Vec<_>>()
        })
        .collect();

    let mut results: Vec<SearchResult> = all_entries
        .par_iter()
        .filter_map(|entry| {
            let file_path = entry.path();
            
            if !file_path.is_file() {
                return None;
            }
            
            let extension = file_path.extension()?;
            let ext_str = extension.to_str()?;
            
            if !video_extensions.contains(&ext_str.to_lowercase().as_str()) {
                return None;
            }
            
            let txt_path = file_path.with_extension("txt");
            
            if !txt_path.exists() {
                return None;
            }
            
            let content = fs::read_to_string(&txt_path).ok()?;
            let lines: Vec<&str> = content.lines().collect();
            let matches: Vec<SearchMatch> = lines
                .iter()
                .enumerate()
                .filter_map(|(line_num, line)| {
                    if line.to_lowercase().contains(&query.to_lowercase()) {
                        // Check if the previous line contains a timestamp
                        let timestamp = if line_num > 0 {
                            let prev_line = lines[line_num - 1];
                            // Check if the previous line looks like a timestamp (contains digits and colons)
                            if prev_line.contains(':') && prev_line.chars().any(|c| c.is_ascii_digit()) {
                                Some(prev_line.to_string())
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        
                        Some(SearchMatch {
                            line_number: line_num + 1,
                            line_text: line.to_string(),
                            timestamp,
                        })
                    } else {
                        None
                    }
                })
                .collect();
            
            if matches.is_empty() {
                None
            } else {
                Some(SearchResult {
                    file_path: file_path.to_string_lossy().to_string(),
                    matches,
                })
            }
        })
        .collect();

    results.sort_by(|a, b| a.file_path.cmp(&b.file_path));

    Ok(results)
}