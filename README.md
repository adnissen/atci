# atci (Andrew's Transcript and Clipping Interface)

atci is an application which provides both a simple command line interface and a responsive web interface for automatically creating video transcripts for all videos in a given set of folders (with whisper) as well as searching through them to easily make gifs, audio, and video clips (via ffmpeg).

The different parts of the application can be run separately. For instance:

* To just watch the set of configured directories for new videos to transcribe:
* * `atci watch`
* To host only the `/api` endpoints (i.e. not the react app, if you want to build your own interface):
* * `atci web api`
* To launch everything, including the directory watcher and the web interface:
* * `atci web`

All of the data that backs the `/api` routes is also available via the command line interface as json output. Run `atci` with no arguments to display the help dialog which lists subcommands, many of which have subcommands themselves.

## 🎯 Why atci?

- **Automatic Processing**: Drop video files in a folder and atci handles the rest
- **Smart Transcription**: Uses Whisper AI for accurate speech-to-text conversion
- **Existing Subtitles**: Automatically extracts and uses existing subtitles when available
- **Beautiful Web UI**: Modern React interface for browsing and searching transcripts
- **Video Playback**: Built-in video player with timestamp synchronization
- **Full-Text Search**: Quickly find content across all your transcripts
- **Edit Capabilities**: Fix transcription errors directly in the web interface
- **Batch Processing**: Handles multiple videos with queue management

## 🚀 Quick Start for Users

### Prerequisites

**For M-series (ARM) Mac users**: The software can automatically download and configure ffprobe, ffmpeg, whisper.cpp, and the AI model for you! Just run the application and follow the guided setup.

**For other users, you'll need:**

1. **FFprobe** - Required for video analysis [Install FFmpeg](https://ffmpeg.org/download.html) (includes ffprobe)
2. **FFmpeg** - Required for video processing [Install FFmpeg](https://ffmpeg.org/download.html)
3. **Whisper.cpp** - For AI transcription [Install whisper.cpp](https://github.com/ggerganov/whisper.cpp)

### Installation

1. **Download a prebuilt release**
   Download the latest release from the GitHub releases page for your platform.

2. **Run the application**
   ```bash
   ./atci web

3. **Follow the guided setup**
   - The application will guide you through configuration
   - Configure paths to required tools (or let the app download them automatically on M-series Macs)
   - Set your watch directories (where your videos are stored)
   - Choose and download an AI model for transcription

4. **Open your browser**
   Navigate to [http://localhost:4620](http://localhost:4620)


That's it! Drop video files (MP4, MOV, MKV) into your watch directories and atci will automatically process them.

## ⚙️ Configuration File

The application stores its configuration in a JSON file containing the following properties:

```json
{
  "watch_directories": ["/path/to/videos1", "/path/to/videos2"],
  "whispercli_path": "/path/to/whisper_cli",
  "ffmpeg_path": "/path/to/ffmpeg",
  "ffprobe_path": "/path/to/ffprobe",
  "model_path": "/path/to/model.bin",
  "model_name": "ggml-base",
  "password": ""
}
```

Get the path to the current configuration file with:
```
atci config path
```

Or output the configuration file with:
```
atci config
```

You can also manually set the path to the config file by running any command with the `ATCI_CONFIG_PATH` var set, for example:
```
ATCI_CONFIG_PATH=~/.mycustomconfig.json atci web
```

You can use this to have multiple instances of `atci` running at the same time with different configurations.

**Configuration Properties:**

- **`watch_directories`** (array): List of directories to monitor for new video files
- **`whispercli_path`** (string): Path to the whisper.cpp executable
- **`ffmpeg_path`** (string): Path to the ffmpeg executable  
- **`ffprobe_path`** (string): Path to the ffprobe executable
- **`model_path`** (string): Direct path to a Whisper model file (.bin) (alternative to model_name)
- **`model_name`** (string): Name of a model to use from ~/.atci/models/ (alternative to model_path)
- **`password`** (string): Optional password for all connections. Can be set either in the cookie or via basic auth (no username)

**Notes:**
- Either `model_path` or `model_name` must be specified
- Watch directories cannot be subdirectories of each other
- All paths are validated for existence when the configuration is loaded
- The configuration can be edited through the web interface or by directly editing the file

## 💻 Developer Guide

### Development Setup

1. **Install all dependencies**
   ```bash
   # Backend dependencies
   cargo check
   
   # Frontend dependencies
   cd frontend
   npm install
   cd ..
   ```

2. **Start development servers**
   ```bash
   # In one terminal
   cargo run -- web
   
   # In another terminal - Vite build with watch (constantly builds static files on change)
   cd frontend
   npx vite build --watch
   ```

3. **Run tests**
   ```bash
   cargo test
   ```