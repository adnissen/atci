use std::path::Path;
use std::process::Command;
use rocket::serde::Deserialize;
use rocket::{get, response::status};
use std::fs;
use crate::auth::AuthGuard;

fn get_video_extensions() -> Vec<&'static str> {
    vec!["mp4", "avi", "mov", "mkv", "wmv", "flv", "webm", "m4v"]
}

fn validate_video_file(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if !path.exists() {
        return Err(format!("File does not exist: {}", path.display()).into());
    }
    
    if !path.is_file() {
        return Err(format!("Path is not a file: {}", path.display()).into());
    }
    
    let extension = path.extension()
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

pub fn clip(
    path: &Path,
    start: &str,
    end: &str,
    text: Option<&str>,
    display_text: bool,
    format: &str,
    font_size: Option<u32>,
) -> Result<(), Box<dyn std::error::Error>> {
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

    // Create a static filename with start/end times and caption
    let caption_part = match display_text || text.is_some() {
        false => String::new(),
        true => {
            if let Some(text_content) = text {
                // Sanitize text for filename - remove/replace problematic characters
                let sanitized_text = text_content
                    // Remove special chars except word chars, spaces, hyphens
                    .chars()
                    .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '-')
                    .collect::<String>()
                    // Replace spaces with underscores
                    .split_whitespace()
                    .collect::<Vec<&str>>()
                    .join("_")
                    // Limit length
                    .chars()
                    .take(50)
                    .collect::<String>();
                
                format!("_{}", sanitized_text)
            } else {
                String::new()
            }
        }
    };

    let start_time_str = format!("{:.1}", start_seconds);
    let end_time_str = format!("{:.1}", end_seconds);
    let format_param = format;
    let font_size_part = font_size.map(|fs| format!("_fs{}", fs)).unwrap_or_default();
    
    let temp_clip_name = format!("clip_{}_{}{}_{}.{}", start_time_str, end_time_str, caption_part, font_size_part, format_param);
    let temp_clip_path = std::env::temp_dir().join(&temp_clip_name);
    
    if temp_clip_path.exists() {
        println!("{}", temp_clip_path.display());
        return Ok(());
    }

    let duration = (end_seconds - start_seconds) + 0.1;

    let video_args = match format {
        "mp4" => {
            let audio_codec_args = get_audio_codec_args(path, Path::new(&cfg.ffprobe_path))?;
            if display_text && text.is_some() {
                video_with_text_args(path, start_seconds, duration, text.expect("text was missing"), &temp_clip_path, &audio_codec_args, font_size)
            } else {
                video_no_text_args(path, start_seconds, duration, &temp_clip_path, &audio_codec_args)
            }
        },
        "gif" => {
            if display_text && text.is_some() {
                gif_with_text_args(path, start_seconds, duration, text.expect("text was missing"), &temp_clip_path, font_size)
            } else {
                gif_no_text_args(path, start_seconds, duration, &temp_clip_path)
            }
        },
        "mp3" => {
            audio_file_args(path, start_seconds, duration, &temp_clip_path)
        },
        _ => {
            return Err(format!("Unsupported format: {}", format).into());
        }
    };
    
    let mut cmd = Command::new(&cfg.ffmpeg_path);
    cmd.args(&video_args);
    
    let output = cmd.output()?;
    
    if output.status.success() {
        println!("{}", temp_clip_path.display());
        Ok(())
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
        if input.ends_with('f') {
            let frame_str = &input[..input.len()-1];
            let frames = frame_str.parse::<u32>()
                .map_err(|_| "Invalid frame number format")?;
            return Ok(TimeFormat::Frames(frames));
        }
        
        // Try to parse as seconds (default)
        let seconds = input.parse::<f64>()
            .map_err(|_| "Invalid time format")?;
        Ok(TimeFormat::Seconds(seconds))
    }
    
    pub fn to_seconds(&self, video_path: &Path, ffprobe_path: &Path) -> Result<f64, Box<dyn std::error::Error>> {
        match self {
            TimeFormat::Seconds(s) => Ok(*s),
            TimeFormat::Frames(f) => {
                let fps = get_video_fps(video_path, ffprobe_path)?;
                Ok(*f as f64 / fps)
            },
            TimeFormat::Timestamp(ts) => {
                parse_timestamp_to_seconds(ts)
            }
        }
    }
}

