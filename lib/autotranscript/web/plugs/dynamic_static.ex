defmodule Autotranscript.Web.Plugs.DynamicStatic do
  @moduledoc """
  A plug that dynamically serves static files from the watch directory
  configured in ConfigManager.
  """

  import Plug.Conn
  require Logger

  def init(opts), do: opts

  def call(%Plug.Conn{path_info: ["files" | path]} = conn, _opts) do
    # Get the watch directory from ConfigManager
    case Autotranscript.ConfigManager.get_config_value("watch_directory") do
      nil ->
        Logger.warning("Watch directory not configured, cannot serve files")
        conn
        |> send_resp(404, "Not Found")
        |> halt()

      watch_directory ->
        file_path = Path.join([watch_directory] ++ path)

        if File.exists?(file_path) and not File.dir?(file_path) do
          # Serve the file
          conn
          |> put_resp_content_type(MIME.from_path(file_path))
          |> send_file(200, file_path)
          |> halt()
        else
          conn
          |> send_resp(404, "Not Found")
          |> halt()
        end
    end
  end

  def call(conn, _opts), do: conn
end