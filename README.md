# atci

atci is a powerful web-based video transcription system that automatically converts your video files to searchable text. Built with Elixir/Phoenix and React, it provides a beautiful interface for managing, viewing, and searching through your video transcripts.

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
   ./atci
   ```

3. **Open your browser**
   Navigate to [http://localhost:4000](http://localhost:4000)

4. **Follow the guided setup**
   - The application will guide you through configuration
   - Set your watch directories (where your videos are stored)
   - Configure paths to required tools (or let the app download them automatically on M-series Macs)
   - Choose and download an AI model for transcription

That's it! Drop video files (MP4, MOV, MKV) into your watch directories and atci will automatically process them.

## ⚙️ Configuration File (.atciconfig)

The application stores its configuration in a `.atciconfig` file in your home directory. This JSON file contains the following properties:

```json
{
  "watch_directories": ["/path/to/videos1", "/path/to/videos2"],
  "whispercli_path": "/path/to/whisper_cli",
  "ffmpeg_path": "/path/to/ffmpeg",
  "ffprobe_path": "/path/to/ffprobe",
  "model_path": "/path/to/model.bin",
  "model_name": "ggml-base",
  "nonlocal_password": ""
```

**Configuration Properties:**

- **`watch_directories`** (array): List of directories to monitor for new video files
- **`whispercli_path`** (string): Path to the whisper.cpp executable
- **`ffmpeg_path`** (string): Path to the ffmpeg executable  
- **`ffprobe_path`** (string): Path to the ffprobe executable
- **`model_path`** (string): Direct path to a Whisper model file (.bin) (alternative to model_name)
- **`model_name`** (string): Name of a model to use from ~/.atci/models/ (alternative to model_path)
- **`nonlocal_password`** (string): Optional password for connections with a non-localhost origin

**Notes:**
- Either `model_path` or `model_name` must be specified
- Watch directories cannot be subdirectories of each other
- All paths are validated for existence when the configuration is loaded
- The configuration can be edited through the web interface or by directly editing the file

## 💻 Developer Guide

### Project Structure

```
atci/
├── lib/                    # Elixir/Phoenix backend
│   └── atci/
│       ├── web/           # Phoenix web layer
│       ├── transcriber/   # Core transcription logic
│       └── *.ex           # Various managers and helpers
├── frontend/              # React frontend
│   ├── src/
│   │   ├── components/    # React components
│   │   └── pages/        # Page components
│   └── package.json
├── test/                  # Elixir tests
└── mix.exs               # Elixir project file
```

### Development Setup

1. **Install all dependencies**
   ```bash
   # Backend dependencies
   mix deps.get
   
   # Frontend dependencies
   cd frontend
   npm install
   cd ..
   ```

2. **Start development servers**
   ```bash
   # In one terminal - Phoenix server with live reload
   mix phx.server
   
   # In another terminal - Vite build with watch (constantly builds static files on change)
   cd frontend
   npx vite build --watch
   ```

3. **Run tests**
   ```bash
   # Elixir tests
   mix test
   ```

### Key Components

#### Backend (Elixir/Phoenix)

- **VideoProcessor**: Manages the transcription queue and processing pipeline
- **Transcriber**: Handles the actual transcription using whisper.cpp
- **ConfigManager**: Manages application configuration and .atciconfig file
- **MetaFileHandler**: Handles metadata storage for transcripts
- **FFmpegManager**: Manages FFmpeg and FFprobe binaries, including downloading them for different platforms
- **ModelManager**: Manages Whisper models, including listing available models and downloading them from Hugging Face
- **WhisperCLIManager**: Manages whisper-cli binaries, including downloading them for different platforms
- **VideoInfoCache**: Maintains a cache of video file information and updates when videos are processed
- **Web Controllers**: RESTful API endpoints for the frontend

#### Frontend (React/TypeScript)

- **HomePage**: Main transcript listing with search
- **TranscriptView**: Individual transcript viewer with video player
- **ConfigPage**: Configuration management interface
- **FileCard**: Video file representation with status indicators

### API Endpoints

#### Main Routes
- `GET /` - Main application entry point
- `GET /app` - React application
- `GET /app/*path` - React application catch-all

#### Transcript Routes
- `GET /transcripts/:filename` - Get specific transcript content
- `POST /transcripts/:filename/replace` - Replace entire transcript
- `POST /transcripts/:filename/partial_reprocess` - Reprocess part of transcript
- `POST /transcripts/:filename/set_line` - Update specific line in transcript
- `POST /transcripts/:filename/rename` - Rename transcript and associated files
- `GET /transcripts/:filename/meta` - Get transcript metadata
- `POST /transcripts/:filename/meta` - Set transcript metadata

#### Processing Routes
- `POST /regenerate/:filename` - Reprocess entire video
- `POST /regenerate-meta/:filename` - Regenerate metadata
- `GET /queue` - Get processing queue status

#### Search and Browse Routes
- `GET /grep/:text` - Search across all transcripts
- `GET /files` - List all video files with metadata
- `GET /sources` - Get unique transcript sources (models used)
- `GET /watch_directory` - Get watch directory info
- `GET /watch_directories` - Get all watch directories

#### Video Player Routes
- `GET /player/:filename` - Video player interface
- `GET /clip_player/:filename` - Clip player interface
- `GET /frame/:filename/:time` - Get video frame at specific time
- `GET /random_frame` - Get random frame from random video
- `GET /clip` - Generate video/audio clip

#### Configuration Routes
- `GET /config` - Get configuration
- `POST /config` - Update configuration

#### API Routes (JSON)
- `GET /api/models` - List available Whisper models
- `POST /api/models/download` - Download a Whisper model
- `GET /api/ffmpeg/tools` - List FFmpeg/FFprobe tools status
- `POST /api/ffmpeg/download` - Download FFmpeg tools
- `POST /api/ffmpeg/use-downloaded` - Use downloaded FFmpeg tools
- `POST /api/ffmpeg/use-auto-detection` - Use auto-detected FFmpeg tools
- `GET /api/whisper-cli/tools` - List Whisper CLI tools status
- `POST /api/whisper-cli/download` - Download Whisper CLI tools
- `POST /api/whisper-cli/use-downloaded` - Use downloaded Whisper CLI tools
- `POST /api/whisper-cli/use-auto-detection` - Use auto-detected Whisper CLI tools

### Building for Production

```bash
# Build frontend assets
cd frontend && npx vite build && cd ..

# Create production release
MIX_ENV=prod mix release

# Run the release
_build/prod/rel/atci/bin/atci start
```