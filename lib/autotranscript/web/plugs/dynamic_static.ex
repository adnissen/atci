defmodule Autotranscript.Web.Plugs.DynamicStatic do
  @moduledoc """
  A plug that dynamically serves static files from the watch directory
  configured in ConfigManager.
  """
alias Autotranscript.PathHelper
  import Plug.Conn
  require Logger
  require Record
  Record.defrecordp(:file_info, Record.extract(:file_info, from_lib: "kernel/include/file.hrl"))

  def init(opts), do: opts

  def call(%Plug.Conn{path_info: ["files" | path]} = conn, _opts) when length(path) > 0 do
    # Get the watch directories from ConfigManager
    case Autotranscript.ConfigManager.get_config_value("watch_directories") do
      nil ->
        Logger.warning("Watch directories not configured, cannot serve files")
        conn
        |> send_resp(404, "Not Found")
        |> halt()

      [] ->
        Logger.warning("Watch directories list is empty, cannot serve files")
        conn
        |> send_resp(404, "Not Found")
        |> halt()

      watch_directories ->
        # Try to find the file in each watch directory
        case find_file_in_watch_directories(watch_directories, path) do
          {found_path, file_info} ->
            # Get range header if present
            range = get_req_header(conn, "range")

            # Serve the file with range support
            conn
            |> put_resp_content_type(get_content_type(found_path))
            |> put_resp_header("accept-ranges", "bytes")
            |> serve_range(file_info, found_path, range)

          nil ->
            conn
            |> send_resp(404, "Not Found")
            |> halt()
        end
    end
  end

  def call(conn, _opts), do: conn

  defp serve_range(conn, file_info, path, [range]) do
    file_info(size: file_size) = file_info

    with %{"bytes" => bytes} <- Plug.Conn.Utils.params(range),
         {range_start, range_end} <- start_and_end(bytes, file_size) do
      send_range(conn, path, range_start, range_end, file_size)
    else
      _ -> send_entire_file(conn, path)
    end
  end

  defp serve_range(conn, _file_info, path, _range) do
    send_entire_file(conn, path)
  end

  defp start_and_end("-" <> rest, file_size) do
    case Integer.parse(rest) do
      {last, ""} when last > 0 and last <= file_size -> {file_size - last, file_size - 1}
      _ -> :error
    end
  end

  defp start_and_end(range, file_size) do
    case Integer.parse(range) do
      {first, "-"} when first >= 0 ->
        {first, file_size - 1}

      {first, "-" <> rest} when first >= 0 ->
        case Integer.parse(rest) do
          {last, ""} when last >= first -> {first, min(last, file_size - 1)}
          _ -> :error
        end

      _ ->
        :error
    end
  end

  defp send_range(conn, path, 0, range_end, file_size) when range_end == file_size - 1 do
    send_entire_file(conn, path)
  end

  defp send_range(conn, path, range_start, range_end, file_size) do
    length = range_end - range_start + 1

    conn
    |> put_resp_header("content-range", "bytes #{range_start}-#{range_end}/#{file_size}")
    |> send_file(206, path, range_start, length)
    |> halt()
  end

  defp send_entire_file(conn, path) do
    conn
    |> send_file(200, path)
    |> halt()
  end

  defp get_content_type(file_path) do
    file_path
    |> Path.extname()
    |> String.downcase()
    |> case do
      ".mp4" -> "video/mp4"
      ".mov" -> "video/quicktime"
      ".mp3" -> "audio/mpeg"
      ".txt" -> "text/plain"
      ".json" -> "application/json"
      ".pdf" -> "application/pdf"
      ".jpg" -> "image/jpeg"
      ".jpeg" -> "image/jpeg"
      ".png" -> "image/png"
      ".gif" -> "image/gif"
      ".webm" -> "video/webm"
      ".avi" -> "video/x-msvideo"
      ".mkv" -> "video/x-matroska"
      _ -> "application/octet-stream"
    end
  end

  # Helper function to find a file in any of the watch directories
  defp find_file_in_watch_directories(watch_directories, path) do
    Enum.find_value(watch_directories, fn watch_directory ->
      file_path = Path.join([watch_directory] ++ path)
      decoded_path = Autotranscript.Web.TranscriptController.decode_filename(file_path)
      
      # If no extension, try all video extensions
      paths_to_try = if Path.extname(decoded_path) == "" do
        Enum.map(PathHelper.video_extensions(), fn ext ->
          decoded_path <> "." <> ext
        end)
      else
        [decoded_path]
      end

      # Try each possible path
      Enum.find_value(paths_to_try, fn path ->
        case :prim_file.read_file_info(path) do
          {:ok, file_info(type: :regular) = file_info} -> {path, file_info}
          _ -> nil
        end
      end)
    end)
  end
end
