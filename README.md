# atci (Andrew's Transcript and Clipping Interface)

atci is an application which provides both a simple command line interface and a responsive web interface for automatically creating video transcripts for all videos in a given set of folders (with whisper) as well as searching through them to easily make gifs, audio, and video clips (via ffmpeg). Transcripts are generated as human readable `.txt` files placed next to the video they correspond to.

The web interface is centered around searching and clipping, especially allowing for quick time adjustments even on mobile. The clip editor is a major area that could use improvement (the regenerate button exists because the state management there has gotten out of hand), but it's still already very functional.

<img width="1512" height="786" alt="Screenshot 2025-09-07 at 2 35 01 PM" src="https://github.com/user-attachments/assets/c8f883e3-3894-4ae8-acdb-484c67ce145a" />
<img width="250" height="784" alt="Screenshot 2025-09-07 at 2 36 25 PM" src="https://github.com/user-attachments/assets/9e8819d4-3a7e-41f2-8964-524fafe01e36" />
<img width="250" height="784" alt="Screenshot 2025-09-07 at 2 36 11 PM" src="https://github.com/user-attachments/assets/15cd0b9a-f253-4db9-8039-6796815e45c4" />


The different parts of the application can be run separately. For instance:

* To just watch the set of configured directories for new videos to transcribe:
* * `atci watch`
* To host only the `/api` endpoints (i.e. not the react app, if you want to build your own interface):
* * `atci web api`
* To launch everything, including the directory watcher and the web interface:
* * `atci web all`

All of the data that backs the `/api` routes is also available via the command line interface. Output is human-readable by default, but can be changed to JSON with the `--json` flag. Run `atci` with no arguments to display the help dialog which lists subcommands, many of which have subcommands themselves.

## Command line examples

To search (default formatted output):
```
atci search "Special Agent Codebreaker, Jonathan Kingdon"
File: /Users/andrewnissen/Movies/Decker vs Dracula: Episode 02.mp4
126: 00:05:25.920 --> 00:05:46.060
127:	 Hello? Guys? Is anybody home? Dracula, Dracula, come out wherever you are. It's me, Special Agent Codebreaker, Jonathan Kingdon
```

To search with a path filter, for example if you only want to return files with `Episode 01` in the path:
```
atci search kington -f "Episode 01"

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

Disable extracting subtitles:
```
atci config set allow_subtitles false
```

Disable whisper processing:
```
atci config set allow_whisper false
```

Set a command to run when processing completes successfully (the video file path is sent to the command):
```
atci config set processing_success_command "xargs terminal-notifier -title 'Processing Complete' -message"
```

Download a live stream in 60-second parts (configurable with `stream_chunk_size`):
```
atci streamdl "my-livestream" "https://example.com/stream/playlist.m3u8"
```

By default, the first subtitle track is used if subtitles are enabeld. Sometimes, you might want to use a different one, or use a different whisper model than the currently configured one. You can perform an interactive regeneration, which allows you to select how to process it:
```
atci transcripts regenerate -i /path/to/file.mp4

=== File Information ===
File: /path/to/file.mp4
Size: 1591.80 MB
Duration: 01:26:21

=== Processing Options ===
Choose a processing method:
> Subtitles: English (2)
  Subtitles: English (3)
  Subtitles: English (4)
  Subtitles: French (5)
  Whisper Model: ggml-large-v3-turbo-q5_0
  Whisper Model: ggml-large-v3-turbo-q8_0 (currently configured)
  Cancel
