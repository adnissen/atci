#![allow(dead_code)]

use indicatif::{ProgressBar, ProgressStyle};
use std::io::Read;
use std::path::{Path, PathBuf};

pub async fn download_test_content(dir: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    // Determine the target directory
    let target_dir = if let Some(d) = dir {
        d
    } else {
        // Use home directory + atci_dev
        let home = dirs::home_dir().ok_or("Could not find home directory")?;
        home.join("atci_dev")
    };

    todo!(
        "Target dir: {}. Still need to decide what videos to download!",
        target_dir.display()
    );
    /*
    // Create the directory if it doesn't exist
    std::fs::create_dir_all(&target_dir)?;
    println!("Created/verified directory: {}", target_dir.display());

    // Add to watch directories
    let config = crate::config::load_config()?;
    let mut updated_config = config.clone();

    // Add the directory if it's not already in the watch list
    let target_str = target_dir.to_string_lossy().to_string();
    if !updated_config.watch_directories.contains(&target_str) {
        updated_config.watch_directories.push(target_str.clone());
        crate::config::store_config(&updated_config)?;
        println!("Added {} to watch directories", target_str);
    } else {
        println!("Directory already in watch list");
    }

    // Hardcoded list of videos (empty for now)
    let videos: Vec<(&str, &str)> = vec![
        // Add videos in format: (url, filename)
        // Example: ("https://example.com/video.mp4", "test_video.mp4"),
    ];

    if videos.is_empty() {
        println!("No test videos configured yet");
    } else {
        println!("Downloading {} test video(s)...\n", videos.len());

        for (video_url, filename) in videos {
            match download_video(video_url, filename, &target_dir) {
                Ok(path) => {
                    println!("Successfully downloaded: {}\n", path.display());
                }
                Err(e) => {
                    eprintln!("Failed to download {}: {}\n", filename, e);
                }
            }
        }

        println!(
            "Download complete! Videos saved to: {}",
            target_dir.display()
        );
    }

    Ok(())
    */
}

fn download_video(
    url: &str,
    filename: &str,
    target_dir: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    println!("Downloading: {} from {}", filename, url);

    let response = reqwest::blocking::get(url)?;
    let total_size = response.content_length().unwrap_or(0);

    // Create progress bar
    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")?
            .progress_chars("#>-")
    );
    pb.set_message(format!("Downloading {}", filename));

    // Download with progress tracking
    let mut bytes = Vec::new();
    let mut response = response;
    let mut buffer = [0; 8192]; // 8KB buffer

    loop {
        match response.read(&mut buffer) {
            Ok(0) => break, // EOF
            Ok(n) => {
                bytes.extend_from_slice(&buffer[..n]);
                pb.inc(n as u64);
            }
            Err(e) => return Err(e.into()),
        }
    }

    pb.finish_with_message(format!("Downloaded {} successfully!", filename));

    // Save the file
    let output_path = target_dir.join(filename);
    std::fs::write(&output_path, &bytes)?;

    Ok(output_path)
}
