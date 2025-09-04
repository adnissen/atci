# atci (Andrew's Transcript and Clipping Interface)

atci is an application which provides both a simple command line interface and a responsive web interface for automatically creating video transcripts for all videos in a given set of folders (with whisper) as well as searching through them to easily make gifs, audio, and video clips (via ffmpeg). Transcripts are generated as human readable `.txt` files placed next to the video they correspond to.

The web interface is centered around searching and clipping, especially allowing for quick time adjustments even on mobile. The clip editor is a major area that could use improvement (the regenerate button exists because the state management there has gotten out of hand), but it's still already very functional.


<img width="720" alt="Screenshot 2025-09-03 at 9 08 19 PM" src="https://github.com/user-attachments/assets/e526e1d8-d883-4416-bced-19cd57e72eec" />

The different parts of the application can be run separately. For instance:

* To just watch the set of configured directories for new videos to transcribe:
* * `atci watch`
* To host only the `/api` endpoints (i.e. not the react app, if you want to build your own interface):
* * `atci web api`
* To launch everything, including the directory watcher and the web interface:
* * `atci web all`

All of the data that backs the `/api` routes is also available via the command line interface as json output. Run `atci` with no arguments to display the help dialog which lists subcommands, many of which have subcommands themselves.

## Command line examples

To search (with formatted output):
```
atci search "Special Agent Codebreaker, Jonathan Kingdon" --pretty
File: /Users/andrewnissen/Movies/Decker vs Dracula: Episode 02.mp4
126: 00:05:25.920 --> 00:05:46.060
127:	 Hello? Guys? Is anybody home? Dracula, Dracula, come out wherever you are. It's me, Special Agent Codebreaker, Jonathan Kingdon
```

To search with a path filter, for example if you only want to return files with `Episode 01` in the path:
```
atci search kington -f "Episode 01" --pretty

File: /Users/andrewnissen/Movies/Decker vs Dracula: Episode 01.mp4
174: 00:04:31.060 --> 00:04:36.660
175:	 We already have Jonathan Kington on the site.

180: 00:04:41.500 --> 00:04:43.380
181:	 Kington is a good man.

195: 00:05:06.200 --> 00:05:12.280
196:	 It's the only thing you care about, but luckily, Kington is a good man, and he will find a way to defeat Dracula.
```

Generate a clip (outputs to the `/tmp` directory):
```
atci clip "/Users/andrew.nissen/Movies/Decker vs Dracula: Episode 03.mp4" 04:23 04:30
```

Generate a frame with some text (outputs to the `/tmp` directory):
```
atci frame "/Users/andrew.nissen/Movies/Decker vs Dracula: Episode 03.mp4" 00:01:30.720 "What do you want, Mr President\?" --font-size=36
```

## Quick Start

### Prerequisites

**For M-series (ARM) Mac users**: The software can automatically download and configure ffprobe, ffmpeg, whisper.cpp, and a selected whisper model for you. Just run the application and follow the guided setup in the console.

**For other users, you'll need:**

1. **FFprobe** - Required for video analysis [Install FFmpeg](https://ffmpeg.org/download.html) (includes ffprobe)
2. **FFmpeg** - Required for video processing [Install FFmpeg](https://ffmpeg.org/download.html)
3. **Whisper.cpp** - For AI transcription [Install whisper.cpp](https://github.com/ggerganov/whisper.cpp)
4. **(if building from source) Rust** 
5. **(if doing development) npm around (10.9.3) and node around (22.19.0)**

### Installation
#### From a pre-built release

1. **Download a prebuilt release**
   Download the latest release from the GitHub releases page for your platform (currently the binary is only built for arm-based Macs).
   
   *  For macOS 10.15 or higher you need to remove the file from quarantine.
      You can do this in the Terminal:
      `xattr -dr com.apple.quarantine atci`

   *  Apple Silicon (arm) devices can only run signed files.
      If needed you can ad-hoc sign the downloaded file in the Terminal:
      `xattr -cr atci`
      `codesign -s - atci`

2. **Run the file watcher and web ui**
   ```bash
   ./atci web all
   
#### From source
1.  **Check out the latest source**

   `git clone git@github.com:adnissen/atci.git`

2. **Build the executable**

   `cargo build -r # outputs the atci program to /target/release/`

**Follow the guided setup**
   - The application will guide you through configuration
   - Configure paths to required tools (or let the app download them automatically on M-series Macs)
   - Choose and download a whisper model for transcription
   - Set your watch directories (where your videos are stored)

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

Output the configuration file with:
```
atci config
```

Make changes with:
```
atci config set/unset
```

Changes to the config are reflected immediately in the watch behavior (no server restart required), but will require a browser refresh to reflect in the web ui.

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

## How it works

atci maintains state across runs with various files stored in the `.atci/` directory in the users home folder.
* `.queue` - the queue of files to be processed
* `.currently_processing` - the video file currently being processed
* `.video_info_cache.msgpack` - a cache of video files in the watch directory and metadata about them 

There are a few key components:

* The **file watcher** thread (`watch_for_missing_metadata` in `/src/queue.rs`) runs in a loop every two seconds (started with `atci watch` or `atci web`).
* * This traverses each watch directory and finds video files which don't have an associated .txt file.
* * It organizes these so each directory is added to the queue together in alphabetical order.
* * It adds these to the end of the `.queue` file 
* * * **Note:** This is the only thread to **write** to the `.queue` file.
* * Then, if there is no `.currently_processing` file, we create one with the topmost line from `.queue`.
* * * **Note:** This is the only thread to **write to or create** the `.currently_processing` file.
* * * Clear the top line of `.queue`.
* The **video_processor** thread (`process_queue` in `/src/queue.rs` and the methods in `video_processor.rs`) also runs in a loop every two seconds (started along with `atci watch` or `atci web`).
* * It looks for the presence of a `.currently_processing` file, and, if present, reads it.
* * Go through the subtitle extraction / transcription process. 
* * Update `.video_info_cache.msgpack` with the latest file information for fast retrieval.
* * No matter what, **delete** the `.currently_processing` file at the end of each iteration.
* * * By doing this, the next time the **file watcher** runs, it will take the top line from the queue and create a new `.currently_processing` file, and so on and so forth.
* The **Rocket server** thread handles the web requests. See `web.rs` for a list of routes, the functions for which are located in their respective modules.
