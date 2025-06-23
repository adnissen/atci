defmodule Autotranscript.Web.Endpoint do
  use Phoenix.Endpoint, otp_app: :autotranscript

  # Serve static files from "/priv/static" at "/"
  plug Plug.Static,
    at: "/",
    from: :autotranscript,
    gzip: false,
    only: ~w(css fonts images js favicon.ico robots.txt index.html)

  plug :redirect_root_to_index

  defp redirect_root_to_index(%Plug.Conn{request_path: "/"} = conn, _opts) do
    Phoenix.Controller.redirect(conn, external: "/index.html")
  end
  defp redirect_root_to_index(conn, _opts), do: conn
end
