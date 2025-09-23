// atci (andrew's transcript and clipping interface)
// Copyright (C) 2025 Andrew Nissen

use crate::metadata;
use regex::Regex;
use rocket::serde::json::Json;
use rocket::{get, response::status::BadRequest};
use std::env;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::sleep;

/// Helper function to execute success command for processed videos
fn execute_success_command(video_path: &Path) {
    if let Ok(cfg_for_command) = crate::config::load_config()
        && !cfg_for_command.processing_success_command.is_empty()
        && let Err(e) = crate::config::execute_processing_command(
            &cfg_for_command.processing_success_command,
            video_path,
            true,
        )
    {
        eprintln!(
            "Error executing success command for {}: {}",
            video_path.display(),
            e
        );
    }
}

/// Helper function to execute failure command for failed videos
fn execute_failure_command(video_path: &Path) {
    if let Ok(cfg_for_command) = crate::config::load_config()
        && !cfg_for_command.processing_failure_command.is_empty()
        && let Err(e) = crate::config::execute_processing_command(
            &cfg_for_command.processing_failure_command,
            video_path,
            false,
        )
    {
        eprintln!(
            "Error executing failure command for {}: {}",
            video_path.display(),
            e
        );
    }
}

fn check_cancel_file() -> bool {
    let home_dir = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    let cancel_file = home_dir.join(".atci").join(".commands").join("CANCEL");
    cancel_file.exists()
}

fn cleanup_cancel_and_processing_files(
    video_path: &Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let home_dir = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    let cancel_file = home_dir.join(".atci").join(".commands").join("CANCEL");
    let mp3_path = video_path.with_extension("mp3");

    // Remove cancel file
    if cancel_file.exists() {
        let _ = fs::remove_file(&cancel_file);
    }

    // Remove currently processing entry from database
    if let Ok(conn) = crate::db::get_connection() {
        let video_path_str = video_path.to_string_lossy();
        let _ = conn.execute(
            "DELETE FROM currently_processing WHERE path = ?1",
            [video_path_str.as_ref()],
        );
    }

    // Remove mp3 file if it exists
    if mp3_path.exists() {
        let _ = fs::remove_file(&mp3_path);
    }

    Ok(())
}

