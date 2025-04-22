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

## Configuration

Configure the following in `config/config.exs`:

```elixir
config :autotranscript,
  watch_directory: "/path/to/your/videos",
  whispercli_path: "/path/to/whisper-cli",
  model_path: "/path/to/your/whisper.cpp/model.bin"
```