# Autotranscript

Autotranscript is an Elixir application that automatically transcribes video files to text using Whisper. It watches a configured directory for new video files (MP4, MOV, MKV) and processes them automatically.

## Description

- Monitors one or more directories for new video files (MP4, MOV, MKV) or videos without a .txt transcript
- Automatically detects and extracts existing subtitles from video files
- If no subtitles are found, converts videos to MP3 audio using ffmpeg
- Transcribes audio to text using Whisper (when subtitles are not available)
- Deletes the MP3 file after transcription
- When subtitles are extracted, saves "source: subtitles" to the .meta file
- Source information (model name or "subtitles") is stored in .meta files

## Prerequisites

- Elixir 1.18 or later
- ffmpeg and ffprobe installed and available in PATH
- [whisper.cpp](https://github.com/ggerganov/whisper.cpp) compiled and model downloaded

## Configuration

Configuration is stored in a `.atconfig` file in JSON format in your home directory:

```json
{
  "watch_directories": ["/path/to/your/videos", "/path/to/another/video/folder"],
  "whispercli_path": "/path/to/whisper-cli",
  "model_path": "/path/to/your/whisper.cpp/model.bin"
}
```

**Note**: Watch directories cannot be subdirectories of each other. The system uses the first watch directory for primary operations.