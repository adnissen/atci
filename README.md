# Autotranscript

Autotranscript is a powerful web-based video transcription system that automatically converts your video files to searchable text. Built with Elixir/Phoenix and React, it provides a beautiful interface for managing, viewing, and searching through your video transcripts.

## 🎯 Why Autotranscript?

- **Automatic Processing**: Drop video files in a folder and Autotranscript handles the rest
- **Smart Transcription**: Uses Whisper AI for accurate speech-to-text conversion
- **Existing Subtitles**: Automatically extracts and uses existing subtitles when available
- **Beautiful Web UI**: Modern React interface for browsing and searching transcripts
- **Video Playback**: Built-in video player with timestamp synchronization
- **Full-Text Search**: Quickly find content across all your transcripts
- **Edit Capabilities**: Fix transcription errors directly in the web interface
- **Batch Processing**: Handles multiple videos with queue management

## 🚀 Quick Start for Users

### Prerequisites

1. **Elixir 1.18+** - [Install Elixir](https://elixir-lang.org/install.html)
2. **Node.js 18+** - [Install Node.js](https://nodejs.org/)
3. **FFmpeg** - [Install FFmpeg](https://ffmpeg.org/download.html)
4. **Whisper.cpp** - [Install whisper.cpp](https://github.com/ggerganov/whisper.cpp)
   ```bash
   git clone https://github.com/ggerganov/whisper.cpp
   cd whisper.cpp
   make
   # Download a model (e.g., base model)
   bash ./models/download-ggml-model.sh base
   ```

### Installation

1. **Clone the repository**
   ```bash
   git clone https://github.com/adnissen/autotranscript.git
   cd autotranscript
   ```

2. **Install dependencies**
   ```bash
   mix deps.get
   cd frontend && npm install && cd ..
   ```

3. **Build the frontend**
   ```bash
   cd frontend && npm run build && cd ..
   ```

4. **Start the application**
   ```bash
   mix phx.server
   ```

5. **Open your browser**
   Navigate to [http://localhost:4000](http://localhost:4000)

6. **Configure the application**
   - Click on the configuration page
   - Set your watch directories (where your videos are stored)
   - Set the path to your whisper.cpp executable
   - Set the path to your Whisper model file

That's it! Drop video files (MP4, MOV, MKV) into your watch directories and Autotranscript will automatically process them.

## 💻 Developer Guide

### Project Structure

```
autotranscript/
├── lib/                    # Elixir/Phoenix backend
│   └── autotranscript/
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
   
   # In another terminal - Vite dev server for React hot reload
   cd frontend
   npm run dev
   ```

3. **Run tests**
   ```bash
   # Elixir tests
   mix test
   
   # Frontend tests (if configured)
   cd frontend && npm test
   ```

### Key Components

#### Backend (Elixir/Phoenix)

- **VideoProcessor**: Manages the transcription queue and processing pipeline
- **Transcriber**: Handles the actual transcription using whisper.cpp
- **ConfigManager**: Manages application configuration
- **MetaFileHandler**: Handles metadata storage for transcripts
- **Web Controllers**: RESTful API endpoints for the frontend

#### Frontend (React/TypeScript)

- **HomePage**: Main transcript listing with search
- **TranscriptView**: Individual transcript viewer with video player
- **ConfigPage**: Configuration management interface
- **FileCard**: Video file representation with status indicators

### API Endpoints

- `GET /api/transcripts` - List all transcripts
- `GET /api/transcripts/:id` - Get specific transcript
- `PUT /api/transcripts/:id` - Update transcript text
- `POST /api/transcripts/reprocess` - Reprocess a video
- `GET /api/config` - Get configuration
- `PUT /api/config` - Update configuration
- `GET /api/queue` - Get processing queue status

### Adding New Features

1. **Backend Feature**
   ```elixir
   # Add new module in lib/autotranscript/
   # Add route in lib/autotranscript/web/router.ex
   # Add controller action in appropriate controller
   ```

2. **Frontend Feature**
   ```typescript
   // Add component in frontend/src/components/
   // Update or create new page in frontend/src/pages/
   // Add API calls as needed
   ```

### Building for Production

```bash
# Build frontend assets
cd frontend && npm run build && cd ..

# Create production release
MIX_ENV=prod mix release

# Run the release
_build/prod/rel/autotranscript/bin/autotranscript start
```

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

## 📝 License

This project is licensed under the MIT License - see the LICENSE file for details.