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
    get "/transcripts/:filename", TranscriptController, :show
    get "/transcripts/grep/:text", TranscriptController, :grep
  end
end
