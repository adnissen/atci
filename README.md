# Autotranscript

Autotranscript is an Elixir application that automatically transcribes video files to text using Whisper. It watches a configured directory for new MP4 files and processes them automatically.

## Description

- Monitors a directory for new MP4 video files or videos without a .txt transcript
- Automatically converts videos to MP3 audio using ffmpeg
- Transcribes audio to text using Whisper
- Deletes the MP3 file

## Prerequisites

- Elixir 1.18 or later
- ffmpeg installed and available in PATH
- [whisper.cpp](https://github.com/ggerganov/whisper.cpp) compiled and model downloaded

## Quick Start

📋 **New to Autotranscript?** See [GETTING_STARTED.md](GETTING_STARTED.md) for a step-by-step setup guide.

## Configuration

**Configuration is required** - Autotranscript will not start without a valid `.atconfig` file.

### Option 1: Configuration File

Create a `.atconfig` file in either:
- The directory where you run the application (takes precedence), or
- Your home directory (`~/.atconfig`)

The `.atconfig` file must contain all three required settings:

```
# Directory where video files are stored and transcripts will be saved
watch_directory=/path/to/your/videos

# Path to the whisper-cli executable  
whispercli_path=/usr/local/bin/whisper-cli

# Path to the Whisper model file (.bin)
model_path=/path/to/your/whisper.cpp/model.bin
```

### Option 2: Web Interface

If no valid configuration file is found, the web interface will prompt you to configure these settings when you first access the application. The configuration will be saved to a `.atconfig` file in the current directory.

### Configuration Priority

1. `.atconfig` file in current directory (highest priority)
2. `.atconfig` file in home directory

**Note:** All three configuration values (`watch_directory`, `whispercli_path`, `model_path`) are required for the application to function.

## API Endpoints

The application provides the following configuration API endpoints:

- `GET /api/config` - Get current configuration
- `POST /api/config` - Update configuration
- `GET /api/config/status` - Check if configuration is valid