pub fn add_key_to_metadata_block(
    video_path: &Path,
    key: &str,
    value: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Check if this is a video part file - if so, don't add metadata (only add to master files)
    if let Some(video_part) = crate::video_parts::parse_video_part(video_path) {
        if video_part.part_number != 1 {
            // For parts other than part 1, skip metadata addition
            return Ok(());
        }
        // For part 1, try to add metadata to the master file if it exists
        let (_, master_transcript_path) = crate::video_parts::get_master_paths(&video_part);
        let master_transcript = Path::new(&master_transcript_path);
        if master_transcript.exists() {
            // Add metadata to master transcript instead
            return add_key_to_metadata_block(master_transcript, key, value);
        }
    }

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

    let mut insert_pos = 0;

    // If key wasn't found, add it at the top
    if !key_found {
        lines.insert(0, format!("{}: {}", key, value));
        insert_pos = 1;
    }

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

#[derive(Debug, Clone, rocket::serde::Serialize)]
pub struct SubtitleStream {
    pub index: usize,
    pub language: Option<String>,
}

fn expand_language_code(code: &str) -> String {
    match code.to_lowercase().as_str() {
        "eng" => "English".to_string(),
        "fre" | "fra" => "French".to_string(),
        "ger" | "deu" => "German".to_string(),
        "spa" | "es" => "Spanish".to_string(),
        "ita" | "it" => "Italian".to_string(),
        "por" | "pt" => "Portuguese".to_string(),
        "rus" | "ru" => "Russian".to_string(),
        "jpn" | "ja" => "Japanese".to_string(),
        "chi" | "zho" | "zh" => "Chinese".to_string(),
        "kor" | "ko" => "Korean".to_string(),
        "ara" | "ar" => "Arabic".to_string(),
        "hin" | "hi" => "Hindi".to_string(),
        "dut" | "nld" | "nl" => "Dutch".to_string(),
        "swe" | "sv" => "Swedish".to_string(),
        "nor" | "no" => "Norwegian".to_string(),
        "dan" | "da" => "Danish".to_string(),
        "fin" | "fi" => "Finnish".to_string(),
        "pol" | "pl" => "Polish".to_string(),
        "cze" | "ces" | "cs" => "Czech".to_string(),
        "hun" | "hu" => "Hungarian".to_string(),
        "tur" | "tr" => "Turkish".to_string(),
        "gre" | "ell" | "el" => "Greek".to_string(),
        "heb" | "he" => "Hebrew".to_string(),
        "tha" | "th" => "Thai".to_string(),
        "vie" | "vi" => "Vietnamese".to_string(),
        "ukr" | "uk" => "Ukrainian".to_string(),
        "bul" | "bg" => "Bulgarian".to_string(),
        "hrv" | "hr" => "Croatian".to_string(),
        "srp" | "sr" => "Serbian".to_string(),
        "slv" | "sl" => "Slovenian".to_string(),
        "slk" | "sk" => "Slovak".to_string(),
        "ron" | "ro" => "Romanian".to_string(),
        "lit" | "lt" => "Lithuanian".to_string(),
        "lav" | "lv" => "Latvian".to_string(),
        "est" | "et" => "Estonian".to_string(),
        "cat" | "ca" => "Catalan".to_string(),
        "baq" | "eus" | "eu" => "Basque".to_string(),
        "glg" | "gl" => "Galician".to_string(),
        "ice" | "isl" | "is" => "Icelandic".to_string(),
        "iri" | "gle" | "ga" => "Irish".to_string(),
        "wel" | "cym" | "cy" => "Welsh".to_string(),
        "sco" | "gd" => "Scottish Gaelic".to_string(),
        "mal" | "ms" => "Malay".to_string(),
        "ind" | "id" => "Indonesian".to_string(),
        "tgl" | "tl" => "Tagalog".to_string(),
        _ => code.to_uppercase(),
    }
}

impl SubtitleStream {
    pub fn language_display(&self) -> String {
        match &self.language {
            Some(code) => expand_language_code(code),
            None => "Unknown".to_string(),
        }
    }
}

pub async fn get_subtitle_streams(
    video_path: &Path,
    ffprobe_path: &Path,
) -> Result<Vec<SubtitleStream>, String> {
    let output = Command::new(ffprobe_path)
        .args([
            "-v",
            "error",
            "-select_streams",
            "s",
            "-show_entries",
            "stream=index,codec_name,codec_type,tags:stream_tags=language",
            "-of",
            "csv=p=0",
            video_path.to_str().unwrap(),
        ])
        .output()
        .await;

    match output {
        Ok(output) => {
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                let streams: Vec<SubtitleStream> = output_str
                    .trim()
                    .split('\n')
                    .filter(|line| !line.trim().is_empty())
                    .filter_map(|line| {
                        let parts: Vec<&str> = line.split(',').collect();
                        if parts.len() >= 3 && parts[2] == "subtitle" {
                            if let Ok(index) = parts[0].parse::<usize>() {
                                let language =
                                    if parts.len() > 3 && !parts[3].is_empty() && parts[3] != "N/A"
                                    {
                                        Some(expand_language_code(parts[3]))
                                    } else {
                                        None
                                    };
                                Some(SubtitleStream { index, language })
                            } else {
                                None
                            }
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

pub async fn extract_subtitle_stream(
    video_path: &Path,
    stream_index: usize,
    ffmpeg_path: &Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let video_path_obj = Path::new(video_path);
    let txt_path = video_path_obj.with_extension("txt");

    let temp_dir = env::temp_dir();
    let temp_srt_path = temp_dir.join("temp_subtitle.srt");

    let output = Command::new(ffmpeg_path)
        .args([
            "-i",
            video_path.to_str().unwrap(),
            "-map",
            &format!("0:{}", stream_index),
            "-c:s",
            "srt",
            "-y",
            temp_srt_path.to_str().unwrap(),
        ])
        .output()
        .await;

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

pub async fn get_video_duration(video_path: &Path, ffprobe_path: &Path) -> Result<String, String> {
    let output = Command::new(ffprobe_path)
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            video_path.to_str().unwrap(),
        ])
        .output()
        .await;

    match output {
        Ok(output) => {
            if output.status.success() {
                let duration_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if let Ok(duration) = duration_str.parse::<f64>() {
                    let total_seconds = duration.round() as u64;
                    let hours = total_seconds / 3600;
                    let minutes = (total_seconds % 3600) / 60;
                    let seconds = total_seconds % 60;
                    Ok(format!("{:02}:{:02}:{:02}", hours, minutes, seconds))
                } else {
                    Err(format!("Failed to parse duration: {}", duration_str))
                }
            } else {
                let error_output = String::from_utf8_lossy(&output.stderr);
                Err(format!("ffprobe failed: {}", error_output))
            }
        }
        Err(e) => Err(format!("Failed to execute ffprobe: {}", e)),
    }
}

pub async fn has_audio_stream(video_path: &Path, ffprobe_path: &Path) -> Result<bool, String> {
    let output = Command::new(ffprobe_path)
        .args([
            "-v",
            "error",
            "-select_streams",
            "a",
            "-show_entries",
            "stream=index",
            "-of",
            "csv=p=0",
            video_path.to_str().unwrap(),
        ])
        .output()
        .await;

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

fn strip_html_tags(text: &str) -> String {
    let tag_regex = Regex::new(r"<[^>]*>").unwrap();
    tag_regex.replace_all(text, "").to_string()
}

fn parse_srt_content(srt_path: &Path) -> Result<String, String> {
    let content =
        fs::read_to_string(srt_path).map_err(|e| format!("Failed to read SRT file: {}", e))?;

    // Split content into subtitle blocks
    let cleaned_content = content.trim().replace('\r', "");
    let blocks: Vec<&str> = cleaned_content
        .split("\n\n")
        .filter(|block| !block.trim().is_empty())
        .collect();

    let timestamp_regex =
        Regex::new(r"^(\d{2}:\d{2}:\d{2}),(\d{3}) --> (\d{2}:\d{2}:\d{2}),(\d{3})").unwrap();

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
                        let cleaned_text = strip_html_tags(&text);

                        Some(format!(
                            "{} --> {}\n{}",
                            start_timestamp, end_timestamp, cleaned_text
                        ))
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

/// Main entry point for transcript creation that handles both regular videos and video parts
pub async fn cancellable_create_transcript(
    video_path: &Path,
    model: Option<String>,
    subtitle_stream_index: Option<i32>,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    // Check if this is a video part
    if let Some(video_part) = crate::video_parts::parse_video_part(video_path) {
        cancellable_create_transcript_for_part(video_part, model, subtitle_stream_index).await
    } else {
        // For single videos, wrap with failure command handling
        match cancellable_create_transcript_single(video_path, model, subtitle_stream_index).await {
            Ok(success) => {
                execute_success_command(video_path);
                Ok(success)
            }
            Err(e) => {
                // Execute failure command for single video processing
                execute_failure_command(video_path);
                Err(e)
            }
        }
    }
}

/// Create transcript for a video part with proper concatenation and timestamp handling
pub async fn cancellable_create_transcript_for_part(
    video_part: crate::video_parts::VideoPart,
    model: Option<String>,
    subtitle_stream_index: Option<i32>,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::db::get_connection()?;
    let video_path = Path::new(&video_part.video_path);
    let (master_video_path, master_transcript_path) =
        crate::video_parts::get_master_paths(&video_part);

    println!(
        "Processing video part: {} (Part {})",
        video_part.base_name, video_part.part_number
    );

    // Check for missing previous parts
    let missing_parts = crate::video_parts::find_missing_parts(
        &conn,
        &video_part.base_name,
        video_part.part_number - 1,
    )?;
    if !missing_parts.is_empty() {
        println!("Missing previous parts: {:?}", missing_parts);
        crate::video_parts::create_missing_part_placeholder(
            &master_transcript_path,
            &missing_parts,
            video_part.part_number,
        )?;
        return Ok(true);
    }

    // Create transcript for this part using the normal single video logic
    let success = match cancellable_create_transcript_single(
        video_path,
        model.clone(),
        subtitle_stream_index,
    )
    .await
    {
        Ok(success) => success,
        Err(e) => {
            // Execute failure command for the part file
            let cfg_for_command = crate::config::load_config()?;
            if !cfg_for_command.processing_failure_command.is_empty()
                && let Err(cmd_err) = crate::config::execute_processing_command(
                    &cfg_for_command.processing_failure_command,
                    video_path,
                    false,
                )
            {
                eprintln!(
                    "Error executing failure command for part {}: {}",
                    video_part.part_number, cmd_err
                );
            }

            // Handle processing failure - insert error message into master transcript
            let error_message = format!(
                ">>> Part {} FAILED: {} <<<\nError processing part {}: {}\n",
                video_part.part_number, video_part.base_name, video_part.part_number, e
            );

            let final_content =
                if video_part.part_number == 1 || !Path::new(&master_transcript_path).exists() {
                    error_message
                } else {
                    let existing_content =
                        std::fs::read_to_string(&master_transcript_path).unwrap_or_default();
                    format!("{}\n{}", existing_content, error_message)
                };

            std::fs::write(&master_transcript_path, final_content)?;

            // Create error transcript for the part video as well
            let part_error_transcript =
                format!("Error processing part {}: {}\n", video_part.part_number, e);
            std::fs::write(video_path.with_extension("txt"), part_error_transcript)?;

            // Don't delete the part video on failure - keep it for debugging
            println!(
                "Failed to process part {} of {}: {}",
                video_part.part_number, video_part.base_name, e
            );

            // Still check for next part and queue it
            let _ = crate::video_parts::check_and_queue_next_part(&video_part);

            return Ok(true); // Return true to continue processing other parts
        }
    };

    if !success {
        return Ok(false); // Cancelled
    }

    // Read the part transcript that was just created
    let part_transcript_path = video_path.with_extension("txt");
    let part_transcript = std::fs::read_to_string(&part_transcript_path)?;

    // Calculate timestamp offset based on existing parts
    let existing_parts = crate::video_parts::get_processed_parts(&conn, &video_part.base_name)?;
    let mut total_duration_ms = 0u64;

    // Calculate cumulative duration from all previous parts
    for part_num in &existing_parts {
        if *part_num < video_part.part_number {
            // Try to get duration from the master video first, then from the part file
            let master_video_exists = Path::new(&master_video_path).exists();
            let cfg = crate::config::load_config()?;

            if master_video_exists && *part_num == *existing_parts.iter().max().unwrap_or(&0) {
                // If master video exists and this is the last processed part, get duration from master
                if let Ok(duration_str) =
                    get_video_duration(Path::new(&master_video_path), Path::new(&cfg.ffprobe_path))
                        .await
                {
                    total_duration_ms = parse_duration_to_ms(&duration_str)?;
                    break; // We have the total duration already
                }
            } else {
                // Try to get duration from individual part file
                let part_video_path = format!(
                    "{}/{}.part{}.{}",
                    Path::new(&video_part.video_path)
                        .parent()
                        .unwrap()
                        .display(),
                    video_part.base_name,
                    *part_num,
                    video_part.extension
                );

                if let Ok(duration_str) =
                    get_video_duration(Path::new(&part_video_path), Path::new(&cfg.ffprobe_path))
                        .await
                {
                    total_duration_ms += parse_duration_to_ms(&duration_str)?;
                }
            }
        }
    }

    // Adjust timestamps in the transcript
    let adjusted_transcript = adjust_transcript_timestamps(&part_transcript, total_duration_ms)?;

    // Append or create master transcript
    let final_content = if !Path::new(&master_transcript_path).exists() {
        // New master file - start with this part's content
        adjusted_transcript.clone()
    } else {
        // Append to existing master transcript
        let existing_content = std::fs::read_to_string(&master_transcript_path).unwrap_or_default();
        let trimmed_existing = existing_content.trim_end();
        let trimmed_new = adjusted_transcript.trim_start();
        format!("{}\n\n{}", trimmed_existing, trimmed_new)
    };

    std::fs::write(&master_transcript_path, final_content)?;

    // Record this part as processed
    let transcript_lines = adjusted_transcript.lines().count() as i32;
    crate::video_parts::record_processed_part(&conn, &video_part, transcript_lines)?;

    // Clean up part transcript (but keep part video for concatenation)
    let _ = std::fs::remove_file(&part_transcript_path);

    // Update or create master video (after transcript cleanup but before video cleanup)
    update_master_video(&video_part).await?;

    // Update master video length metadata after concatenation
    let (master_video_path, _) = crate::video_parts::get_master_paths(&video_part);
    let master_path = Path::new(&master_video_path);
    if master_path.exists() {
        if let Err(e) = add_length_metadata_to_path(master_path).await {
            eprintln!("Failed to update master video length metadata: {}", e);
        } else {
            println!(
                "Updated length metadata for master video: {}",
                master_video_path
            );
        }
    }

    // Execute success command for the part file before deletion
    execute_success_command(video_path);

    // Check for next part and queue it
    let _ = crate::video_parts::check_and_queue_next_part(&video_part);

    println!(
        "Successfully processed part {} of {}",
        video_part.part_number, video_part.base_name
    );
    Ok(true)
}

/// Original transcript creation logic for single videos (not parts)
pub async fn cancellable_create_transcript_single(
    video_path: &Path,
    model: Option<String>,
    subtitle_stream_index: Option<i32>,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let cfg: crate::AtciConfig = crate::config::load_config()?;
    let txt_path = video_path.with_extension("txt");

    println!("Creating transcript for: {}", video_path.display());

    // Check for subtitle streams first (if allowed)
    if cfg.allow_subtitles {
        if let Some(stream_index) = subtitle_stream_index {
            // Use the specified subtitle stream index
            println!("Using specified subtitle stream index: {}", stream_index);
            if let Ok(()) = extract_subtitle_stream(
                video_path,
                stream_index as usize,
                Path::new(&cfg.ffmpeg_path),
            )
            .await
            {
                // Get stream info for metadata
                let streams = get_subtitle_streams(video_path, Path::new(&cfg.ffprobe_path))
                    .await
                    .unwrap_or_else(|_| Vec::new());
                let stream_info = streams.iter().find(|s| s.index == stream_index as usize);
                let source_info = match stream_info {
                    Some(stream) => format!(
                        "subtitles: {} ({})",
                        stream.language_display(),
                        stream_index
                    ),
                    None => format!("subtitles: Unknown ({})", stream_index),
                };
                add_key_to_metadata_block(video_path, "source", &source_info)?;
                println!("Created transcript file: {}", txt_path.display());
                return Ok(true);
            } else {
                println!(
                    "Failed to extract specified subtitle stream, trying whisper transcription"
                );
            }
        } else {
            // Auto-detect subtitle streams
            let streams = get_subtitle_streams(video_path, Path::new(&cfg.ffprobe_path))
                .await
                .unwrap_or_else(|e| {
                    eprintln!("Failed to check for subtitle streams: {}", e);
                    Vec::new()
                });

            if !streams.is_empty() {
                println!("Found subtitle streams: {:?}", streams);
                if let Ok(()) = extract_subtitle_stream(
                    video_path,
                    streams[0].index,
                    Path::new(&cfg.ffmpeg_path),
                )
                .await
                {
                    let source_info = format!(
                        "subtitles: {} ({})",
                        streams[0].language_display(),
                        streams[0].index
                    );
                    add_key_to_metadata_block(video_path, "source", &source_info)?;
                    println!("Created transcript file: {}", txt_path.display());
                    return Ok(true);
                } else {
                    println!("Failed to extract subtitles, trying whisper transcription");
                }
            }
        }
    }

    // Check if we should cancel before proceeding with audio extraction
    if check_cancel_file() {
        cleanup_cancel_and_processing_files(video_path)?;
        return Ok(false);
    }

    // Check if whisper is allowed
    if !cfg.allow_whisper {
        fs::write(&txt_path, "")?;
        println!(
            "Whisper transcription disabled, created empty transcript file: {}",
            txt_path.display()
        );
        return Ok(true);
    }

    // Check for audio streams
    let has_audio = has_audio_stream(video_path, Path::new(&cfg.ffprobe_path))
        .await
        .unwrap_or(false);
    if !has_audio {
        fs::write(&txt_path, "")?;
        println!(
            "No audio streams found, created empty transcript file: {}",
            txt_path.display()
        );
        return Ok(true);
    }

    // Extract audio with cancellation check
    println!("Extracting audio");
    let audio_path = video_path.with_extension("mp3");

    // Start audio extraction in background
    let mut child = Command::new(&cfg.ffmpeg_path)
        .args([
            "-i",
            video_path.to_str().unwrap(),
            "-map",
            "0:a:0",
            "-q:a",
            "0",
            "-ac",
            "1",
            "-ar",
            "16000",
            "-y",
            audio_path.to_str().unwrap(),
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
    let model_name = model.as_ref().unwrap_or(&cfg.model_name);
    let model_path = home_dir
        .join(".atci/models")
        .join(format!("{}.bin", model_name));

    let mut child = Command::new(&cfg.whispercli_path)
        .args([
            "-m",
            model_path.to_str().unwrap(),
            "-np",
            "--max-context",
            "0",
            "-ovtt",
            "-f",
            audio_path.to_str().unwrap(),
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

        add_key_to_metadata_block(video_path, "source", model_name)?;
        println!("Successfully created transcript: {}", txt_path.display());
    }

    Ok(true)
}

#[get("/api/video/subtitle-streams?<path>")]
pub async fn web_get_subtitle_streams(
    path: &str,
) -> Result<
    Json<crate::web::ApiResponse<Vec<SubtitleStream>>>,
    BadRequest<Json<crate::web::ApiResponse<Vec<SubtitleStream>>>>,
> {
    let video_path = std::path::Path::new(path);

    if !video_path.exists() {
        return Err(BadRequest(Json(crate::web::ApiResponse::error(format!(
            "Video file not found: {}",
            path
        )))));
    }

    let cfg = match crate::config::load_config() {
        Ok(config) => config,
        Err(e) => {
            return Err(BadRequest(Json(crate::web::ApiResponse::error(format!(
                "Failed to load config: {}",
                e
            )))));
        }
    };

    match get_subtitle_streams(video_path, std::path::Path::new(&cfg.ffprobe_path)).await {
        Ok(streams) => Ok(Json(crate::web::ApiResponse::success(streams))),
        Err(e) => Err(BadRequest(Json(crate::web::ApiResponse::error(format!(
            "Failed to get subtitle streams: {}",
            e
        ))))),
    }
}

pub async fn cancellable_add_length_to_metadata(
    video_path: &Path,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    if check_cancel_file() {
        cleanup_cancel_and_processing_files(video_path)?;
        return Ok(false);
    }

    // Check if this is a video part file - if so, handle specially
    if let Some(video_part) = crate::video_parts::parse_video_part(video_path) {
        if video_part.part_number != 1 {
            // For parts other than part 1, skip metadata addition
            return Ok(true);
        }
        // For part 1, try to use the master file if it exists
        let (master_video_path, _) = crate::video_parts::get_master_paths(&video_part);
        let master_path = Path::new(&master_video_path);
        if master_path.exists() {
            // Recursively call with master path, but use Box::pin to avoid recursion issues
            return Box::pin(add_length_metadata_to_path(master_path)).await;
        }
    }

    // Add metadata to the current path
    add_length_metadata_to_path(video_path).await
}

async fn add_length_metadata_to_path(
    target_path: &Path,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let cfg: crate::AtciConfig = crate::config::load_config()?;

    let output = Command::new(&cfg.ffprobe_path)
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            target_path.to_str().unwrap(),
        ])
        .output()
        .await?;

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

        add_key_to_metadata_block(target_path, "length", &formatted)?;
        println!(
            "Created or updated meta file: {}",
            target_path.with_extension("meta").display()
        );
        Ok(true)
    } else {
        Err(format!("Failed to parse duration: {}", duration_str).into())
    }
}

/// Parse duration string (HH:MM:SS) to milliseconds
fn parse_duration_to_ms(
    duration_str: &str,
) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
    let parts: Vec<&str> = duration_str.split(':').collect();
    if parts.len() != 3 {
        return Err("Invalid duration format".into());
    }

    let hours: u64 = parts[0].parse()?;
    let minutes: u64 = parts[1].parse()?;
    let seconds: f64 = parts[2].parse()?;

    let total_ms = (hours * 3600 + minutes * 60) * 1000 + (seconds * 1000.0) as u64;
    Ok(total_ms)
}

/// Adjust timestamps in transcript by adding an offset
fn adjust_transcript_timestamps(
    transcript: &str,
    offset_ms: u64,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    use regex::Regex;

    let timestamp_regex =
        Regex::new(r"(\d{2}:\d{2}:\d{2})\.(\d{3}) --> (\d{2}:\d{2}:\d{2})\.(\d{3})")?;

    let adjusted = timestamp_regex.replace_all(transcript, |caps: &regex::Captures| {
        let start_time = &caps[1];
        let start_ms = &caps[2];
        let end_time = &caps[3];
        let end_ms = &caps[4];

        // Parse start timestamp
        if let Ok(start_total_ms) = parse_timestamp_to_ms(start_time, start_ms)
            && let Ok(end_total_ms) = parse_timestamp_to_ms(end_time, end_ms)
        {
            let adjusted_start = start_total_ms + offset_ms;
            let adjusted_end = end_total_ms + offset_ms;

            let adjusted_start_str = format_ms_to_timestamp(adjusted_start);
            let adjusted_end_str = format_ms_to_timestamp(adjusted_end);

            return format!("{} --> {}", adjusted_start_str, adjusted_end_str);
        }

        // Fallback: return original if parsing fails
        caps.get(0).unwrap().as_str().to_string()
    });

    Ok(adjusted.to_string())
}

/// Parse timestamp (HH:MM:SS.mmm) to milliseconds
fn parse_timestamp_to_ms(
    time_str: &str,
    ms_str: &str,
) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
    let time_parts: Vec<&str> = time_str.split(':').collect();
    if time_parts.len() != 3 {
        return Err("Invalid time format".into());
    }

    let hours: u64 = time_parts[0].parse()?;
    let minutes: u64 = time_parts[1].parse()?;
    let seconds: u64 = time_parts[2].parse()?;
    let milliseconds: u64 = ms_str.parse()?;

    let total_ms = (hours * 3600 + minutes * 60 + seconds) * 1000 + milliseconds;
    Ok(total_ms)
}

/// Format milliseconds back to timestamp string (HH:MM:SS.mmm)
fn format_ms_to_timestamp(total_ms: u64) -> String {
    let hours = total_ms / (3600 * 1000);
    let minutes = (total_ms % (3600 * 1000)) / (60 * 1000);
    let seconds = (total_ms % (60 * 1000)) / 1000;
    let milliseconds = total_ms % 1000;

    format!(
        "{:02}:{:02}:{:02}.{:03}",
        hours, minutes, seconds, milliseconds
    )
}

/// Update or create the master video file by appending the current part
async fn update_master_video(
    video_part: &crate::video_parts::VideoPart,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cfg = crate::config::load_config()?;
    let (master_video_path, _) = crate::video_parts::get_master_paths(video_part);
    let parent_dir = Path::new(&video_part.video_path).parent().unwrap();
    let current_part_path = Path::new(&video_part.video_path);
    let master_path = Path::new(&master_video_path);

    // Check if current part file exists
    if !current_part_path.exists() {
        println!(
            "Current part file doesn't exist: {}",
            current_part_path.display()
        );
        return Ok(());
    }

    if master_path.exists() {
        // Master file exists - append current part to it
        println!(
            "Appending part {} to existing master video: {}",
            video_part.part_number, master_video_path
        );

        // Create concat file for master + current part
        let concat_file_path = parent_dir.join(format!("{}_append.txt", video_part.base_name));
        let concat_content = format!(
            "file '{}'\nfile '{}'",
            master_path.file_name().unwrap().to_string_lossy(),
            current_part_path.file_name().unwrap().to_string_lossy()
        );
        std::fs::write(&concat_file_path, concat_content)?;

        // Create temporary output file
        let temp_master_path = parent_dir.join(format!(
            "{}_temp.{}",
            video_part.base_name, video_part.extension
        ));

        // Use FFmpeg to append current part to master
        let output = tokio::process::Command::new(&cfg.ffmpeg_path)
            .args([
                "-f",
                "concat",
                "-safe",
                "0",
                "-i",
                concat_file_path.to_str().unwrap(),
                "-c",
                "copy",
                "-y",
                temp_master_path.to_str().unwrap(),
            ])
            .current_dir(parent_dir)
            .output()
            .await?;

        // Clean up concat file
        let _ = std::fs::remove_file(&concat_file_path);

        if !output.status.success() {
            let error_output = String::from_utf8_lossy(&output.stderr);
            let _ = std::fs::remove_file(&temp_master_path);
            return Err(format!("FFmpeg append failed: {}", error_output).into());
        }

        // Replace master with temp file
        std::fs::rename(&temp_master_path, master_path)?;
        println!(
            "Successfully appended part {} to master video",
            video_part.part_number
        );
    } else {
        // No master file exists - rename current part to become the master
        println!(
            "Creating master video from part {}: {}",
            video_part.part_number, master_video_path
        );
        std::fs::rename(current_part_path, master_path)?;
        println!(
            "Successfully created master video from part {}",
            video_part.part_number
        );
        return Ok(()); // Don't delete the part file since we renamed it
    }

    // Clean up the current part file after successful append
    let _ = std::fs::remove_file(current_part_path);
    println!("Cleaned up part file: {}", current_part_path.display());

    Ok(())
}
