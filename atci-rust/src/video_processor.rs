use std::fs;
use std::path::Path;
use std::process::Command;
use std::env;
use regex::Regex;
use std::io::{BufRead, BufReader};

pub fn create_transcript(video_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating transcript for: {}", video_path.display());
    let cfg: crate::AtciConfig = confy::load("atci", "config")?;
    let video_path_obj = Path::new(video_path);
    let txt_path = video_path_obj.with_extension("txt");
    
    if !txt_path.exists() {
        // Check for subtitle streams first
        match get_subtitle_streams(video_path, Path::new(&cfg.ffprobe_path)) {
            Ok(streams) => {
                if !streams.is_empty() {
                    println!("Found subtitle streams: {:?}", streams);
                    // Extract subtitles from the first stream
                    match extract_subtitle_stream(video_path, streams[0], Path::new(&cfg.ffmpeg_path)) {
                        Ok(()) => {
                            write_key_to_meta_file(video_path_obj, "source", "subtitles")?;
                            println!("Created transcript file: {}", txt_path.display());
                        }
                        Err(e) => {
                            eprintln!("Failed to extract subtitles: {}, creating empty transcript file", e);
                            fs::write(&txt_path, "")?;
                            println!("Created empty transcript file: {}", txt_path.display());
                        }
                    }
                } else {
                    // No subtitle streams found, create empty file
                    fs::write(&txt_path, "")?;
                    println!("No subtitle streams found, created empty transcript file: {}", txt_path.display());
                }
            }
            Err(e) => {
                eprintln!("Failed to check for subtitle streams: {}, creating empty transcript file", e);
                fs::write(&txt_path, "")?;
                println!("Created empty transcript file: {}", txt_path.display());
            }
        }
    }
    
    Ok(())
}

pub fn create_metafile(video_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let meta_path = video_path.with_extension("meta");
    let cfg: crate::AtciConfig = confy::load("atci", "config")?;
    write_key_to_meta_file(video_path, "length", &get_video_length(video_path, Path::new(&cfg.ffprobe_path))?)?;
    println!("Created or updated meta file: {}", meta_path.display());
    
    Ok(())
}

pub fn write_key_to_meta_file(video_path: &Path, key: &str, value: &str) -> Result<(), Box<dyn std::error::Error>> {
    let video_path = Path::new(video_path);
    let meta_path = video_path.with_extension("meta");
    
    // Read existing content
    let mut lines = Vec::new();
    let mut key_found = false;
    
    if meta_path.exists() {
        let file = fs::File::open(&meta_path)?;
        let reader = BufReader::new(file);
        
        for line in reader.lines() {
            let line = line?;
            if line.starts_with(&format!("{}:", key)) {
                lines.push(format!("{}: {}", key, value));
                key_found = true;
            } else {
                lines.push(line);
            }
        }
    }
    
    // If key wasn't found, add it
    if !key_found {
        lines.push(format!("{}: {}", key, value));
    }
    
    // Write back to file
    fs::write(&meta_path, lines.join("\n"))?;
    
    Ok(())
}

pub fn get_video_length(video_path: &Path, ffprobe_path: &Path) -> Result<String, String> {
    let output = Command::new(ffprobe_path)
        .args(&[
            "-v", "error",
            "-show_entries", "format=duration",
            "-of", "default=noprint_wrappers=1:nokey=1",
            video_path.to_str().unwrap()
        ])
        .output().expect("Failed to execute ffprobe to get video length");


    if output.status.success() {
        let duration_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if let Ok(duration) = duration_str.parse::<f64>() {
            let total_seconds = duration.round() as u64;
            let hours = total_seconds / 3600;
            let minutes = (total_seconds % 3600) / 60;
            let seconds = total_seconds % 60;
            let formatted = format!("{:02}:{:02}:{:02}", hours, minutes, seconds);
            Ok(formatted)
        } else {
            Err(format!("Failed to parse duration: {}", duration_str))
        }
    } else {
        let error_output = String::from_utf8_lossy(&output.stderr);
        Err(format!("ffprobe failed: {}", error_output))
    }

}

