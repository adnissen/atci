defmodule Autotranscript.Web.Plugs.DynamicStatic do
  @moduledoc """
  A plug that serves static files from a runtime-configured directory.
  """

  import Plug.Conn
  alias Autotranscript.Config

  def init(opts) do
    %{
      at: Keyword.get(opts, :at, "/"),
      gzip: Keyword.get(opts, :gzip, false)
    }
  end

  def call(conn, opts) do
    case Config.get(:watch_directory) do
      nil ->
        conn
      watch_directory ->
        # Only process requests that start with the configured path
        if String.starts_with?(conn.request_path, opts.at) do
          serve_from_directory(conn, watch_directory, opts)
        else
          conn
        end
    end
  end

  defp serve_from_directory(conn, directory, opts) do
    # Remove the mount point from the path
    relative_path = String.replace_leading(conn.request_path, opts.at, "")
    file_path = Path.join(directory, relative_path)

    # Security check: ensure the path doesn't escape the directory
    case Path.safe_relative_to(file_path, directory) do
      nil ->
        # Path tries to escape the directory
        conn
        |> send_resp(403, "Forbidden")
        |> halt()
      _safe_path ->
        case File.stat(file_path) do
          {:ok, %File.Stat{type: :regular}} ->
            serve_file(conn, file_path, opts)
          _other ->
            conn
        end
    end
  end

  defp serve_file(conn, file_path, _opts) do
    # Get MIME type
    content_type = MIME.from_path(file_path)
    
    case File.read(file_path) do
      {:ok, content} ->
        conn
        |> put_resp_content_type(content_type)
        |> send_resp(200, content)
        |> halt()
      {:error, _reason} ->
        conn
        |> send_resp(404, "Not Found")
        |> halt()
    end
  end
end