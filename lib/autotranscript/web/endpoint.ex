defmodule Autotranscript.Web.Endpoint do
  use Phoenix.Endpoint, otp_app: :autotranscript,
    render_errors: [formats: [html: Autotranscript.Web.ErrorView], layout: false]

  def start_link(opts) do
    atconfig = Keyword.get(opts, :atconfig, %{})
    # Store the atconfig for later use
    Application.put_env(:autotranscript, :atconfig, atconfig)
    super(opts)
  end

  # Serve static files from "/priv/static" at "/"
  plug Plug.Static,
    at: "/",
    from: :autotranscript,
    gzip: false,
    only: ~w(index.css index.js index.html fonts images js favicon.ico robots.txt)

  # Serve MP4 files from watch directory at "/files/"
  # Note: This will be dynamically handled since we moved away from compile-time config
  # The watch directory serving will need to be handled differently

  # Code reloading can be explicitly enabled under the
  # :code_reloader configuration of your endpoint.
  if code_reloading? do
    plug Phoenix.CodeReloader
    plug Phoenix.Ecto.CheckRepoStatus, otp_app: :autotranscript
  end

  # Session configuration
  plug Plug.Session,
    store: :cookie,
    key: "_autotranscript_key",
    signing_salt: "your-signing-salt-here"

  plug Autotranscript.Web.Router
end
