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

pub fn get_whisper_cli_url(platform: &str) -> Option<&'static str> {
    match platform {
        "windows" => Some("https://example.com/ffprobe-windows.exe"),
        "macos-arm" => Some("https://autotranscript.s3.us-east-1.amazonaws.com/binaries/whisper-cli"),
        "macos-x86" => Some("https://www.osxexperts.net/ffprobe71intel.zip"),
        "linux" => Some("https://example.com/ffprobe-linux"),
        _ => None,
    }
}

pub fn binaries_directory(tool: &str) -> std::path::PathBuf {
    let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    std::path::Path::new(&home_dir).join(".atci").join(tool)
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
    let tools = ["ffmpeg", "ffprobe", "whisper-cli"];

    let cfg: crate::AtciConfig = confy::load("atci", "config").unwrap();
    
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
            current_path: get_current_path(tool, &cfg),
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
    let binaries_dir = binaries_directory(tool);
    let extension = if cfg!(target_os = "windows") { ".exe" } else { "" };
    binaries_dir.join(format!("{}{}", tool, extension)).to_string_lossy().to_string()
}

fn find_in_system_path(tool: &str) -> Option<String> {
    which::which(tool).ok().map(|path| path.to_string_lossy().to_string())
}

fn get_current_path(tool: &str, cfg: &crate::AtciConfig) -> String {
    if tool == "whisper-cli" {
        cfg.whispercli_path.clone()
    } else if tool == "ffmpeg" {
        cfg.ffmpeg_path.clone()
    } else if tool == "ffprobe" {
        cfg.ffprobe_path.clone()
    } else {
        "not found".to_string()
    }
}

pub fn download_tool(tool: &str) -> Result<String, Box<dyn std::error::Error>> {
    let platform = detect_platform();
    
    let url = match tool {
        "ffmpeg" => get_ffmpeg_url(&platform),
        "ffprobe" => get_ffprobe_url(&platform),
        "whisper-cli" => get_whisper_cli_url(&platform),
        _ => return Err(format!("Unknown tool: {}", tool).into()),
    };
    println!("Downloading tool: {} from {}", tool, url.unwrap());

    let url = url.ok_or(format!("No download URL available for {} on {}", tool, platform))?;
    
    let binaries_dir = binaries_directory(tool);
    std::fs::create_dir_all(&binaries_dir)?;
    
    let response = reqwest::blocking::get(url)?;
    let bytes = response.bytes()?;
    
    let cursor = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor)?;
    
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let file_name = file.name();
        
        if file_name.contains(tool) && !file_name.ends_with('/') {
            let extension = if cfg!(target_os = "windows") { ".exe" } else { "" };
            let output_path = binaries_dir.join(format!("{}{}", tool, extension));
            
            let mut output_file = std::fs::File::create(&output_path)?;
            std::io::copy(&mut file, &mut output_file)?;
            
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let metadata = std::fs::metadata(&output_path)?;
                let mut permissions = metadata.permissions();
                permissions.set_mode(0o755);
                std::fs::set_permissions(&output_path, permissions)?;
            }
            
            #[cfg(target_os = "macos")]
            {
                if let Err(e) = handle_macos_quarantine(&output_path.to_string_lossy(), &platform) {
                    eprintln!("Warning: Failed to handle macOS quarantine: {}", e);
                }
            }
            
            return Ok(output_path.to_string_lossy().to_string());
        }
    }
    
    Err(format!("Could not find {} binary in the downloaded archive", tool).into())
}

#[cfg(target_os = "macos")]
pub fn handle_macos_quarantine(executable_path: &str, platform: &str) -> Result<(), Box<dyn std::error::Error>> {
    check_macos_version()?;
    remove_quarantine(executable_path)?;
    handle_arm_mac_signing(executable_path, platform)?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn check_macos_version() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::process::Command::new("sw_vers")
        .arg("-productVersion")
        .output()?;

    if !output.status.success() {
        return Err("Unable to get macOS version".into());
    }

    let version_string = String::from_utf8(output.stdout)?;
    let version_parts: Vec<&str> = version_string.trim().split('.').collect();

    if version_parts.len() < 2 {
        return Err("Unable to parse macOS version".into());
    }

    let major: i32 = version_parts[0].parse()?;
    let minor: i32 = version_parts[1].parse()?;

    if major > 10 || (major == 10 && minor >= 15) {
        Ok(())
    } else {
        Err("macOS version too old for quarantine handling".into())
    }
}

#[cfg(target_os = "macos")]
fn remove_quarantine(executable_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Removing quarantine from {}", executable_path);

    let output = std::process::Command::new("xattr")
        .args(["-dr", "com.apple.quarantine", executable_path])
        .output()?;

    if output.status.success() {
        println!("Successfully removed quarantine");
        Ok(())
    } else {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        eprintln!("xattr command failed with exit code {:?}: {}", output.status.code(), error_msg);
        Err(format!("Failed to remove quarantine: {}", error_msg).into())
    }
}

#[cfg(target_os = "macos")]
fn handle_arm_mac_signing(executable_path: &str, platform: &str) -> Result<(), Box<dyn std::error::Error>> {
    if platform == "macos-arm" {
        println!("Handling ARM Mac code signing for {}", executable_path);
        clear_extended_attributes(executable_path)?;
        codesign_executable(executable_path)?;
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn clear_extended_attributes(executable_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Clearing extended attributes");

    let output = std::process::Command::new("xattr")
        .args(["-cr", executable_path])
        .output()?;

    if output.status.success() {
        println!("Successfully cleared extended attributes");
        Ok(())
    } else {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        eprintln!("xattr -cr command failed with exit code {:?}: {}", output.status.code(), error_msg);
        Err(format!("Failed to clear extended attributes: {}", error_msg).into())
    }
}

#[cfg(target_os = "macos")]
fn codesign_executable(executable_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Code signing executable");

    let output = std::process::Command::new("codesign")
        .args(["-s", "-", executable_path])
        .output()?;

    if output.status.success() {
        println!("Successfully code signed executable");
        Ok(())
    } else {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        eprintln!("codesign command failed with exit code {:?}: {}", output.status.code(), error_msg);
        Err(format!("Failed to code sign executable: {}", error_msg).into())
    }
}