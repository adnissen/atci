// atci (andrew's transcript and clipping interface)
// Copyright (C) 2025 Andrew Nissen

use crate::Asset;
use crate::auth::AuthGuard;
use rocket::serde::Deserialize;
use rocket::{get, response::status};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use std::process::Command;

fn get_video_extensions() -> Vec<&'static str> {
    vec![
        "mp4", "avi", "mov", "mkv", "wmv", "flv", "webm", "m4v", "ts",
    ]
}

fn get_font_path() -> Result<String, Box<dyn std::error::Error>> {
    use uuid::Uuid;

    // Extract font file from embedded assets
    if let Some(font_data) = Asset::get("SourceSans3-BoldItalic.ttf") {
        let temp_font_name = format!("font_{}.ttf", Uuid::new_v4());
        let temp_font_path = std::env::temp_dir().join(&temp_font_name);

        std::fs::write(&temp_font_path, font_data.data.as_ref())?;
        Ok(temp_font_path.to_string_lossy().to_string())
    } else {
        Err("Font file not found in embedded assets".into())
    }
}

fn validate_video_file(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if !path.exists() {
        return Err(format!("File does not exist: {}", path.display()).into());
    }

    if !path.is_file() {
        return Err(format!("Path is not a file: {}", path.display()).into());
    }

    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase());

    if let Some(ext) = extension {
        let video_extensions = get_video_extensions();
        if !video_extensions.contains(&ext.as_str()) {
            return Err(format!("Unsupported video format: {}", ext).into());
        }
    } else {
        return Err("File has no extension or invalid extension".into());
    }

    Ok(())
}

pub fn grab_frame(
    path: &Path,
    time: &str,
    text: Option<&str>,
    font_size: Option<u32>,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let cfg: crate::AtciConfig = crate::config::load_config()?;
    let ffprobe_path = Path::new(&cfg.ffprobe_path);

    // Parse time format
    let time_format = TimeFormat::parse(time)?;

    // Convert to seconds
    let time_seconds = time_format.to_seconds(path, ffprobe_path)?;

    validate_video_file(path)?;

    // Create a static filename with time and caption
    let caption_part = match text {
        Some(text_content) => {
            // Sanitize text for filename - remove/replace problematic characters
            let sanitized_text = text_content
                .chars()
                .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '-')
                .collect::<String>()
                .split_whitespace()
                .collect::<Vec<&str>>()
                .join("_")
                .chars()
                .take(50)
                .collect::<String>();

            format!("_{}", sanitized_text)
        }
        None => String::new(),
    };

    let time_str = format!("{:.3}", time_seconds);
    let font_size_part = font_size.map(|fs| format!("_fs{}", fs)).unwrap_or_default();

    let temp_frame_name = format!("frame_{}{}_{}.png", time_str, caption_part, font_size_part);
    let temp_frame_path = std::env::temp_dir().join(&temp_frame_name);

    if temp_frame_path.exists() {
        return Ok(temp_frame_path);
    }

    let frame_args = grab_frame_args(path, time_seconds, text, &temp_frame_path, font_size);

    let mut cmd = Command::new(&cfg.ffmpeg_path);
    cmd.args(&frame_args);

    let output = cmd.output()?;

    if output.status.success() {
        Ok(temp_frame_path)
    } else {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        Err(format!("Error creating frame with ffmpeg: {}", error_msg).into())
    }
}

