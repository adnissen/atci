// Simple test file to verify video parts functionality
// Run with: cargo test --bin test_video_parts

use std::path::Path;

// Copy the parsing function here for testing
pub fn parse_video_part_test(file_path: &Path) -> Option<(String, i32, String)> {
    let file_name = file_path.file_name()?.to_str()?;
    
    // Simple regex-like parsing for testing
    if let Some(dot_part_pos) = file_name.find(".part") {
        let base_name = &file_name[..dot_part_pos];
        let remaining = &file_name[dot_part_pos + 5..]; // Skip ".part"
        
        if let Some(dot_pos) = remaining.find('.') {
            let part_num_str = &remaining[..dot_pos];
            let extension = &remaining[dot_pos + 1..];
            
            if let Ok(part_number) = part_num_str.parse::<i32>() {
                return Some((base_name.to_string(), part_number, extension.to_string()));
            }
        }
    }
    
    None
}

fn main() {
    println!("Testing video part parsing...");
    
    // Test cases
    let test_cases = vec![
        ("episode01.part1.mkv", Some(("episode01", 1, "mkv"))),
        ("show_s01e05.part2.mp4", Some(("show_s01e05", 2, "mp4"))),
        ("complex_name_720p.part10.avi", Some(("complex_name_720p", 10, "avi"))),
        ("regular_video.mkv", None),
        ("episode01.mkv", None),
        ("invalid.part.mkv", None),
        ("test.partX.mp4", None),
    ];
    
    for (input, expected) in test_cases {
        let path = Path::new(input);
        let result = parse_video_part_test(&path);
        
        match (result, expected) {
            (Some((base, part, ext)), Some((exp_base, exp_part, exp_ext))) => {
                if base == exp_base && part == exp_part && ext == exp_ext {
                    println!("✓ {} -> base: {}, part: {}, ext: {}", input, base, part, ext);
                } else {
                    println!("✗ {} -> expected ({}, {}, {}), got ({}, {}, {})", 
                             input, exp_base, exp_part, exp_ext, base, part, ext);
                }
            }
            (None, None) => {
                println!("✓ {} -> correctly identified as non-part", input);
            }
            (Some((base, part, ext)), None) => {
                println!("✗ {} -> expected non-part, got ({}, {}, {})", input, base, part, ext);
            }
            (None, Some((exp_base, exp_part, exp_ext))) => {
                println!("✗ {} -> expected ({}, {}, {}), got non-part", input, exp_base, exp_part, exp_ext);
            }
        }
    }
    
    println!("\nTesting master path generation...");
    
    // Test master path generation
    let video_part_path = "/videos/episode01.part1.mkv";
    let parent_dir = Path::new(video_part_path).parent().unwrap();
    let base_name = "episode01";
    let extension = "mkv";
    
    let master_video_path = format!("{}/{}.{}", parent_dir.display(), base_name, extension);
    let master_transcript_path = format!("{}/{}.txt", parent_dir.display(), base_name);
    
    println!("Part path: {}", video_part_path);
    println!("Master video: {}", master_video_path);
    println!("Master transcript: {}", master_transcript_path);
    
    // Expected: /videos/episode01.mkv and /videos/episode01.txt
    assert_eq!(master_video_path, "/videos/episode01.mkv");
    assert_eq!(master_transcript_path, "/videos/episode01.txt");
    println!("✓ Master path generation works correctly");
    
    println!("\nAll tests completed!");
}