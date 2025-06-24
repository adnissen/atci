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
  end

  pipeline :api do
    plug :accepts, ["json"]
  end

  scope "/", Autotranscript.Web do
    pipe_through :browser

    get "/", PageController, :index
    get "/transcripts", TranscriptController, :index
    get "/queue", TranscriptController, :queue
    get "/files", TranscriptController, :files
    get "/random_frame", TranscriptController, :random_frame
    get "/transcripts/:filename", TranscriptController, :show
    get "/transcripts/grep/:text", TranscriptController, :grep
    post "/transcripts/regenerate/:filename", TranscriptController, :regenerate
    get "/player/:filename", TranscriptController, :player
  end
end
