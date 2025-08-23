use std::fs;
use std::path::Path;

pub fn get_transcript(video_path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let video_path_obj = Path::new(video_path);
    let txt_path = video_path_obj.with_extension("txt");
    
    if !txt_path.exists() {
        return Err(format!("Transcript file does not exist: {}", txt_path.display()).into());
    }
    
    let content = fs::read_to_string(&txt_path)?;
    Ok(content)
}

pub fn set_line(video_path: &str, line_number: usize, new_content: &str) -> Result<(), Box<dyn std::error::Error>> {
    if line_number == 0 {
        return Err("Line number must be greater than 0".into());
    }
    
    let video_path_obj = Path::new(video_path);
    let txt_path = video_path_obj.with_extension("txt");
    
    if !txt_path.exists() {
        return Err(format!("Transcript file does not exist: {}", txt_path.display()).into());
    }
    
    let content = fs::read_to_string(&txt_path)?;
    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    
    let line_index = line_number - 1; // Convert to 0-based index
    
    if line_index >= lines.len() {
        return Err(format!("Line number {} is beyond the end of the file (file has {} lines)", 
                          line_number, lines.len()).into());
    }
    
    lines[line_index] = new_content.to_string();
    
    // Preserve the original line ending style
    let line_ending = if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    
    let updated_content = lines.join(line_ending);
    fs::write(&txt_path, updated_content)?;
    
    Ok(())
}

pub fn set(video_path: &str, new_content: &str) -> Result<(), Box<dyn std::error::Error>> {
    let video_path_obj = Path::new(video_path);
    let txt_path = video_path_obj.with_extension("txt");
    
    fs::write(txt_path, new_content)?;
    Ok(())
}

pub fn regenerate(video_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let video_path_obj = Path::new(video_path);
    let txt_path = video_path_obj.with_extension("txt");
    let meta_path = video_path_obj.with_extension("meta");
    
    let mut deleted_files = Vec::new();
    
    if txt_path.exists() {
        fs::remove_file(&txt_path)?;
        deleted_files.push("transcript");
    }
    
    if meta_path.exists() {
        fs::remove_file(&meta_path)?;
        deleted_files.push("meta");
    }
    
    if deleted_files.is_empty() {
        return Err("No transcript or meta files found to delete".into());
    }
    
    Ok(())
}