fn get_video_fps(video_path: &Path, ffprobe_path: &Path) -> Result<f64, Box<dyn std::error::Error>> {
    let output = Command::new(ffprobe_path)
        .args([
            "-v", "error",
            "-select_streams", "v:0",
            "-show_entries", "stream=r_frame_rate",
            "-of", "csv=p=0",
        ])
        .arg(video_path)
        .output()?;

    if output.status.success() {
        let fps_str = String::from_utf8(output.stdout)?
            .trim()
            .to_string();
        
        // Parse fraction format like "30/1" or "2997/100"
        if fps_str.contains('/') {
            let parts: Vec<&str> = fps_str.split('/').collect();
            if parts.len() == 2 {
                let numerator: f64 = parts[0].parse()?;
                let denominator: f64 = parts[1].parse()?;
                return Ok(numerator / denominator);
            }
        }
        
        // Fallback to direct parsing
        fps_str.parse::<f64>()
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
        },
        3 => {
            // HH:MM:SS.sss format
            let hours: f64 = parts[0].parse()?;
            let minutes: f64 = parts[1].parse()?;
            let seconds: f64 = parts[2].parse()?;
            Ok(hours * 3600.0 + minutes * 60.0 + seconds)
        },
        _ => Err("Invalid timestamp format. Use MM:SS.sss or HH:MM:SS.sss".into())
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
    use uuid::Uuid;
    use std::fs;
    
    let temp_text_name = format!("text_{}.txt", Uuid::new_v4());
    let temp_text_path = std::env::temp_dir().join(&temp_text_name);
    
    match fs::write(&temp_text_path, text) {
        Ok(_) => {
            let font_size = font_size.unwrap_or_else(|| calculate_font_size_for_video(text.len()));
            
            vec![
                "-ss",
                &format!("{}", start),
                "-t",
                &format!("{}", duration),
                "-i",
                &input_path.to_string_lossy(),
                "-vf",
                &format!("drawtext=textfile='{}':fontcolor=white:fontsize={}:fontfile='/Users/andrewnissen/MYRIADPRO-BOLDIT.OTF':x=(w-text_w)/2:y=h-th-10,fps=10,scale=480:-1:flags=lanczos,split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse", 
                       temp_text_path.to_string_lossy(), font_size),
                "-loop",
                "0",
                "-y",
                &output_path.to_string_lossy(),
            ]
            .into_iter()
            .map(|s| s.to_string())
            .collect()
        }
        Err(_) => {
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
        "fps=8,scale=320:-1:flags=fast_bilinear,split[s0][s1];[s0]palettegen=max_colors=128:stats_mode=single[p];[s1][p]paletteuse=dither=bayer:bayer_scale=2",
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
    use uuid::Uuid;
    use std::fs;
    
    let temp_text_name = format!("text_{}.txt", Uuid::new_v4());
    let temp_text_path = std::env::temp_dir().join(&temp_text_name);
    
    match fs::write(&temp_text_path, text) {
        Ok(_) => {
            let font_size = font_size.unwrap_or_else(|| calculate_font_size_for_video(text.len()));
            let frames_count = (duration * 30.0).trunc() as i32;
            
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
                &format!("drawtext=textfile='{}':fontcolor=white:fontsize={}:fontfile='/Users/andrewnissen/MYRIADPRO-BOLDIT.OTF':x=(w-text_w)/2:y=h-th-10", 
                       temp_text_path.to_string_lossy(), font_size),
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

            args.extend(vec![
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
            .map(|s| s.to_string()));

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

            args.extend(vec![
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
            .map(|s| s.to_string()));

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
    let frames_count = (duration * 30.0).trunc() as i32;
    
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

    args.extend(vec![
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
    .map(|s| s.to_string()));

    args
}

fn audio_file_args(
    input_path: &Path,
    start: f64,
    duration: f64,
    output_path: &Path
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

fn calculate_font_size_for_video(text_length: usize) -> u32 {
    // Simple heuristic: base font size adjusted by text length
    let base_size = 24;
    if text_length > 100 {
        base_size - 4
    } else if text_length > 50 {
        base_size - 2
    } else {
        base_size
    }
}

fn get_audio_codec_args(
    path: &Path,
    ffprobe_path: &Path,
) -> Result<Vec<&'static str>, Box<dyn std::error::Error>> {
    // some files need more processing than others. for example, _all_ webm files need "basic" re-encoding.
    // then, additionally, some files need even more advanced processing if their "layout" is not stereo or mono
    let source_extension = path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase())
        .unwrap_or_default();

    let extension_needs_basic_audio_reencoding = 
        matches!(source_extension.as_str(), "mkv" | "webm" | "avi" | "mov");

    let (needs_advanced_audio_reencoding, layout) = check_if_advanced_audio_reencoding_needed(path, ffprobe_path)?;
    
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
            "-v", "error",
            "-select_streams", "a:0",
            "-show_entries", "stream=channel_layout",
            "-of", "csv=p=0",
        ])
        .arg(video_path)
        .output()?;

    if output.status.success() {
        let layout = String::from_utf8(output.stdout)?
            .trim()
            .to_lowercase();
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


#[get("/api/clip?<query..>")]
pub fn web_clip(_auth: AuthGuard, query: ClipQuery) -> Result<Vec<u8>, status::BadRequest<&'static str>> {
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

    // Call the clip function that supports multiple time formats
    match clip(video_path, &query.start_time, &query.end_time, text, display_text, format, font_size) {
        Ok(()) => {
            // We need to parse the times to get the actual seconds for filename generation
            let cfg = match crate::config::load_config() {
                Ok(cfg) => cfg,
                Err(_) => return Err(status::BadRequest("Error loading config"))
            };
            let ffprobe_path = Path::new(&cfg.ffprobe_path);
            
            let (start_seconds, end_seconds) = match (
                TimeFormat::parse(&query.start_time).and_then(|t| t.to_seconds(video_path, ffprobe_path)),
                TimeFormat::parse(&query.end_time).and_then(|t| t.to_seconds(video_path, ffprobe_path))
            ) {
                (Ok(start), Ok(end)) => (start, end),
                _ => return Err(status::BadRequest("Error parsing time formats"))
            };
            
            // The clip function prints the output path, but we need to construct it ourselves
            // to read the file and return its contents
            let caption_part = match display_text || text.is_some() {
                false => String::new(),
                true => {
                    if let Some(text_content) = text {
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
                    } else {
                        String::new()
                    }
                }
            };

            let start_time_str = format!("{:.1}", start_seconds);
            let end_time_str = format!("{:.1}", end_seconds);
            let font_size_part = font_size.map(|fs| format!("_fs{}", fs)).unwrap_or_default();
            
            let temp_clip_name = format!("clip_{}_{}{}_{}.{}", start_time_str, end_time_str, caption_part, font_size_part, format);
            let temp_clip_path = std::env::temp_dir().join(&temp_clip_name);
            
            fs::read(&temp_clip_path)
                .map_err(|_| status::BadRequest("Error reading generated clip"))
        },
        Err(_) => Err(status::BadRequest("Error creating clip"))
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
            _ => panic!("Expected seconds format")
        }
    }

    #[test]
    fn test_time_format_parse_frames() {
        let result = TimeFormat::parse("300f").unwrap();
        match result {
            TimeFormat::Frames(f) => assert_eq!(f, 300),
            _ => panic!("Expected frames format")
        }
    }

    #[test]
    fn test_time_format_parse_timestamp() {
        let result = TimeFormat::parse("01:30:15.5").unwrap();
        match result {
            TimeFormat::Timestamp(ts) => assert_eq!(ts, "01:30:15.5"),
            _ => panic!("Expected timestamp format")
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

