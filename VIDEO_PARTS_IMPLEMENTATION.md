# Video Parts Implementation Summary

## Overview
This implementation adds support for processing multi-part videos in the atci system. Videos with filenames like `episode01.part1.mkv`, `episode01.part2.mkv`, etc. are automatically detected, processed, and combined into a single master video with continuous timestamps.

## Key Features

### 1. **Automatic Part Detection**
- Detects filename pattern: `basename.partN.extension`
- Examples: `episode01.part1.mkv`, `show_s01e05.part2.mp4`
- Parses base name, part number, and extension

### 2. **Sequential Processing**
- Parts must be processed in order (part 1, then part 2, etc.)
- If a part arrives out of order, creates placeholder transcript
- Automatically queues next part when current part completes

### 3. **Continuous Timestamps**
- Adjusts timestamps in transcripts to be continuous across parts
- Part 1: 00:00:00 - 00:10:00
- Part 2: 00:10:00 - 00:20:00 (timestamps adjusted by part 1 duration)

### 4. **Master File Management**
- Creates `episode01.mkv` (master video) and `episode01.txt` (master transcript)
- Master files are updated each time a new part is processed
- Original part files are deleted after successful processing

### 5. **Error Handling**
- Failed parts insert error messages into master transcript
- Original part video files are preserved on failure
- Processing continues with subsequent parts

## Database Changes

### New Table: `video_parts`
```sql
CREATE TABLE video_parts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    base_name TEXT NOT NULL,
    part_number INTEGER NOT NULL,
    video_path TEXT NOT NULL UNIQUE,
    processed_at TEXT NOT NULL,
    transcript_length INTEGER DEFAULT 0,
    UNIQUE(base_name, part_number)
);
```

### Schema Version Update
- Updated from `20250909-4` to `20250921-1`

## File Structure Changes

### New Module: `src/video_parts.rs`
- `parse_video_part()` - Detects and parses video part filenames
- `get_master_paths()` - Generates master file paths
- `is_part_processed()` - Checks if a part was already processed
- `get_processed_parts()` - Returns list of processed parts
- `find_missing_parts()` - Identifies missing parts in sequence
- `record_processed_part()` - Records successful processing in database
- `check_and_queue_next_part()` - Automatically queues next sequential part
- `create_missing_part_placeholder()` - Creates placeholder for missing parts

### Updated Module: `src/video_processor.rs`
- `cancellable_create_transcript()` - Updated to handle both regular and part videos
- `cancellable_create_transcript_for_part()` - New function for part-specific processing
- `cancellable_create_transcript_single()` - Renamed original function for regular videos
- `adjust_transcript_timestamps()` - Adjusts timestamps by duration offset
- `update_master_video()` - Concatenates parts using FFmpeg
- Helper functions for timestamp and duration parsing

### Updated Module: `src/db.rs`
- Added `video_parts` table creation
- Updated schema version

### Updated Module: `src/main.rs`
- Added `video_parts` module import

## Workflow Example

### Processing `episode01.part1.mkv`:
1. Detects as video part (base: "episode01", part: 1)
2. No missing previous parts
3. Creates transcript normally
4. Saves to master transcript with ">>> Part 1 <<<" header
5. Creates master video `episode01.mkv`
6. Records part 1 as processed
7. Deletes `episode01.part1.mkv`
8. Checks for `episode01.part2.mkv` and queues if found

### Processing `episode01.part2.mkv`:
1. Detects as video part (base: "episode01", part: 2)
2. Finds part 1 already processed
3. Creates transcript for part 2
4. Calculates duration offset from part 1
5. Adjusts timestamps in part 2 transcript
6. Appends to master transcript with ">>> Part 2 <<<" header
7. Updates master video by concatenating parts 1 and 2
8. Records part 2 as processed
9. Deletes `episode01.part2.mkv`
10. Checks for `episode01.part3.mkv` and queues if found

### Processing `episode01.part3.mkv` (missing part 2):
1. Detects as video part (base: "episode01", part: 3)
2. Finds part 2 is missing
3. Creates placeholder transcript: ">>> Part 3 of video, missing part 2 <<<"
4. Does not process the video content
5. Still queues `episode01.part4.mkv` if found

### Processing `episode01.part2.mkv` (after part 3):
1. Detects as video part (base: "episode01", part: 2)
2. No missing previous parts (part 1 exists)
3. Processes normally and updates master files
4. Automatically queues `episode01.part3.mkv` for reprocessing

## Error Handling

### Processing Failures:
- Insert error message into master transcript
- Keep original part video file for debugging
- Continue processing subsequent parts
- Example error message: ">>> Part 2 FAILED: episode01 <<< Error processing part 2: [error details]"

### Missing Parts:
- Create placeholder transcript
- Example: ">>> Part 3 of video, missing part(s): 2 <<< Processing paused until missing parts are available."

### FFmpeg Concatenation Failures:
- Log detailed error messages
- Master video remains in previous state
- Part processing continues for transcript

## Configuration Requirements
- FFmpeg must be configured for video concatenation
- FFprobe must be configured for duration calculation
- All existing transcription settings (Whisper, subtitles) work normally

## Backward Compatibility
- Regular (non-part) videos continue to work exactly as before
- No changes to existing API endpoints
- No changes to existing database records

## Testing
- Unit tests for filename parsing in `src/video_parts.rs`
- Integration test file: `test_video_parts.rs`
- Manual testing recommended with actual video part files

## Future Enhancements
- Web UI support for video parts management
- Batch part processing
- Part reordering/insertion capabilities
- Progress tracking for multi-part videos