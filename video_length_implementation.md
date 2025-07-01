# Video Length Implementation

## Overview
Added video length detection and storage functionality to the autotranscript application. Video lengths are determined using ffmpeg and stored in meta files during video processing.

## Changes Made

### 1. Video Processor Updates (`lib/autotranscript/transcriber/video_processor.ex`)

#### New Functions Added:
- `save_video_length/1`: Gets video length and saves it to a meta file
- `get_video_length/1`: Uses ffmpeg to extract video duration in `hh:mm:ss` format

#### Modified Functions:
- `process_video_file/1`: Now calls `save_video_length/1` after deleting the MP3 file

#### Implementation Details:
- Uses `ffmpeg -i <video_file> -f null -` to get video information
- Parses duration from ffmpeg output using regex pattern `Duration: (\d{2}:\d{2}:\d{2}\.\d{2})`
- Removes milliseconds to get clean `hh:mm:ss` format
- Saves length to `filename.meta` file in the same directory as the video

### 2. Transcript Controller Updates (`lib/autotranscript/web/controllers/transcript_controller.ex`)

#### Modified Functions:
- `get_mp4_files/0`: Now reads video length from meta files when available

#### Implementation Details:
- Checks for `filename.meta` file when transcript exists
- Reads and trims the length content from the meta file
- Includes `length` property in the returned file map
- Falls back to `nil` if meta file doesn't exist or can't be read

## File Structure
After processing, each video will have these associated files:
- `video.mp4` - Original video file
- `video.txt` - Transcript file (if processed)
- `video.meta` - Video length in `hh:mm:ss` format (if processed)

## Frontend Integration
The frontend already expects a `length` property in the file data and displays it in the file list table. This implementation provides that data from the backend.

## Error Handling
- Video processor logs warnings if length extraction fails
- Transcript controller gracefully handles missing meta files
- Invalid video files are rejected with appropriate error messages