pub fn clip(
    path: &Path,
    start: &str,
    end: &str,
    text: Option<&str>,
    display_text: bool,
    format: &str,
    font_size: Option<u32>,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let cfg: crate::AtciConfig = crate::config::load_config()?;
    let ffprobe_path = Path::new(&cfg.ffprobe_path);

    // Parse time formats
    let start_time = TimeFormat::parse(start)?;
    let end_time = TimeFormat::parse(end)?;

    // Convert to seconds
    let start_seconds = start_time.to_seconds(path, ffprobe_path)?;
    let end_seconds = end_time.to_seconds(path, ffprobe_path)?;

    validate_video_file(path)?;

    if end_seconds <= start_seconds {
        return Err("End time must be greater than start time".into());
    }

    // Create a static filename using SHA256 hash of all attributes
    let caption_part = match display_text || text.is_some() {
        false => String::new(),
        true => {
            if let Some(text_content) = text {
                text_content.to_string()
            } else {
                String::new()
            }
        }
    };

    let start_time_str = format!("{:.1}", start_seconds);
    let end_time_str = format!("{:.1}", end_seconds);
    let format_param = format;
    let font_size_part = font_size.map(|fs| format!("fs{}", fs)).unwrap_or_default();

    // Combine all attributes into a single string for hashing
    let combined_attributes = format!(
        "clip_{}_{}_{}_{}_{}.{}",
        start_time_str, end_time_str, caption_part, font_size_part, format_param, display_text
    );

    // Generate SHA256 hash
    let mut hasher = Sha256::new();
    hasher.update(combined_attributes.as_bytes());
    let hash = format!("{:x}", hasher.finalize());

    let temp_clip_name = format!("clip_{}.{}", hash, format_param);
    let temp_clip_path = std::env::temp_dir().join(&temp_clip_name);

    if temp_clip_path.exists() {
        return Ok(temp_clip_path);
    }

    let duration = (end_seconds - start_seconds) + 0.1;

    let video_args = match format {
        "mp4" => {
            let audio_codec_args = get_audio_codec_args(path, Path::new(&cfg.ffprobe_path))?;
            if let (true, Some(text_content)) = (display_text, text) {
                video_with_text_args(
                    path,
                    start_seconds,
                    duration,
                    text_content,
                    &temp_clip_path,
                    &audio_codec_args,
                    font_size,
                )
            } else {
                video_no_text_args(
                    path,
                    start_seconds,
                    duration,
                    &temp_clip_path,
                    &audio_codec_args,
                )
            }
        }
        "gif" => {
            if let (true, Some(text_content)) = (display_text, text) {
                gif_with_text_args(
                    path,
                    start_seconds,
                    duration,
                    text_content,
                    &temp_clip_path,
                    font_size,
                )
            } else {
                gif_no_text_args(path, start_seconds, duration, &temp_clip_path)
            }
        }
        "mp3" => audio_file_args(path, start_seconds, duration, &temp_clip_path),
        _ => {
            return Err(format!("Unsupported format: {}", format).into());
        }
    };

    let mut cmd = Command::new(&cfg.ffmpeg_path);
    cmd.args(&video_args);

    let output = cmd.output()?;

    if output.status.success() {
        Ok(temp_clip_path)
    } else {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        Err(format!("Error creating {} clip with ffmpeg: {}", format, error_msg).into())
    }
}

#[derive(Debug, Clone)]
pub enum TimeFormat {
    Seconds(f64),
    Frames(u32),
    Timestamp(String),
}

impl TimeFormat {
    pub fn parse(input: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // Try to parse as timestamp (HH:MM:SS.sss or MM:SS.sss)
        if input.contains(':') {
            return Ok(TimeFormat::Timestamp(input.to_string()));
        }

        // Try to parse as frame number (ends with 'f')
        if let Some(frame_str) = input.strip_suffix('f') {
            let frames = frame_str
                .parse::<u32>()
                .map_err(|_| "Invalid frame number format")?;
            return Ok(TimeFormat::Frames(frames));
        }

        // Try to parse as seconds (default)
        let seconds = input.parse::<f64>().map_err(|_| "Invalid time format")?;
        Ok(TimeFormat::Seconds(seconds))
    }

    pub fn to_seconds(
        &self,
        video_path: &Path,
        ffprobe_path: &Path,
    ) -> Result<f64, Box<dyn std::error::Error>> {
        match self {
            TimeFormat::Seconds(s) => Ok(*s),
            TimeFormat::Frames(f) => {
                let fps = get_video_fps(video_path, ffprobe_path)?;
                Ok(*f as f64 / fps)
            }
            TimeFormat::Timestamp(ts) => parse_timestamp_to_seconds(ts),
        }
    }
}

