use assert_cmd::Command;
use predicates::str;
use serde_json::Value;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn setup_test_config() -> (Command, PathBuf) {
    let temp_dir = env::temp_dir();
    let counter = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let test_config_path = temp_dir.join(format!("atci_test_config_{}_{}.toml", 
        std::process::id(), 
        counter));
    
    // Clean up any existing test config
    if test_config_path.exists() {
        fs::remove_file(&test_config_path).ok();
    }
    
    let mut cmd = Command::cargo_bin("atci").unwrap();
    cmd.env("ATCI_CONFIG_PATH", &test_config_path);
    
    (cmd, test_config_path)
}

fn cleanup_test_config(config_path: &PathBuf) {
    if config_path.exists() {
        fs::remove_file(config_path).ok();
    }
}

#[test]
fn test_config_show_command() {
    let (mut cmd, config_path) = setup_test_config();
    cmd.args(&["config", "show"]);
    
    let output = cmd.assert().success();
    let stdout = std::str::from_utf8(&output.get_output().stdout).unwrap();
    
    // Parse JSON output to verify structure
    let json: Value = serde_json::from_str(stdout).expect("Should be valid JSON");
    assert!(json.get("ffmpeg_path").is_some());
    assert!(json.get("ffprobe_path").is_some());
    assert!(json.get("model_name").is_some());
    assert!(json.get("watch_directories").is_some());
    assert!(json.get("whispercli_path").is_some());
    
    cleanup_test_config(&config_path);
}

#[test]
fn test_config_path_command() {
    let (mut cmd, config_path) = setup_test_config();
    cmd.args(&["config", "path"]);
    
    cmd.assert()
        .success()
        .stdout(str::contains("config.toml"));
    
    cleanup_test_config(&config_path);
}

#[test]
fn test_config_set_valid_field() {
    let (mut cmd, config_path) = setup_test_config();
    cmd.args(&["config", "set", "model_name", "test-model"]);
    
    cmd.assert()
        .success()
        .stdout("Set model_name = test-model\n");
    
    cleanup_test_config(&config_path);
}

#[test]
fn test_config_set_ffmpeg_path() {
    let (mut cmd, config_path) = setup_test_config();
    cmd.args(&["config", "set", "ffmpeg_path", "/usr/local/bin/ffmpeg"]);
    
    cmd.assert()
        .success()
        .stdout("Set ffmpeg_path = /usr/local/bin/ffmpeg\n");
    
    cleanup_test_config(&config_path);
}

#[test]
fn test_config_set_ffprobe_path() {
    let (mut cmd, config_path) = setup_test_config();
    cmd.args(&["config", "set", "ffprobe_path", "/usr/local/bin/ffprobe"]);
    
    cmd.assert()
        .success()
        .stdout("Set ffprobe_path = /usr/local/bin/ffprobe\n");
    
    cleanup_test_config(&config_path);
}

#[test]
fn test_config_set_whispercli_path() {
    let (mut cmd, config_path) = setup_test_config();
    cmd.args(&["config", "set", "whispercli_path", "/usr/local/bin/whisper-cli"]);
    
    cmd.assert()
        .success()
        .stdout("Set whispercli_path = /usr/local/bin/whisper-cli\n");
    
    cleanup_test_config(&config_path);
}

#[test]
fn test_config_set_watch_directories() {
    let (mut cmd, config_path) = setup_test_config();
    cmd.args(&["config", "set", "watch_directories", "/path/to/videos"]);
    
    cmd.assert()
        .success()
        .stdout("Set watch_directories = /path/to/videos\n");
    
    cleanup_test_config(&config_path);
}

#[test]
fn test_config_set_password() {
    let (mut cmd, config_path) = setup_test_config();
    cmd.args(&["config", "set", "password", "secret123"]);
    
    cmd.assert()
        .success()
        .stdout("Set password = secret123\n");
    
    cleanup_test_config(&config_path);
}

#[test]
fn test_config_set_invalid_field() {
    let (mut cmd, config_path) = setup_test_config();
    cmd.args(&["config", "set", "invalid_field", "some_value"]);
    
    cmd.assert()
        .failure()
        .stderr(str::contains("Unknown field 'invalid_field'"))
        .stderr(str::contains("Valid fields are: ffmpeg_path, ffprobe_path, model_name, whispercli_path, watch_directories, password"));
    
    cleanup_test_config(&config_path);
}

#[test]
fn test_config_unset_model_name() {
    let (mut cmd, config_path) = setup_test_config();
    cmd.args(&["config", "unset", "model_name"]);
    
    cmd.assert()
        .success()
        .stdout("Unset model_name\n");
    
    cleanup_test_config(&config_path);
}

#[test]
fn test_config_unset_ffmpeg_path() {
    let (mut cmd, config_path) = setup_test_config();
    cmd.args(&["config", "unset", "ffmpeg_path"]);
    
    cmd.assert()
        .success()
        .stdout("Unset ffmpeg_path\n");
    
    cleanup_test_config(&config_path);
}