```

## Quick Start

### Prerequisites

**For Mac and Windows users**: The software can automatically download and configure ffprobe, ffmpeg, whisper.cpp, and a selected whisper model for you. Just run the application and follow the guided setup in the console.

**For other users, you'll need:**

1. **FFprobe** - Required for video analysis [Install FFmpeg](https://ffmpeg.org/download.html) (includes ffprobe)
2. **FFmpeg** - Required for video processing [Install FFmpeg](https://ffmpeg.org/download.html)
3. **Whisper.cpp** - For AI transcription [Install whisper.cpp](https://github.com/ggerganov/whisper.cpp)
4. **(if building from source) Rust** 
5. **(if doing development) npm around (10.9.3) and node around (22.19.0)**

### Installation
#### From a pre-built release

1. **Download a prebuilt release**
   Download the latest release from the GitHub releases page for your platform.
   
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
  "model_name": "ggml-base",
  "password": "",
  "allow_whisper": true,
  "allow_subtitles": true,
  "processing_success_command": "",
  "processing_failure_command": "",
  "stream_chunk_size": 60
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

## Working with Stream Parts and Partial Files

When using the `streamdl` command, atci automatically splits streams into manageable parts based on your `stream_chunk_size` configuration (default: 60 seconds). These files are saved with a naming pattern like:

```
stream-name.20250122_143055.part1.ts
stream-name.20250122_143055.part2.ts
stream-name.20250122_143055.part3.ts
...
```

### Processing Partial Files

atci can process any video file, including these partial stream files. Simply use any atci command that accepts video file paths:

**Search within a specific part:**
```bash
atci search "keyword" -f "part3"
```

**Generate clips from partial files:**
```bash
atci clip "/path/to/stream-name.20250122_143055.part2.ts" 30 45
```

**View transcripts for a specific part:**
```bash
atci transcripts get "/path/to/stream-name.20250122_143055.part1.ts"
```

**Process parts interactively:**
```bash
atci transcripts regenerate -i "/path/to/stream-name.20250122_143055.part4.ts"
```

Each part is treated as an independent video file with its own transcript and metadata. This allows you to:
- Search across all parts simultaneously
- Focus on specific time segments by targeting particular parts
- Process long streams without memory constraints
- Resume processing if interrupted

The watch system automatically detects and processes new stream parts as they're created during live downloads.

**Configuration Properties:**

- **`watch_directories`** (array): List of directories to monitor for new video files
- **`whispercli_path`** (string): Path to the whisper.cpp executable
- **`ffmpeg_path`** (string): Path to the ffmpeg executable  
- **`ffprobe_path`** (string): Path to the ffprobe executable
- **`model_name`** (string): Name of a model to use from ~/.atci/models/
- **`password`** (string): Optional password for all connections. Can be set either in the cookie or via basic auth (no username)
- **`allow_whisper`** (boolean): Enable/disable Whisper transcription processing (default: true)
- **`allow_subtitles`** (boolean): Enable/disable subtitle extraction from video files (default: true)
- **`processing_success_command`** (string): Shell command to run when video processing completes successfully. The video file path is sent to the command's stdin
- **`processing_failure_command`** (string): Shell command to run when video processing fails. The video file path is sent to the command's stdin
- **`stream_chunk_size`** (number): Duration in seconds for splitting streams when using the `streamdl` command (default: 60)

**Notes:**
- `model_name` must be specified for transcription to work
- Watch directories cannot be subdirectories of each other

## How it works

atci maintains state across runs with a SQLite database stored in the `.atci/` directory in the users home folder.
* `queue` table - stores the queue of files to be processed
* `currently_processing` table - tracks the video file currently being processed
* `video_info` table - contains video files in the watch directory and metadata about them. Nothing is stored here that isn't stored in the transcript itself or available via system apis. This is purely for fast access, not actual storage.

There are a few key components:

* The **file watcher** thread (`watch_for_missing_metadata` in `/src/queue.rs`) runs in a loop every two seconds (started with `atci watch` or `atci web`).
* * This traverses each watch directory and finds video files which don't have an associated .txt file.
* * It organizes these so each directory is added to the queue together in alphabetical order.
* * It adds these to the end of the `queue` table 
* * Then, if there is no entry in the `currently_processing` table, we create one with the topmost entry from the `queue` table.
* * * **Note:** This is the only thread to **write to or create** entries in the `currently_processing` table.
* * * Clear the top entry from the `queue` table.
* The **video_processor** thread (`process_queue` in `/src/queue.rs` and the methods in `video_processor.rs`) also runs in a loop every two seconds (started along with `atci watch` or `atci web`).
* * It looks for entries in the `currently_processing` table, and, if present, reads the file path.
* * Go through the subtitle extraction / transcription process. 
* * Update the `video_info` table with the latest file information for fast retrieval.
* * No matter what, **delete** the entry from the `currently_processing` table at the end of each iteration.
* * * By doing this, the next time the **file watcher** runs, it will take the top entry from the queue table and create a new entry in the `currently_processing` table, and so on and so forth.
* The **Rocket server** thread handles the web requests. See `web.rs` for a list of routes, the functions for which are located in their respective modules.
