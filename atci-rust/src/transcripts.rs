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