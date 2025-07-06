defmodule Autotranscript.Web.Router do
  use Phoenix.Router, helpers: false

  import Plug.Conn
  import Phoenix.Controller

  pipeline :browser do
    plug :accepts, ["html"]
    plug :fetch_session
    plug :put_root_layout, html: {Autotranscript.Web.Layouts, :root}
    plug :protect_from_forgery
    plug :put_secure_browser_headers
    plug Plug.Parsers,
      parsers: [:urlencoded, :multipart, :json],
      pass: ["*/*"],
      json_decoder: Phoenix.json_library()
  end

  pipeline :api do
    plug :accepts, ["json"]
    plug Plug.Parsers,
      parsers: [:urlencoded, :multipart, :json],
      pass: ["*/*"],
      json_decoder: Phoenix.json_library()
  end

  pipeline :config do
    plug :accepts, ["html", "json"]
    plug :fetch_session
    plug :put_root_layout, html: {Autotranscript.Web.Layouts, :root}
    plug :put_secure_browser_headers
    plug Plug.Parsers,
      parsers: [:urlencoded, :multipart, :json],
      pass: ["*/*"],
      json_decoder: Phoenix.json_library()
    # Note: No CSRF protection for config endpoints
  end

  scope "/", Autotranscript.Web do
    pipe_through :browser

    get "/", PageController, :index

    get "/app", TranscriptController, :index
    get "/transcripts/:filename", TranscriptController, :show
    post "/transcripts/:filename/replace", TranscriptController, :replace_transcript
    post "/transcripts/:filename/regenerate", TranscriptController, :regenerate
    post "/transcripts/:filename/regenerate_meta", TranscriptController, :regenerate_meta
    post "/transcripts/:filename/partial_reprocess", TranscriptController, :partial_reprocess
    post "/transcripts/:filename/set_line", TranscriptController, :set_line

    get "/grep/:text", TranscriptController, :grep
    get "/player/:filename", TranscriptController, :player
    get "/frame/:filename/:time", TranscriptController, :frame_at_time

    get "/queue", TranscriptController, :queue
    get "/files", TranscriptController, :files
    get "/random_frame", TranscriptController, :random_frame
    get "/clip", TranscriptController, :clip
    get "/watch_directory", TranscriptController, :watch_directory
    post "/watch_directory", TranscriptController, :set_watch_directory
  end

  scope "/", Autotranscript.Web do
    pipe_through :config

    # Configuration endpoints (no CSRF protection, accepts both HTML and JSON)
    get "/config", ConfigController, :show
    post "/config", ConfigController, :update
  end
end
