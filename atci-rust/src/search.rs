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
    let mut results = Vec::new();

    for watch_directory in &cfg.watch_directories {
        for entry in WalkDir::new(watch_directory).into_iter().filter_map(|e| e.ok()) {
            let file_path = entry.path();
            
            if file_path.is_file() {
                if let Some(extension) = file_path.extension() {
                    if let Some(ext_str) = extension.to_str() {
                        if video_extensions.contains(&ext_str.to_lowercase().as_str()) {
                            let txt_path = file_path.with_extension("txt");
                            
                            if txt_path.exists() {
                                if let Ok(content) = fs::read_to_string(&txt_path) {
                                    let mut line_numbers = Vec::new();
                                    
                                    for (line_num, line) in content.lines().enumerate() {
                                        if line.to_lowercase().contains(&query.to_lowercase()) {
                                            line_numbers.push(line_num + 1);
                                        }
                                    }
                                    
                                    if !line_numbers.is_empty() {
                                        results.push(SearchResult {
                                            video_path: file_path.to_string_lossy().to_string(),
                                            transcript_path: txt_path.to_string_lossy().to_string(),
                                            line_numbers,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(results)
}