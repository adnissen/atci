use std::fs;
use std::path::Path;
use std::process::Command;
use std::env;
use regex::Regex;

pub fn create_transcript(video_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating transcript for: {}", video_path);
    let cfg: crate::AtciConfig = confy::load("atci", "config")?;
    let video_path_obj = Path::new(video_path);
    let txt_path = video_path_obj.with_extension("txt");
    
    if !txt_path.exists() {
        // Check for subtitle streams first
        match get_subtitle_streams(video_path, &cfg.ffprobe_path) {
            Ok(streams) => {
                if !streams.is_empty() {
                    println!("Found subtitle streams: {:?}", streams);
                    // Extract subtitles from the first stream
                    match extract_subtitle_stream(video_path, streams[0], &cfg.ffmpeg_path) {
                        Ok(()) => {
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

pub fn create_metafile(video_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let video_path = Path::new(video_path);
    let meta_path = video_path.with_extension("meta");
    
    if !meta_path.exists() {
        fs::write(&meta_path, "")?;
        println!("Created meta file: {}", meta_path.display());
    }
    
    Ok(())
}

pub fn get_subtitle_streams(video_path: &str, ffprobe_path: &str) -> Result<Vec<usize>, String> {
    let ffprobe_path = if ffprobe_path.is_empty() {
        "ffprobe"
    } else {
        ffprobe_path
    };

    let output = Command::new(ffprobe_path)
        .args(&[
            "-v", "error",
            "-select_streams", "s",
            "-show_entries", "stream=index,codec_name,codec_type",
            "-of", "csv=p=0",
            video_path
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

pub fn extract_subtitle_stream(video_path: &str, stream_index: usize, ffmpeg_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let video_path_obj = Path::new(video_path);
    let txt_path = video_path_obj.with_extension("txt");
    
    let temp_dir = env::temp_dir();
    let temp_srt_path = temp_dir.join("temp_subtitle.srt");
    
    let ffmpeg_path = if ffmpeg_path.is_empty() {
        "ffmpeg"
    } else {
        ffmpeg_path
    };
    
    let output = Command::new(ffmpeg_path)
        .args(&[
            "-i", video_path,
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