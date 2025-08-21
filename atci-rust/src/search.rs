use std::fs;
use std::path::Path;
use walkdir::WalkDir;
use serde::Serialize;
use crate::AtciConfig;
use rayon::prelude::*;

#[derive(Serialize)]
pub struct SearchResult {
    pub video_path: String,
    pub transcript_path: String,
    pub line_numbers: Vec<usize>,
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

    let results: Vec<SearchResult> = all_entries
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
            let line_numbers: Vec<usize> = content
                .lines()
                .enumerate()
                .filter_map(|(line_num, line)| {
                    if line.to_lowercase().contains(&query.to_lowercase()) {
                        Some(line_num + 1)
                    } else {
                        None
                    }
                })
                .collect();
            
            if line_numbers.is_empty() {
                None
            } else {
                Some(SearchResult {
                    video_path: file_path.to_string_lossy().to_string(),
                    transcript_path: txt_path.to_string_lossy().to_string(),
                    line_numbers,
                })
            }
        })
        .collect();

    Ok(results)
}