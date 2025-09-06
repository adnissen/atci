// atci (andrew's transcript and clipping interface)
// Copyright (C) 2025 Andrew Nissen

use std::fs;
use std::path::Path;
use tokio::process::Command;
use std::env;
use regex::Regex;
use std::io::{BufRead, BufReader};
use crate::metadata;
use tokio::time::sleep;
use std::time::Duration;

fn check_cancel_file() -> bool {
    let home_dir = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    let cancel_file = home_dir.join(".atci").join(".commands").join("CANCEL");
    cancel_file.exists()
}

fn cleanup_cancel_and_processing_files(video_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let home_dir = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    let cancel_file = home_dir.join(".atci").join(".commands").join("CANCEL");
    let currently_processing_path = home_dir.join(".atci/.currently_processing");
    let mp3_path = video_path.with_extension("mp3");
    
    // Remove cancel file
    if cancel_file.exists() {
        let _ = fs::remove_file(&cancel_file);
    }
    
    // Remove currently processing file
    if currently_processing_path.exists() {
        let _ = fs::remove_file(&currently_processing_path);
    }
    
    // Remove mp3 file if it exists
    if mp3_path.exists() {
        let _ = fs::remove_file(&mp3_path);
    }
    
    Ok(())
}




pub fn add_key_to_metadata_block(video_path: &Path, key: &str, value: &str) -> Result<(), Box<dyn std::error::Error>> {
    let video_path = Path::new(video_path);
    let txt_path = video_path.with_extension("txt");
    
    // Read existing content
    let mut lines = Vec::new();
    let mut key_found = false;
    
    if txt_path.exists() {
        let file = fs::File::open(&txt_path)?;
        let reader = BufReader::new(file);
        
        for (i, line) in reader.lines().enumerate() {
            let line = line?;
            
            // we're going to re-add the metadata end marker later
            if line == ">>>.atcimetaend" {
                continue;
            }
            
            // Check if this line is a metadata line (key: value format at the top)
            if line.starts_with(&format!("{}:", key)) && i < metadata::META_FIELDS.len() {
                lines.push(format!("{}: {}", key, value));
                key_found = true;
            } else {
                // This is content, not metadata
                lines.push(line);
            }
        }
    }
    
    // If key wasn't found, add it at the top
    if !key_found {
        lines.insert(0, format!("{}: {}", key, value));
    }
    
    let mut insert_pos = 0;
    // Add the metadata end marker
    // If we have content, insert before the first non-empty content line
    for (i, line) in lines.iter().enumerate() {
        if !line.contains(": ") {
            insert_pos = i;
            break;
        }
    }
    
    lines.insert(insert_pos, ">>>.atcimetaend".to_string());

    // Write back to file
    fs::write(&txt_path, lines.join("\n"))?;
    
    Ok(())
}