fn get_video_dimensions(
    video_path: &Path,
    ffprobe_path: &Path,
) -> Result<(u32, u32), Box<dyn std::error::Error>> {
    let output = Command::new(ffprobe_path)
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height",
            "-of",
            "csv=p=0",
        ])
        .arg(video_path)
        .output()?;

    if output.status.success() {
        let dimensions_str = String::from_utf8(output.stdout)?.trim().to_string();

        let parts: Vec<&str> = dimensions_str.split(',').collect();
        if parts.len() == 2 {
            let width: u32 = parts[0].parse()?;
            let height: u32 = parts[1].parse()?;
            return Ok((width, height));
        }
    }

    // Default to 1920x1080 if detection fails
    Ok((1920, 1080))
}

fn get_video_fps(
    video_path: &Path,
    ffprobe_path: &Path,
) -> Result<f64, Box<dyn std::error::Error>> {
    let output = Command::new(ffprobe_path)
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=r_frame_rate",
            "-of",
            "csv=p=0",
        ])
        .arg(video_path)
        .output()?;

    if output.status.success() {
        let fps_str = String::from_utf8(output.stdout)?
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .to_string();

        // Parse fraction format like "30/1" or "2997/100"
        if fps_str.contains('/') {
            let parts: Vec<&str> = fps_str.trim().split('/').collect();
            if parts.len() == 2 {
                let numerator: f64 = parts[0].parse::<f64>()?;
                let denominator: f64 = parts[1].parse::<f64>()?;
                return Ok(numerator / denominator);
            }
        }
        // Fallback to direct parsing
        fps_str
            .parse::<f64>()
            .map_err(|_| format!("Invalid frame rate format: {}", fps_str).into())
    } else {
        // Default to 30fps if detection fails
        Ok(30.0)
    }
}

fn parse_timestamp_to_seconds(timestamp: &str) -> Result<f64, Box<dyn std::error::Error>> {
    let parts: Vec<&str> = timestamp.split(':').collect();

    match parts.len() {
        2 => {
            // MM:SS.sss format
            let minutes: f64 = parts[0].parse()?;
            let seconds: f64 = parts[1].parse()?;
            Ok(minutes * 60.0 + seconds)
        }
        3 => {
            // HH:MM:SS.sss format
            let hours: f64 = parts[0].parse()?;
            let minutes: f64 = parts[1].parse()?;
            let seconds: f64 = parts[2].parse()?;
            Ok(hours * 3600.0 + minutes * 60.0 + seconds)
        }
        _ => Err("Invalid timestamp format. Use MM:SS.sss or HH:MM:SS.sss".into()),
    }
}

fn gif_with_text_args(
    input_path: &Path,
    start: f64,
    duration: f64,
    text: &str,
    output_path: &Path,
    font_size: Option<u32>,
) -> Vec<String> {
    use std::fs;
    use uuid::Uuid;

    let temp_text_name = format!("text_{}.txt", Uuid::new_v4());
    let temp_text_path = std::env::temp_dir().join(&temp_text_name);

    match fs::write(&temp_text_path, text) {
        Ok(_) => {
            let cfg = crate::config::load_config().unwrap_or_default();
            let ffprobe_path = Path::new(&cfg.ffprobe_path);
            let (width, _) = get_video_dimensions(input_path, ffprobe_path).unwrap_or((1920, 1080));
            let font_size =
                font_size.unwrap_or_else(|| calculate_font_size_for_video(width, text.len()));
            let font_path =
                get_font_path().unwrap_or_else(|_| "/System/Library/Fonts/Arial.ttf".to_string());
            vec![
                "-ss",
                &format!("{}", start),
                "-t",
                &format!("{}", duration),
                "-i",
                &input_path.to_string_lossy(),
                "-vf",
                &format!("drawtext=textfile='{}':fontcolor=white:fontsize={}:fontfile='{}':x=(w-text_w)/2:y=h-th-10,fps=10,scale=480:-1:flags=lanczos,split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse", 
                       temp_text_path.to_string_lossy(), font_size, font_path),
                "-loop",
                "0",
                "-y",
                &output_path.to_string_lossy(),
            ]
            .into_iter()
            .map(|s| s.to_string())
            .collect()
        }
        Err(_) => vec![
            "-ss",
            &format!("{}", start),
            "-t",
            &format!("{}", duration),
            "-i",
            &input_path.to_string_lossy(),
            "-vf",
            "fps=10,scale=480:-1:flags=lanczos,split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse",
            "-loop",
            "0",
            "-y",
            &output_path.to_string_lossy(),
        ]
        .into_iter()
        .map(|s| s.to_string())
        .collect(),
    }
}

