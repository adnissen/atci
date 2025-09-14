use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

fn create_test_video_with_transcript(dir: &TempDir, video_name: &str, content: &str) -> String {
    let video_path = dir.path().join(format!("{}.mp4", video_name));
    let txt_path = dir.path().join(format!("{}.txt", video_name));

    // Create fake video file
    fs::write(&video_path, b"fake video content").unwrap();
    // Create transcript file
    fs::write(&txt_path, content).unwrap();

    video_path.to_string_lossy().to_string()
}

#[test]
fn test_search_with_regular_apostrophe() {
    let temp_dir = TempDir::new().unwrap();
    let content = "you can't read it\nthis is another line\nand one more line";
    let _video_path = create_test_video_with_transcript(&temp_dir, "test_video", content);

    // Set up config for search
    let config_content = format!(
        r#"
ffmpeg_path = "ffmpeg"
ffprobe_path = "ffprobe"
whispercli_path = "whisper"
model_name = "ggml-base"
watch_directories = ["{}"]
"#,
        temp_dir.path().to_string_lossy()
    );

    let config_path = temp_dir.path().join("config.toml");
    fs::write(&config_path, config_content).unwrap();

    let mut cmd = Command::cargo_bin("atci").unwrap();
    cmd.env("ATCI_CONFIG_PATH", &config_path);
    cmd.args(["search", "can't"]);

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    // Should find the line with "can't"
    assert!(stdout.contains("you can't read it"));
}

#[test]
fn test_search_with_unicode_apostrophe() {
    let temp_dir = TempDir::new().unwrap();
    // Content with Unicode right single quotation mark (U+2019)
    let content = "you can't read it\nthis is another line\nand one more line";
    let _video_path = create_test_video_with_transcript(&temp_dir, "test_video", content);

    // Set up config for search
    let config_content = format!(
        r#"
ffmpeg_path = "ffmpeg"
ffprobe_path = "ffprobe"
whispercli_path = "whisper"
model_name = "ggml-base"
watch_directories = ["{}"]
"#,
        temp_dir.path().to_string_lossy()
    );

    let config_path = temp_dir.path().join("config.toml");
    fs::write(&config_path, config_content).unwrap();

    let mut cmd = Command::cargo_bin("atci").unwrap();
    cmd.env("ATCI_CONFIG_PATH", &config_path);
    cmd.args(["search", "can't"]); // Search with regular apostrophe

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    // Should find the line with Unicode apostrophe when searching with regular apostrophe
    assert!(stdout.contains("you can't read it"));
}

#[test]
fn test_search_unicode_query_with_regular_content() {
    let temp_dir = TempDir::new().unwrap();
    // Content with regular apostrophe
    let content = "you can't read it\nthis is another line\nand one more line";
    let _video_path = create_test_video_with_transcript(&temp_dir, "test_video", content);

    // Set up config for search
    let config_content = format!(
        r#"
ffmpeg_path = "ffmpeg"
ffprobe_path = "ffprobe"
whispercli_path = "whisper"
model_name = "ggml-base"
watch_directories = ["{}"]
"#,
        temp_dir.path().to_string_lossy()
    );

    let config_path = temp_dir.path().join("config.toml");
    fs::write(&config_path, config_content).unwrap();

    let mut cmd = Command::cargo_bin("atci").unwrap();
    cmd.env("ATCI_CONFIG_PATH", &config_path);
    cmd.args(["search", "can't"]); // Search with Unicode apostrophe

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    // Should find the line with regular apostrophe when searching with Unicode apostrophe
    assert!(stdout.contains("you can't read it"));
}