#[test]
fn test_config_unset_ffprobe_path() {
    let (mut cmd, config_path) = setup_test_config();
    cmd.args(&["config", "unset", "ffprobe_path"]);
    
    cmd.assert()
        .success()
        .stdout("Unset ffprobe_path\n");
    
    cleanup_test_config(&config_path);
}

#[test]
fn test_config_unset_whispercli_path() {
    let (mut cmd, config_path) = setup_test_config();
    cmd.args(&["config", "unset", "whispercli_path"]);
    
    cmd.assert()
        .success()
        .stdout("Unset whispercli_path\n");
    
    cleanup_test_config(&config_path);
}

#[test]
fn test_config_unset_watch_directories() {
    let (mut cmd, config_path) = setup_test_config();
    cmd.args(&["config", "unset", "watch_directories"]);
    
    cmd.assert()
        .success()
        .stdout("Unset watch_directories\n");
    
    cleanup_test_config(&config_path);
}

#[test]
fn test_config_unset_password() {
    let (mut cmd, config_path) = setup_test_config();
    cmd.args(&["config", "unset", "password"]);
    
    cmd.assert()
        .success()
        .stdout("Unset password\n");
    
    cleanup_test_config(&config_path);
}

#[test]
fn test_config_unset_invalid_field() {
    let (mut cmd, config_path) = setup_test_config();
    cmd.args(&["config", "unset", "invalid_field"]);
    
    cmd.assert()
        .failure()
        .stderr(str::contains("Unknown field 'invalid_field'"))
        .stderr(str::contains("Valid fields are: ffmpeg_path, ffprobe_path, model_name, whispercli_path, watch_directories, password"));
    
    cleanup_test_config(&config_path);
}

#[test]
fn test_config_command_no_subcommand_shows_config() {
    let (mut cmd, config_path) = setup_test_config();
    cmd.args(&["config"]);
    
    // When no subcommand is provided, it should default to showing config
    let output = cmd.assert().success();
    let stdout = std::str::from_utf8(&output.get_output().stdout).unwrap();
    
    // Should return JSON config
    let _json: Value = serde_json::from_str(stdout).expect("Should be valid JSON");
    
    cleanup_test_config(&config_path);
}

#[test]
fn test_config_set_requires_field_and_value() {
    let (mut cmd1, config_path1) = setup_test_config();
    cmd1.args(&["config", "set"]);
    
    cmd1.assert()
        .failure()
        .stderr(str::contains("required"));
    
    let (mut cmd2, config_path2) = setup_test_config();
    cmd2.args(&["config", "set", "ffmpeg_path"]);
    
    cmd2.assert()
        .failure()
        .stderr(str::contains("required"));
    
    cleanup_test_config(&config_path1);
    cleanup_test_config(&config_path2);
}

#[test]
fn test_config_unset_requires_field() {
    let (mut cmd, config_path) = setup_test_config();
    cmd.args(&["config", "unset"]);
    
    cmd.assert()
        .failure()
        .stderr(str::contains("required"));
    
    cleanup_test_config(&config_path);
}

#[test]
fn test_config_all_valid_fields_can_be_set() {
    let valid_fields = [
        ("ffmpeg_path", "/usr/bin/ffmpeg"),
        ("ffprobe_path", "/usr/bin/ffprobe"),
        ("model_name", "base.en"),
        ("whispercli_path", "/usr/bin/whisper-cli"),
        ("watch_directories", "/path/to/videos"),
        ("password", "secret123"),
    ];
    
    for (field, value) in &valid_fields {
        let (mut cmd, config_path) = setup_test_config();
        cmd.args(&["config", "set", field, value]);
        cmd.assert().success().stdout(format!("Set {} = {}\n", field, value));
        cleanup_test_config(&config_path);
    }
}

#[test]
fn test_config_all_valid_fields_can_be_unset() {
    let valid_fields = [
        "ffmpeg_path",
        "ffprobe_path", 
        "model_name",
        "whispercli_path",
        "watch_directories",
        "password",
    ];
    
    for field in &valid_fields {
        let (mut cmd, config_path) = setup_test_config();
        cmd.args(&["config", "unset", field]);
        cmd.assert().success().stdout(format!("Unset {}\n", field));
        cleanup_test_config(&config_path);
    }
}

#[test]
fn test_config_field_validation_comprehensive() {
    let invalid_fields = [
        "invalid_field",
        "wrong_field",
        "unknown",
        "bad_field_name",
    ];
    
    for field in &invalid_fields {
        // Test set with invalid field
        let (mut set_cmd, config_path1) = setup_test_config();
        set_cmd.args(&["config", "set", field, "test_value"]);
        set_cmd.assert()
            .failure()
            .stderr(str::contains(&format!("Unknown field '{}'", field)));
        cleanup_test_config(&config_path1);
        
        // Test unset with invalid field
        let (mut unset_cmd, config_path2) = setup_test_config();
        unset_cmd.args(&["config", "unset", field]);
        unset_cmd.assert()
            .failure()
            .stderr(str::contains(&format!("Unknown field '{}'", field)));
        cleanup_test_config(&config_path2);
    }
}