pub fn get_subtitle_streams(video_path: &Path, ffprobe_path: &Path) -> Result<Vec<usize>, String> {
    let output = Command::new(ffprobe_path)
        .args(&[
            "-v", "error",
            "-select_streams", "s",
            "-show_entries", "stream=index,codec_name,codec_type",
            "-of", "csv=p=0",
            video_path.to_str().unwrap()
        ])
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                let streams: Vec<usize> = output_str
                    .trim()
                    .split('\n')
                    .filter(|line| !line.trim().is_empty())
                    .filter_map(|line| {
                        let parts: Vec<&str> = line.split(',').collect();
                        if parts.len() >= 3 && parts[2] == "subtitle" {
                            parts[0].parse::<usize>().ok()
                        } else {
                            None
                        }
                    })
                    .collect();

                Ok(streams)
            } else {
                let error_output = String::from_utf8_lossy(&output.stderr);
                Err(format!("ffprobe failed: {}", error_output))
            }
        }
        Err(e) => Err(format!("Failed to execute ffprobe: {}", e)),
    }
}

pub fn extract_subtitle_stream(video_path: &Path, stream_index: usize, ffmpeg_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let video_path_obj = Path::new(video_path);
    let txt_path = video_path_obj.with_extension("txt");
    
    let temp_dir = env::temp_dir();
    let temp_srt_path = temp_dir.join("temp_subtitle.srt");
    
    let output = Command::new(ffmpeg_path)
        .args(&[
            "-i", video_path.to_str().unwrap(),
            "-map", &format!("0:{}", stream_index),
            "-c:s", "srt",
            "-y", temp_srt_path.to_str().unwrap()
        ])
        .output();
    
    match output {
        Ok(output) => {
            if output.status.success() {
                match parse_srt_content(&temp_srt_path) {
                    Ok(transcript_content) => {
                        fs::write(&txt_path, transcript_content)?;
                        let _ = fs::remove_file(&temp_srt_path);
                        Ok(())
                    }
                    Err(reason) => {
                        let _ = fs::remove_file(&temp_srt_path);
                        Err(reason.into())
                    }
                }
            } else {
                let error_output = String::from_utf8_lossy(&output.stderr);
                Err(format!("ffmpeg subtitle extraction failed: {}", error_output).into())
            }
        }
        Err(e) => Err(format!("Failed to execute ffmpeg: {}", e).into()),
    }
}

fn parse_srt_content(srt_path: &Path) -> Result<String, String> {
    let content = fs::read_to_string(srt_path)
        .map_err(|e| format!("Failed to read SRT file: {}", e))?;

    // Split content into subtitle blocks
    let cleaned_content = content
        .trim()
        .replace('\r', "");
    let blocks: Vec<&str> = cleaned_content
        .split("\n\n")
        .filter(|block| !block.trim().is_empty())
        .collect();
    
    let timestamp_regex = Regex::new(r"^(\d{2}:\d{2}:\d{2}),(\d{3}) --> (\d{2}:\d{2}:\d{2}),(\d{3})").unwrap();
    
    let processed_blocks: Vec<String> = blocks
        .iter()
        .filter_map(|block| {
            let lines: Vec<&str> = block.split('\n').collect();
            
            if lines.len() >= 3 {
                let timestamp_line = lines[1];
                let text_lines = &lines[2..];
                
                if !text_lines.is_empty() {
                    if let Some(caps) = timestamp_regex.captures(timestamp_line) {
                        let start_time = &caps[1];
                        let start_millis = &caps[2];
                        let end_time = &caps[3];
                        let end_millis = &caps[4];
                        
                        // Convert to our format with period instead of comma
                        let start_timestamp = format!("{}.{}", start_time, start_millis);
                        let end_timestamp = format!("{}.{}", end_time, end_millis);
                        let text = text_lines.join(" ");
                        
                        Some(format!("{} --> {}\n{}", start_timestamp, end_timestamp, text))
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();
    
    Ok(processed_blocks.join("\n\n"))
}