fn gif_no_text_args(
    input_path: &Path,
    start: f64,
    duration: f64,
    output_path: &Path,
) -> Vec<String> {
    vec![
        "-ss",
        &format!("{}", start),
        "-t",
        &format!("{}", duration),
        "-i",
        &input_path.to_string_lossy(),
        "-vf",
        "fps=10,scale=480:-1:flags=lanczos,split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse",
        "-loop",
        "0",
        "-y",
        &output_path.to_string_lossy(),
    ]
    .into_iter()
    .map(|s| s.to_string())
    .collect()
}

fn video_with_text_args(
    input_path: &Path,
    start: f64,
    duration: f64,
    text: &str,
    output_path: &Path,
    audio_codec_args: &[&str],
    font_size: Option<u32>,
) -> Vec<String> {
    use std::fs;
    use uuid::Uuid;

    let temp_text_name = format!("text_{}.txt", Uuid::new_v4());
    let temp_text_path = std::env::temp_dir().join(&temp_text_name);

    match fs::write(&temp_text_path, text) {
        Ok(_) => {
            let cfg = crate::config::load_config().unwrap_or_default();
            let ffprobe_path = Path::new(&cfg.ffprobe_path);
            let (width, _) = get_video_dimensions(input_path, ffprobe_path).unwrap_or((1920, 1080));
            let font_size =
                font_size.unwrap_or_else(|| calculate_font_size_for_video(width, text.len()));
            let font_path =
                get_font_path().unwrap_or_else(|_| "/System/Library/Fonts/Arial.ttf".to_string());
            let fps = get_video_fps(input_path, ffprobe_path).unwrap_or(30.0);
            let frames_count = (duration * fps).trunc() as i32;

            let mut args = vec![
                "-ss",
                &format!("{}", start),
                "-i",
                &input_path.to_string_lossy(),
                "-ss",
                "00:00:00.001",
                "-t",
                &format!("{}", duration),
                "-vf",
                &format!("drawtext=textfile='{}':fontcolor=white:fontsize={}:fontfile='{}':x=(w-text_w)/2:y=h-th-10", 
                       temp_text_path.to_string_lossy(), font_size, font_path),
                "-frames:v",
                &frames_count.to_string(),
                "-c:v",
                "libx264",
                "-profile:v",
                "baseline",
                "-level",
                "3.1",
                "-pix_fmt",
                "yuv420p",
            ]
            .into_iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();

            args.extend(audio_codec_args.iter().map(|s| (*s).to_string()));

            args.extend(
                vec![
                    "-crf",
                    "28",
                    "-preset",
                    "ultrafast",
                    "-movflags",
                    "faststart+frag_keyframe+empty_moov",
                    "-avoid_negative_ts",
                    "make_zero",
                    "-y",
                    "-map_chapters",
                    "-1",
                    &output_path.to_string_lossy(),
                ]
                .into_iter()
                .map(|s| s.to_string()),
            );

            args
        }
        Err(_) => {
            let mut args = vec![
                "-ss",
                &format!("{}", start),
                "-t",
                &format!("{}", duration),
                "-i",
                &input_path.to_string_lossy(),
                "-c:v",
                "libx264",
                "-profile:v",
                "baseline",
                "-level",
                "3.1",
                "-pix_fmt",
                "yuv420p",
            ]
            .into_iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();

            args.extend(audio_codec_args.iter().map(|s| (*s).to_string()));

            args.extend(
                vec![
                    "-crf",
                    "28",
                    "-preset",
                    "ultrafast",
                    "-movflags",
                    "faststart+frag_keyframe+empty_moov",
                    "-avoid_negative_ts",
                    "make_zero",
                    "-y",
                    "-map_chapters",
                    "-1",
                    &output_path.to_string_lossy(),
                ]
                .into_iter()
                .map(|s| s.to_string()),
            );

            args
        }
    }
}

