defmodule Atci.Web.Router do
  use Phoenix.Router, helpers: false

  import Plug.Conn
  import Phoenix.Controller

  pipeline :browser do
    plug(:accepts, ["html"])
    plug(:fetch_session)
    plug(:put_root_layout, html: {Atci.Web.Layouts, :root})
    plug(:protect_from_forgery)
    plug(:put_secure_browser_headers)

    plug(Plug.Parsers,
      parsers: [:urlencoded, :multipart, :json],
      pass: ["*/*"],
      json_decoder: Phoenix.json_library()
    )

    plug(Atci.Web.Plugs.NonlocalPassword)
  end

  pipeline :api do
    plug(:accepts, ["json"])

    plug(Plug.Parsers,
      parsers: [:urlencoded, :multipart, :json],
      pass: ["*/*"],
      json_decoder: Phoenix.json_library()
    )

    plug(Atci.Web.Plugs.NonlocalPassword)
  end

  pipeline :config do
    plug(:accepts, ["html", "json"])
    plug(:fetch_session)
    plug(:put_root_layout, html: {Atci.Web.Layouts, :root})
    plug(:put_secure_browser_headers)

    plug(Plug.Parsers,
      parsers: [:urlencoded, :multipart, :json],
      pass: ["*/*"],
      json_decoder: Phoenix.json_library()
    )

    # Note: No CSRF protection for config endpoints

    plug(Atci.Web.Plugs.NonlocalPassword)
  end

  scope "/", Atci.Web do
    pipe_through(:browser)

    get("/", PageController, :index)

    get("/app", TranscriptController, :index)
    get("/app/*path", TranscriptController, :index)
    get("/transcripts/:filename", TranscriptController, :show)
    post("/transcripts/:filename/replace", TranscriptController, :replace_transcript)
    post("/regenerate/:filename", TranscriptController, :regenerate)
    post("/regenerate-meta/:filename", TranscriptController, :regenerate_meta)
    post("/transcripts/:filename/partial_reprocess", TranscriptController, :partial_reprocess)
    post("/transcripts/:filename/set_line", TranscriptController, :set_line)
    post("/transcripts/:filename/rename", TranscriptController, :rename)

    # Meta file routes
    get("/transcripts/:filename/meta", TranscriptController, :get_meta_file)
    post("/transcripts/:filename/meta", TranscriptController, :set_meta_file)

    get("/grep/:text", TranscriptController, :grep)
    get("/player/:filename", TranscriptController, :player)
    get("/clip_player/:filename", TranscriptController, :clip_player)
    get("/frame/:filename/:time", TranscriptController, :frame_at_time)

    get("/queue", TranscriptController, :queue)
    get("/files", TranscriptController, :files)
    get("/sources", TranscriptController, :sources)
    get("/random_frame", TranscriptController, :random_frame)
    get("/clip", TranscriptController, :clip)
    get("/watch_directory", TranscriptController, :watch_directory)
    get("/watch_directories", TranscriptController, :watch_directories)
  end

  scope "/", Atci.Web do
    pipe_through(:config)

    # Configuration endpoints (no CSRF protection, accepts both HTML and JSON)
    get("/config", ConfigController, :show)
    post("/config", ConfigController, :update)
  end

  scope "/api", Atci.Web do
    pipe_through(:api)

    # Model management endpoints
    get("/models", ModelController, :list)
    post("/models/download", ModelController, :download)

    # FFmpeg management endpoints
    get("/ffmpeg/tools", FFmpegController, :list)
    post("/ffmpeg/download", FFmpegController, :download)
    post("/ffmpeg/use-downloaded", FFmpegController, :use_downloaded)
    post("/ffmpeg/use-auto-detection", FFmpegController, :use_auto_detection)

    # Whisper-CLI management endpoints
    get("/whisper-cli/tools", WhisperCliController, :list)
    post("/whisper-cli/download", WhisperCliController, :download)
    post("/whisper-cli/use-downloaded", WhisperCliController, :use_downloaded)
    post("/whisper-cli/use-auto-detection", WhisperCliController, :use_auto_detection)

    # Queue management endpoints
    get("/queue/status", QueueController, :status)
    delete("/queue/remove", QueueController, :remove_job)
    post("/queue/reorder", QueueController, :reorder)
    delete("/queue/cancel-current", QueueController, :cancel_current)
  end
end
