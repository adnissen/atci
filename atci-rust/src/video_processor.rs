use std::fs;
use std::path::Path;

pub fn create_transcript(video_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let video_path = Path::new(video_path);
    let txt_path = video_path.with_extension("txt");
    
    if !txt_path.exists() {
        fs::write(&txt_path, "")?;
        println!("Created transcript file: {}", txt_path.display());
    }
    
    Ok(())
}

pub fn create_metafile(video_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let video_path = Path::new(video_path);
    let meta_path = video_path.with_extension("meta");
    
    if !meta_path.exists() {
        fs::write(&meta_path, "")?;
        println!("Created meta file: {}", meta_path.display());
    }
    
    Ok(())
}