fn video_no_text_args(
    input_path: &Path,
    start: f64,
    duration: f64,
    output_path: &Path,
    audio_codec_args: &[&str],
) -> Vec<String> {
    let cfg = crate::config::load_config().unwrap_or_default();
    let ffprobe_path = Path::new(&cfg.ffprobe_path);
    let fps = get_video_fps(input_path, ffprobe_path).unwrap_or(30.0);
    let frames_count = (duration * fps).trunc() as i32;

    let mut args = vec![
        "-ss",
        &format!("{}", start),
        "-i",
        &input_path.to_string_lossy(),
        "-ss",
        "00:00:00.001",
        "-t",
        &format!("{}", duration),
        "-frames:v",
        &frames_count.to_string(),
        "-c:v",
        "libx264",
        "-profile:v",
        "baseline",
        "-level",
        "3.1",
        "-pix_fmt",
        "yuv420p",
    ]
    .into_iter()
    .map(|s| s.to_string())
    .collect::<Vec<String>>();

    args.extend(audio_codec_args.iter().map(|s| (*s).to_string()));

    args.extend(
        vec![
            "-crf",
            "28",
            "-preset",
            "ultrafast",
            "-movflags",
            "faststart+frag_keyframe+empty_moov",
            "-avoid_negative_ts",
            "make_zero",
            "-y",
            "-map_chapters",
            "-1",
            &output_path.to_string_lossy(),
        ]
        .into_iter()
        .map(|s| s.to_string()),
    );

    args
}

fn audio_file_args(
    input_path: &Path,
    start: f64,
    duration: f64,
    output_path: &Path,
) -> Vec<String> {
    vec![
        "-ss",
        &format!("{}", start),
        "-t",
        &format!("{}", duration),
        "-i",
        &input_path.to_string_lossy(),
        "-vn",
        "-acodec",
        "libmp3lame",
        "-ar",
        "44100",
        "-ac",
        "2",
        "-b:a",
        "256k",
        "-y",
        &output_path.to_string_lossy(),
    ]
    .into_iter()
    .map(|s| s.to_string())
    .collect()
}

fn grab_frame_args(
    input_path: &Path,
    time: f64,
    text: Option<&str>,
    output_path: &Path,
    font_size: Option<u32>,
) -> Vec<String> {
    let mut args = vec![
        "-ss".to_string(),
        format!("{}", time),
        "-i".to_string(),
        input_path.to_string_lossy().to_string(),
        "-vframes".to_string(),
        "1".to_string(),
    ];

    if let Some(text_content) = text {
        use std::fs;
        use uuid::Uuid;

        let temp_text_name = format!("text_{}.txt", Uuid::new_v4());
        let temp_text_path = std::env::temp_dir().join(&temp_text_name);

        match fs::write(&temp_text_path, text_content) {
            Ok(_) => {
                let cfg = crate::config::load_config().unwrap_or_default();
                let ffprobe_path = Path::new(&cfg.ffprobe_path);
                let (width, _) =
                    get_video_dimensions(input_path, ffprobe_path).unwrap_or((1920, 1080));
                let font_size = font_size
                    .unwrap_or_else(|| calculate_font_size_for_video(width, text_content.len()));
                let font_path = get_font_path()
                    .unwrap_or_else(|_| "/System/Library/Fonts/Arial.ttf".to_string());

                args.extend(vec![
                    "-vf".to_string(),
                    format!(
                        "drawtext=textfile='{}':fontcolor=white:fontsize={}:fontfile='{}':x=(w-text_w)/2:y=h-th-10",
                        temp_text_path.to_string_lossy(),
                        font_size,
                        font_path
                    ),
                ]);
            }
            Err(_) => {
                // If we can't write the text file, just grab the frame without text
            }
        }
    }

    args.extend(vec![
        "-q:v".to_string(),
        "1".to_string(), // Highest quality for PNG
        "-pix_fmt".to_string(),
        "rgba".to_string(), // Support transparency
        "-y".to_string(),
        output_path.to_string_lossy().to_string(),
    ]);

    args
}

fn calculate_font_size_for_video(horizontal_size: u32, text_length: usize) -> u32 {
    // Base font size proportional to video width (roughly 5% of width)
    let base_size = (horizontal_size as f32 * 0.05) as u32;

    // Reduce font size progressively per character after a threshold
    let threshold = 40; // Start reducing after 20 characters
    if text_length > threshold {
        let excess_chars = text_length - threshold;
        // Reduce by 1% per excess character, with a minimum size of 25% of base
        let reduction_percentage = (excess_chars as f32 * 0.01).min(0.75);
        let reduced_size = base_size as f32 * (1.0 - reduction_percentage);
        (reduced_size as u32).max(base_size / 4)
    } else {
        base_size
    }
}

