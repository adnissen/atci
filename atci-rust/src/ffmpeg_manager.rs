pub fn get_ffmpeg_url(platform: &str) -> Option<&'static str> {
    match platform {
        "windows" => Some("https://example.com/ffmpeg-windows.exe"),
        "macos-arm" => Some("https://www.osxexperts.net/ffmpeg711arm.zip"),
        "macos-x86" => Some("https://www.osxexperts.net/ffmpeg71intel.zip"),
        "linux" => Some("https://example.com/ffmpeg-linux"),
        _ => None,
    }
}

pub fn get_ffprobe_url(platform: &str) -> Option<&'static str> {
    match platform {
        "windows" => Some("https://example.com/ffprobe-windows.exe"),
        "macos-arm" => Some("https://www.osxexperts.net/ffprobe711arm.zip"),
        "macos-x86" => Some("https://www.osxexperts.net/ffprobe71intel.zip"),
        "linux" => Some("https://example.com/ffprobe-linux"),
        _ => None,
    }
}

pub fn binaries_directory() -> std::path::PathBuf {
    let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    std::path::Path::new(&home_dir).join(".atci").join("ffmpeg")
}

#[derive(Debug, serde::Serialize)]
pub struct ToolInfo {
    pub name: String,
    pub platform: String,
    pub downloaded: bool,
    pub downloaded_path: String,
    pub system_available: bool,
    pub system_path: Option<String>,
    pub current_path: String,
}

pub fn list_tools() -> Vec<ToolInfo> {
    let platform = detect_platform();
    let tools = ["ffmpeg", "ffprobe"];
    
    tools.iter().map(|&tool| {
        let downloaded_path = get_downloaded_path(tool);
        let system_path = find_in_system_path(tool);
        
        ToolInfo {
            name: tool.to_string(),
            platform: platform.clone(),
            downloaded: std::path::Path::new(&downloaded_path).exists(),
            downloaded_path: downloaded_path.clone(),
            system_available: system_path.is_some(),
            system_path: system_path.clone(),
            current_path: get_current_path(tool),
        }
    }).collect()
}

fn detect_platform() -> String {
    if cfg!(target_os = "windows") {
        "windows".to_string()
    } else if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            "macos-arm".to_string()
        } else {
            "macos-x86".to_string()
        }
    } else if cfg!(target_os = "linux") {
        "linux".to_string()
    } else {
        "unknown".to_string()
    }
}

fn get_downloaded_path(tool: &str) -> String {
    let binaries_dir = binaries_directory();
    let extension = if cfg!(target_os = "windows") { ".exe" } else { "" };
    binaries_dir.join(format!("{}{}", tool, extension)).to_string_lossy().to_string()
}

fn find_in_system_path(tool: &str) -> Option<String> {
    which::which(tool).ok().map(|path| path.to_string_lossy().to_string())
}

fn get_current_path(tool: &str) -> String {
    let downloaded_path = get_downloaded_path(tool);
    if std::path::Path::new(&downloaded_path).exists() {
        downloaded_path
    } else if let Some(system_path) = find_in_system_path(tool) {
        system_path
    } else {
        "not found".to_string()
    }
}