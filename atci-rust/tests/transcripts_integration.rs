use assert_cmd::Command;
use predicates::str;
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
fn test_transcripts_get_success() {
    let temp_dir = TempDir::new().unwrap();
    let content = "Line 1\nLine 2\nLine 3";
    let video_path = create_test_video_with_transcript(&temp_dir, "test_video", content);
    
    let mut cmd = Command::cargo_bin("atci-rust").unwrap();
    cmd.args(&["transcripts", "get", &video_path]);
    
    cmd.assert()
        .success()
        .stdout(format!("{}\n", content));
}

#[test]
fn test_transcripts_get_file_not_exists() {
    let temp_dir = TempDir::new().unwrap();
    let video_path = temp_dir.path().join("nonexistent.mp4").to_string_lossy().to_string();
    
    let mut cmd = Command::cargo_bin("atci-rust").unwrap();
    cmd.args(&["transcripts", "get", &video_path]);
    
    cmd.assert()
        .failure()
        .stderr(str::contains("Transcript file does not exist"));
}

#[test]
fn test_transcripts_set_line_success() {
    let temp_dir = TempDir::new().unwrap();
    let original_content = "Line 1\nLine 2\nLine 3";
    let video_path = create_test_video_with_transcript(&temp_dir, "test_video", original_content);
    
    let mut cmd = Command::cargo_bin("atci-rust").unwrap();
    cmd.args(&["transcripts", "set-line", &video_path, "2", "Modified Line 2"]);
    
    cmd.assert()
        .success()
        .stdout(str::contains("Successfully updated line 2"));
    
    // Verify the content was actually changed
    let txt_path = temp_dir.path().join("test_video.txt");
    let updated_content = fs::read_to_string(&txt_path).unwrap();
    assert_eq!(updated_content, "Line 1\nModified Line 2\nLine 3");
}

#[test]
fn test_transcripts_set_line_zero_line_number() {
    let temp_dir = TempDir::new().unwrap();
    let content = "Line 1\nLine 2\nLine 3";
    let video_path = create_test_video_with_transcript(&temp_dir, "test_video", content);
    
    let mut cmd = Command::cargo_bin("atci-rust").unwrap();
    cmd.args(&["transcripts", "set-line", &video_path, "0", "New content"]);
    
    cmd.assert()
        .failure()
        .stderr(str::contains("Line number must be greater than 0"));
}

#[test]
fn test_transcripts_set_line_beyond_file_length() {
    let temp_dir = TempDir::new().unwrap();
    let content = "Line 1\nLine 2\nLine 3";
    let video_path = create_test_video_with_transcript(&temp_dir, "test_video", content);
    
    let mut cmd = Command::cargo_bin("atci-rust").unwrap();
    cmd.args(&["transcripts", "set-line", &video_path, "10", "New content"]);
    
    cmd.assert()
        .failure()
        .stderr(str::contains("is beyond the end of the file"));
}

#[test]
fn test_transcripts_set_line_file_not_exists() {
    let temp_dir = TempDir::new().unwrap();
    let video_path = temp_dir.path().join("nonexistent.mp4").to_string_lossy().to_string();
    
    let mut cmd = Command::cargo_bin("atci-rust").unwrap();
    cmd.args(&["transcripts", "set-line", &video_path, "1", "New content"]);
    
    cmd.assert()
        .failure()
        .stderr(str::contains("Transcript file does not exist"));
}

#[test]
fn test_transcripts_set_success() {
    let temp_dir = TempDir::new().unwrap();
    let original_content = "Old content";
    let video_path = create_test_video_with_transcript(&temp_dir, "test_video", original_content);
    
    let new_content = "Completely new content\nWith multiple lines";
    
    let mut cmd = Command::cargo_bin("atci-rust").unwrap();
    cmd.args(&["transcripts", "set", &video_path, new_content]);
    
    cmd.assert()
        .success()
        .stdout(str::contains("Successfully replaced transcript content"));
    
    // Verify the content was actually changed
    let txt_path = temp_dir.path().join("test_video.txt");
    let updated_content = fs::read_to_string(&txt_path).unwrap();
    assert_eq!(updated_content, new_content);
}

