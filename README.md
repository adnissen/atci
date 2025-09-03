# atci (Andrew's Transcript and Clipping Interface)

atci is an application which provides both a simple command line interface and a responsive web interface for automatically creating video transcripts for all videos in a given set of folders (with whisper) as well as searching through them to easily make gifs, audio, and video clips (via ffmpeg).

The different parts of the application can be run separately. For instance:

* To just watch the set of configured directories for new videos to transcribe:
* * `atci watch`
* To host only the `/api` endpoints (i.e. not the react app, if you want to build your own interface):
* * `atci web api`
* To launch everything, including the directory watcher and the web interface:
* * `atci web all`

All of the data that backs the `/api` routes is also available via the command line interface as json output. Run `atci` with no arguments to display the help dialog which lists subcommands, many of which have subcommands themselves.

## Quick Start for Users

### Prerequisites

**For M-series (ARM) Mac users**: The software can automatically download and configure ffprobe, ffmpeg, whisper.cpp, and the AI model for you! Just run the application and follow the guided setup.

**For other users, you'll need:**

1. **FFprobe** - Required for video analysis [Install FFmpeg](https://ffmpeg.org/download.html) (includes ffprobe)
2. **FFmpeg** - Required for video processing [Install FFmpeg](https://ffmpeg.org/download.html)
3. **Whisper.cpp** - For AI transcription [Install whisper.cpp](https://github.com/ggerganov/whisper.cpp)

### Installation
#### From a pre-built release

1. **Download a prebuilt release**
   Download the latest release from the GitHub releases page for your platform.

2. **Run the application**
   ```bash
   ./atci web
   
#### From source
1.  **Check out the latest source**
   `git clone git@github.com:adnissen/atci.git`

2. **Start the rust server**
   `cargo run -- web all`

3. **(Optional, for frontend development)**
   ```
   cd frontend
   npm install
   npx vite build --watch # watch for changes to the frontend files and build
   ```

**Follow the guided setup**
   - The application will guide you through configuration
   - Configure paths to required tools (or let the app download them automatically on M-series Macs)
   - Set your watch directories (where your videos are stored)
   - Choose and download an AI model for transcription

**Open your browser**
   Navigate to [http://localhost:4620](http://localhost:4620)

That's it! Drop video files (MP4, MOV, MKV) into your watch directories and atci will automatically process them.

## Configuration File

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
   ```bash
   # In one terminal
   cargo run -- web
   
   # In another terminal - Vite build with watch (constantly builds static files on change)
   cd frontend
   npx vite build --watch
   ```