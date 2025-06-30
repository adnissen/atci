# Getting Started with Autotranscript

## ⚠️ Configuration Required

**Autotranscript will not start without proper configuration.** You must create a `.atconfig` file before using the application.

## Quick Start

### Step 1: Create Configuration File

Create a `.atconfig` file in your working directory with the following content:

```
watch_directory=/absolute/path/to/your/videos
whispercli_path=/absolute/path/to/whisper-cli
model_path=/absolute/path/to/your/whisper.cpp/model.bin
```

**Replace the paths with your actual paths:**

- `watch_directory`: Directory containing your MP4 video files
- `whispercli_path`: Full path to the whisper-cli executable
- `model_path`: Full path to your Whisper model file (.bin)

### Step 2: Verify Paths

Make sure all paths exist and are accessible:

```bash
# Check if directories exist
ls -la /path/to/your/videos
ls -la /path/to/whisper-cli
ls -la /path/to/your/whisper.cpp/model.bin
```

### Step 3: Start the Application

```bash
# Start the Elixir application
mix phx.server
```

### Step 4: Access Web Interface

Open your browser to `http://localhost:4000`

If your configuration is valid, you'll see the file list. If not, you'll be prompted to configure the application through the web interface.

## Alternative: Web Configuration

If you prefer, you can start with an empty or missing `.atconfig` file. The web interface will automatically prompt you to configure the application and will create the `.atconfig` file for you.

## Configuration File Locations

The application looks for `.atconfig` files in this order:

1. **Current directory** (where you run the command) - **Recommended**
2. **Home directory** (`~/.atconfig`) - For global configuration

## Troubleshooting

### "Configuration Required" Error

If you see a configuration error:

1. Check that `.atconfig` exists in your current directory or home directory
2. Verify all three required settings are present: `watch_directory`, `whispercli_path`, `model_path`
3. Ensure all paths are absolute and accessible
4. Check that there are no typos in the key names

### File Format Issues

- Use `key=value` format (no spaces around the `=`)
- Use absolute paths (start with `/`)
- Comments start with `#`
- Empty lines are ignored

### Example Valid Configuration

```
# Autotranscript Configuration
watch_directory=/home/user/Videos/Transcripts
whispercli_path=/usr/local/bin/whisper-cli
model_path=/home/user/whisper.cpp/models/ggml-base.en.bin
```

## Next Steps

Once configured, Autotranscript will:

1. Monitor your `watch_directory` for new MP4 files
2. Automatically transcribe videos using Whisper
3. Save transcripts as `.txt` files in the same directory
4. Provide a web interface to view and search transcripts

For more details, see the main [README.md](README.md).