#[test]
fn test_transcripts_set_creates_new_file() {
    let temp_dir = TempDir::new().unwrap();
    let video_path = temp_dir.path().join("new_video.mp4").to_string_lossy().to_string();
    
    let content = "New transcript content";
    
    let mut cmd = Command::cargo_bin("atci-rust").unwrap();
    cmd.args(&["transcripts", "set", &video_path, content]);
    
    cmd.assert()
        .success()
        .stdout(str::contains("Successfully replaced transcript content"));
    
    // Verify the file was created
    let txt_path = temp_dir.path().join("new_video.txt");
    assert!(txt_path.exists());
    let saved_content = fs::read_to_string(&txt_path).unwrap();
    assert_eq!(saved_content, content);
}

#[test]

fn test_transcripts_regenerate_success_txt_only() {
    let temp_dir = TempDir::new().unwrap();
    let content = "Test content";
    let video_path = create_test_video_with_transcript(&temp_dir, "test_video", content);
    
    let txt_path = temp_dir.path().join("test_video.txt");
    let meta_path = temp_dir.path().join("test_video.meta");
    
    // Verify only txt file exists
    assert!(txt_path.exists());
    assert!(!meta_path.exists());
    
    let mut cmd = Command::cargo_bin("atci-rust").unwrap();
    cmd.args(&["transcripts", "regenerate", &video_path]);
    
    cmd.assert()
        .success()
        .stdout(str::contains("Successfully deleted transcript files"));
    
    // Verify txt file was deleted
    assert!(!txt_path.exists());
}

#[test]
fn test_transcripts_regenerate_no_files_to_delete() {
    let temp_dir = TempDir::new().unwrap();
    let video_path = temp_dir.path().join("nonexistent.mp4").to_string_lossy().to_string();
    
    let mut cmd = Command::cargo_bin("atci-rust").unwrap();
    cmd.args(&["transcripts", "regenerate", &video_path]);
    
    cmd.assert()
        .failure()
        .stderr(str::contains("No transcript files found to delete"));
}

#[test]
fn test_transcripts_command_requires_subcommand() {
    let mut cmd = Command::cargo_bin("atci-rust").unwrap();
    cmd.args(&["transcripts"]);
    
    cmd.assert()
        .failure()
        .stderr(str::contains("Usage: atci-rust transcripts [COMMAND]"));
}

#[test]
fn test_transcripts_get_requires_path() {
    let mut cmd = Command::cargo_bin("atci-rust").unwrap();
    cmd.args(&["transcripts", "get"]);
    
    cmd.assert()
        .failure()
        .stderr(str::contains("required"));
}

#[test]
fn test_transcripts_set_line_requires_all_args() {
    let mut cmd = Command::cargo_bin("atci-rust").unwrap();
    cmd.args(&["transcripts", "set-line"]);
    
    cmd.assert()
        .failure()
        .stderr(str::contains("required"));
    
    let mut cmd2 = Command::cargo_bin("atci-rust").unwrap();
    cmd2.args(&["transcripts", "set-line", "video.mp4"]);
    
    cmd2.assert()
        .failure()
        .stderr(str::contains("required"));
    
    let mut cmd3 = Command::cargo_bin("atci-rust").unwrap();
    cmd3.args(&["transcripts", "set-line", "video.mp4", "1"]);
    
    cmd3.assert()
        .failure()
        .stderr(str::contains("required"));
}

#[test]
fn test_transcripts_set_requires_all_args() {
    let mut cmd = Command::cargo_bin("atci-rust").unwrap();
    cmd.args(&["transcripts", "set"]);
    
    cmd.assert()
        .failure()
        .stderr(str::contains("required"));
    
    let mut cmd2 = Command::cargo_bin("atci-rust").unwrap();
    cmd2.args(&["transcripts", "set", "video.mp4"]);
    
    cmd2.assert()
        .failure()
        .stderr(str::contains("required"));
}

#[test]
fn test_transcripts_regenerate_requires_path() {
    let mut cmd = Command::cargo_bin("atci-rust").unwrap();
    cmd.args(&["transcripts", "regenerate"]);
    
    cmd.assert()
        .failure()
        .stderr(str::contains("required"));
}