pub fn calculate_font_size_for_video_path(video_path: &Path, text_length: usize) -> u32 {
    let cfg = crate::config::load_config().unwrap_or_default();
    let ffprobe_path = Path::new(&cfg.ffprobe_path);
    let (width, _) = get_video_dimensions(video_path, ffprobe_path).unwrap_or((1920, 1080));
    calculate_font_size_for_video(width, text_length)
}

fn get_audio_codec_args(
    path: &Path,
    ffprobe_path: &Path,
) -> Result<Vec<&'static str>, Box<dyn std::error::Error>> {
    // some files need more processing than others. for example, _all_ webm files need "basic" re-encoding.
    // then, additionally, some files need even more advanced processing if their "layout" is not stereo or mono
    let source_extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase())
        .unwrap_or_default();

    let extension_needs_basic_audio_reencoding =
        matches!(source_extension.as_str(), "mkv" | "webm" | "avi" | "mov");

    let (needs_advanced_audio_reencoding, layout) =
        check_if_advanced_audio_reencoding_needed(path, ffprobe_path)?;

    let audio_codec_args = if extension_needs_basic_audio_reencoding {
        if needs_advanced_audio_reencoding {
            // Use appropriate channel mapping based on detected layout
            let channel_filter = match layout.as_str() {
                "5.1" => {
                    // 5.1 layout: FL FR FC LFE BL BR (back channels) -> keep as 5.1
                    "channelmap=FL-FL|FR-FR|FC-FC|LFE-LFE|BL-BL|BR-BR:5.1"
                }
                "5.1(side)" => {
                    // 5.1(side) layout: FL FR FC LFE SL SR (side channels) -> map to 5.1 back channels
                    "channelmap=FL-FL|FR-FR|FC-FC|LFE-LFE|SL-BL|SR-BR:5.1"
                }
                layout if ["7.1", "7.1(wide)", "7.1(wide-side)"].contains(&layout) => {
                    // 7.1 layout: FL FR FC LFE BL BR SL SR -> keep as 7.1
                    "channelmap=FL-FL|FR-FR|FC-FC|LFE-LFE|BL-BL|BR-BR|SL-SL|SR-SR:7.1"
                }
                _ => {
                    // For other layouts, downmix to stereo
                    "pan=stereo|FL=0.5*FL+0.707*FC+0.5*BL+0.5*SL|FR=0.5*FR+0.707*FC+0.5*BR+0.5*SR"
                }
            };

            vec!["-filter:a", channel_filter, "-c:a", "aac", "-b:a", "256k"]
        } else {
            vec!["-c:a", "aac", "-b:a", "256k"]
        }
    } else if source_extension == "ts" {
        // .ts files often have malformed AAC streams that need the aac_adtstoasc filter
        vec!["-c:a", "copy", "-bsf:a", "aac_adtstoasc"]
    } else {
        vec!["-c:a", "copy"]
    };

    Ok(audio_codec_args)
}

pub fn check_if_advanced_audio_reencoding_needed(
    video_path: &Path,
    ffprobe_path: &Path,
) -> Result<(bool, String), Box<dyn std::error::Error>> {
    let output = Command::new(ffprobe_path)
        .args([
            "-v",
            "error",
            "-select_streams",
            "a:0",
            "-show_entries",
            "stream=channel_layout",
            "-of",
            "csv=p=0",
        ])
        .arg(video_path)
        .output()?;

    if output.status.success() {
        let layout = String::from_utf8(output.stdout)?.trim().to_lowercase();
        let needs_reencoding = !matches!(layout.as_str(), "mono" | "stereo");
        Ok((needs_reencoding, layout))
    } else {
        Ok((false, "stereo".to_string()))
    }
}

#[derive(Deserialize, rocket::FromForm)]
pub struct ClipQuery {
    filename: String,
    start_time: String,
    end_time: String,
    text: Option<String>,
    display_text: Option<String>,
    font_size: Option<String>,
    format: Option<String>,
}