pub async fn get_subtitle_streams(video_path: &Path, ffprobe_path: &Path) -> Result<Vec<usize>, String> {
    let output = Command::new(ffprobe_path)
        .args(&[
            "-v", "error",
            "-select_streams", "s",
            "-show_entries", "stream=index,codec_name,codec_type",
            "-of", "csv=p=0",
            video_path.to_str().unwrap()
        ])
        .output().await;

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

pub async fn extract_subtitle_stream(video_path: &Path, stream_index: usize, ffmpeg_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
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
        .output().await;
    
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

pub async fn has_audio_stream(video_path: &Path, ffprobe_path: &Path) -> Result<bool, String> {
    let output = Command::new(ffprobe_path)
        .args(&[
            "-v",
            "error",
            "-select_streams",
            "a",
            "-show_entries",
            "stream=index",
            "-of",
            "csv=p=0",
            video_path.to_str().unwrap()
        ])
        .output().await;

    match output {
        Ok(output) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if !stdout.trim().is_empty() {
                    Ok(true)
                } else {
                    Ok(false)
                }
            } else {
                let error_output = String::from_utf8_lossy(&output.stderr);
                Err(format!("ffprobe failed: {}", error_output))
            }
        }
        Err(e) => Err(format!("Failed to execute ffprobe: {}", e)),
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
    
    //add a newline to the start of the file so our metadata block has space before the content starts
    Ok(format!("\n{}", processed_blocks.join("\n\n")))
}

pub async fn cancellable_create_transcript(video_path: &Path) -> Result<bool, Box<dyn std::error::Error>> {
    let cfg: crate::AtciConfig = crate::config::load_config()?;
    let txt_path = video_path.with_extension("txt");
    
    if txt_path.exists() {
        return Ok(true); // Already exists, no need to create
    }
    
    println!("Creating transcript for: {}", video_path.display());
    
    // Check for subtitle streams first
    match get_subtitle_streams(video_path, Path::new(&cfg.ffprobe_path)).await {
        Ok(streams) => {
            if !streams.is_empty() {
                println!("Found subtitle streams: {:?}", streams);
                match extract_subtitle_stream(video_path, streams[0], Path::new(&cfg.ffmpeg_path)).await {
                    Ok(()) => {
                        add_key_to_metadata_block(video_path, "source", "subtitles")?;
                        println!("Created transcript file: {}", txt_path.display());
                        return Ok(true);
                    }
                    Err(e) => {
                        println!("Failed to extract subtitles: {}, trying whisper transcription", e);
                    }
                }
            }
            
            // Check if we should cancel before proceeding with audio extraction
            if check_cancel_file() {
                cleanup_cancel_and_processing_files(video_path)?;
                return Ok(false);
            }
            
            // Check for audio streams
            if !(has_audio_stream(video_path, Path::new(&cfg.ffprobe_path)).await?) {
                fs::write(&txt_path, "")?;
                println!("No audio streams found, created empty transcript file: {}", txt_path.display());
                return Ok(true);
            }
            
            // Extract audio with cancellation check
            println!("Extracting audio");
            let audio_path = video_path.with_extension("mp3");
            
            // Start audio extraction in background
            let mut child = Command::new(&cfg.ffmpeg_path)
                .args(&[
                    "-i", video_path.to_str().unwrap(),
                    "-map", "0:a:0",
                    "-q:a", "0",
                    "-ac", "1",
                    "-ar", "16000",
                    "-y", audio_path.to_str().unwrap()
                ])
                .stdout(std::process::Stdio::null())
                .spawn()?;
            
            // Wait for completion while checking for cancellation
            loop {
                tokio::select! {
                    result = child.wait() => {
                        match result {
                            Ok(status) => {
                                if !status.success() {
                                    return Err("Audio extraction failed".into());
                                }
                                break;
                            }
                            Err(e) => return Err(format!("Failed to execute ffmpeg: {}", e).into()),
                        }
                    }
                    _ = sleep(Duration::from_millis(500)) => {
                        if check_cancel_file() {
                            let _ = child.kill().await;
                            cleanup_cancel_and_processing_files(video_path)?;
                            return Ok(false);
                        }
                    }
                }
            }
            
            // Check for cancellation before transcription
            if check_cancel_file() {
                cleanup_cancel_and_processing_files(video_path)?;
                return Ok(false);
            }
            
            // Transcribe audio with cancellation check
            println!("Transcribing audio");
            let home_dir = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
            let model_path = home_dir.join(".atci/models").join(format!("{}.bin", cfg.model_name));
            
            let mut child = Command::new(&cfg.whispercli_path)
                .args(&[
                    "-m", model_path.to_str().unwrap(),
                    "-np",
                    "--max-context", "0",
                    "-ovtt",
                    "-f", audio_path.to_str().unwrap()
                ])
                .stdout(std::process::Stdio::null())
                .spawn()?;
            
            // Wait for completion while checking for cancellation
            loop {
                tokio::select! {
                    result = child.wait() => {
                        match result {
                            Ok(status) => {
                                if !status.success() {
                                    return Err("Whisper transcription failed".into());
                                }
                                break;
                            }
                            Err(e) => return Err(format!("Failed to execute whisper: {}", e).into()),
                        }
                    }
                    _ = sleep(Duration::from_millis(500)) => {
                        if check_cancel_file() {
                            let _ = child.kill().await;
                            cleanup_cancel_and_processing_files(video_path)?;
                            return Ok(false);
                        }
                    }
                }
            }
            
            // Post-process the whisper output
            let vtt_path = audio_path.with_extension("mp3.vtt");
            if vtt_path.exists() {
                // Remove the first line of the vtt file
                let content = fs::read_to_string(&vtt_path)?;
                let lines: Vec<&str> = content.lines().collect();
                if lines.len() > 1 {
                    let new_content = lines[1..].join("\n");
                    fs::write(&vtt_path, new_content)?;
                }
                
                let txt_path = audio_path.with_extension("txt");
                fs::rename(&vtt_path, &txt_path)?;
                let _ = fs::remove_file(&audio_path);
                
                add_key_to_metadata_block(video_path, "source", &cfg.model_name)?;
                println!("Successfully created transcript: {}", txt_path.display());
            }
            
            Ok(true)
        }
        Err(e) => {
            eprintln!("Failed to check for subtitle streams: {}, creating empty transcript file", e);
            fs::write(&txt_path, "")?;
            println!("Created empty transcript file: {}", txt_path.display());
            Ok(true)
        }
    }
}

pub async fn cancellable_add_length_to_metadata(video_path: &Path) -> Result<bool, Box<dyn std::error::Error>> {
    if check_cancel_file() {
        cleanup_cancel_and_processing_files(video_path)?;
        return Ok(false);
    }
    
    let cfg: crate::AtciConfig = crate::config::load_config()?;
    
    let output = Command::new(&cfg.ffprobe_path)
        .args(&[
            "-v", "error",
            "-show_entries", "format=duration",
            "-of", "default=noprint_wrappers=1:nokey=1",
            video_path.to_str().unwrap()
        ])
        .output().await?;

    if !output.status.success() {
        let error_output = String::from_utf8_lossy(&output.stderr);
        return Err(format!("ffprobe failed: {}", error_output).into());
    }

    let duration_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if let Ok(duration) = duration_str.parse::<f64>() {
        let total_seconds = duration.round() as u64;
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;
        let formatted = format!("{:02}:{:02}:{:02}", hours, minutes, seconds);
        
        add_key_to_metadata_block(video_path, "length", &formatted)?;
        println!("Created or updated meta file: {}", video_path.with_extension("meta").display());
        Ok(true)
    } else {
        Err(format!("Failed to parse duration: {}", duration_str).into())
    }
}