use std::path::Path;
use std::process::Command;

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
    start: f64,
    end: f64,
    text: Option<&str>,
    display_text: bool,
    format: &str,
    font_size: Option<u32>,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_video_file(path)?;
    
    if end <= start {
        return Err("End time must be greater than start time".into());
    }
    let cfg: crate::AtciConfig = crate::config::load_config()?;
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

    let start_time_str = format!("{:.1}", start);
    let end_time_str = format!("{:.1}", end);
    let format_param = format;
    let font_size_part = font_size.map(|fs| format!("_fs{}", fs)).unwrap_or_default();
    
    let temp_clip_name = format!("clip_{}_{}{}_{}.{}", start_time_str, end_time_str, caption_part, font_size_part, format_param);
    let temp_clip_path = std::env::temp_dir().join(&temp_clip_name);
    
    if temp_clip_path.exists() {
        println!("{}", temp_clip_path.display());
        return Ok(());
    }

    let duration = (end - start) + 0.1;

    let video_args = match format {
        "mp4" => {
            let audio_codec_args = get_audio_codec_args(path, Path::new(&cfg.ffprobe_path))?;
            if display_text && text.is_some() {
                video_with_text_args(path, start, duration, text.expect("text was missing"), &temp_clip_path, &audio_codec_args, font_size)
            } else {
                video_no_text_args(path, start, duration, &temp_clip_path, &audio_codec_args)
            }
        },
        "gif" => {
            if display_text && text.is_some() {
                gif_with_text_args(path, start, duration, text.expect("text was missing"), &temp_clip_path, font_size)
            } else {
                gif_no_text_args(path, start, duration, &temp_clip_path)
            }
        },
        "mp3" => {
            audio_file_args(path, start, duration, &temp_clip_path)
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