#[derive(Deserialize, rocket::FromForm)]
pub struct FrameQuery {
    filename: String,
    time: String,
    text: Option<String>,
    font_size: Option<String>,
}

#[get("/api/clip?<query..>")]
pub fn web_clip(
    _auth: AuthGuard,
    query: ClipQuery,
) -> Result<Vec<u8>, status::BadRequest<&'static str>> {
    // Check if the video file exists at the given path
    let video_path = Path::new(&query.filename);
    if !video_path.exists() {
        return Err(status::BadRequest("Video file not found"));
    }

    // Parse optional parameters
    let text = query.text.as_deref();
    let display_text = query.display_text.as_deref() == Some("true");
    let format = query.format.as_deref().unwrap_or("mp4");
    let font_size = query.font_size.as_deref().and_then(|s| s.parse().ok());

    // Call the clip function and get the output path
    match clip(
        video_path,
        &query.start_time,
        &query.end_time,
        text,
        display_text,
        format,
        font_size,
    ) {
        Ok(output_path) => fs::read(&output_path).map_err(|e| {
            eprintln!("Error reading generated clip: {}", e);
            status::BadRequest("Error reading generated clip")
        }),
        Err(e) => {
            eprintln!("Error creating clip: {}", e);
            Err(status::BadRequest("Error creating clip"))
        }
    }
}

#[get("/api/frame?<query..>")]
pub fn web_frame(
    _auth: AuthGuard,
    query: FrameQuery,
) -> Result<(rocket::http::ContentType, Vec<u8>), status::BadRequest<&'static str>> {
    // Check if the video file exists at the given path
    let video_path = Path::new(&query.filename);
    if !video_path.exists() {
        return Err(status::BadRequest("Video file not found"));
    }

    // Parse optional parameters
    let text = query.text.as_deref();
    let font_size = query.font_size.as_deref().and_then(|s| s.parse().ok());

    // Call the grab_frame function and get the output path
    match grab_frame(video_path, &query.time, text, font_size) {
        Ok(output_path) => fs::read(&output_path)
            .map(|data| (rocket::http::ContentType::PNG, data))
            .map_err(|_| status::BadRequest("Error reading generated frame")),
        Err(_) => Err(status::BadRequest("Error creating frame")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_format_parse_seconds() {
        let result = TimeFormat::parse("10.5").unwrap();
        match result {
            TimeFormat::Seconds(s) => assert_eq!(s, 10.5),
            _ => panic!("Expected seconds format"),
        }
    }

    #[test]
    fn test_time_format_parse_frames() {
        let result = TimeFormat::parse("300f").unwrap();
        match result {
            TimeFormat::Frames(f) => assert_eq!(f, 300),
            _ => panic!("Expected frames format"),
        }
    }

    #[test]
    fn test_time_format_parse_timestamp() {
        let result = TimeFormat::parse("01:30:15.5").unwrap();
        match result {
            TimeFormat::Timestamp(ts) => assert_eq!(ts, "01:30:15.5"),
            _ => panic!("Expected timestamp format"),
        }
    }

    #[test]
    fn test_parse_timestamp_to_seconds_mm_ss() {
        assert_eq!(parse_timestamp_to_seconds("02:30").unwrap(), 150.0);
        assert_eq!(parse_timestamp_to_seconds("01:30.5").unwrap(), 90.5);
    }

    #[test]
    fn test_parse_timestamp_to_seconds_hh_mm_ss() {
        assert_eq!(parse_timestamp_to_seconds("01:02:30").unwrap(), 3750.0);
        assert_eq!(parse_timestamp_to_seconds("00:01:30.5").unwrap(), 90.5);
    }

    #[test]
    fn test_parse_timestamp_invalid_format() {
        assert!(parse_timestamp_to_seconds("invalid").is_err());
        assert!(parse_timestamp_to_seconds("1:2:3:4").is_err());
    }

    #[test]
    fn test_time_format_parse_invalid_frame() {
        assert!(TimeFormat::parse("invalidf").is_err());
        assert!(TimeFormat::parse("f").is_err());
    }

    #[test]
    fn test_time_format_parse_invalid_seconds() {
        assert!(TimeFormat::parse("invalid").is_err());
        assert!(TimeFormat::parse("").is_err());
    }
}
