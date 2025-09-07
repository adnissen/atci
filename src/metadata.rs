// atci (andrew's transcript and clipping interface)
// Copyright (C) 2025 Andrew Nissen

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub const META_FIELDS: &[&str] = &[
    "length",
    "source",
];

#[derive(Debug, Clone, Default)]
pub struct Metadata {
    pub length: Option<String>,
    pub source: Option<String>,
}

pub fn get_metadata_fields(video_path: &Path) -> Option<Metadata> {
    let text_path = video_path.with_extension("txt");
    
    let file = match File::open(&text_path) {
        Ok(file) => file,
        Err(_) => return None,
    };
    
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader
        .lines()
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_default();
    
    let mut metadata = Metadata {
        length: None,
        source: None,
    };
    
    for line in lines {
        if line.starts_with("length:") {
            metadata.length = Some(line.trim_start_matches("length:").trim().to_string());
        } else if line.starts_with("source:") {
            metadata.source = Some(line.trim_start_matches("source:").trim().to_string());
        }
    }
    
    Some(metadata)
}
