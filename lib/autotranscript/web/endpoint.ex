defmodule Autotranscript.Web.Endpoint do
  use Phoenix.Endpoint, otp_app: :autotranscript,
    render_errors: [formats: [html: Autotranscript.ErrorView], layout: false]

  # Serve static files from "/priv/static" at "/"
  plug Plug.Static,
    at: "/",
    from: :autotranscript,
    gzip: false,
    only: ~w(css fonts images js favicon.ico robots.txt)

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
