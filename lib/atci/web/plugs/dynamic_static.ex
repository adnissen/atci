defmodule Atci.Web.Plugs.DynamicStatic do
  @moduledoc """
  A plug that dynamically serves static files from the watch directory
  configured in ConfigManager.
  """
  alias Atci.PathHelper
  import Plug.Conn
  require Logger
  require Record
  Record.defrecordp(:file_info, Record.extract(:file_info, from_lib: "kernel/include/file.hrl"))

  def init(opts), do: opts

  def call(%Plug.Conn{path_info: ["files" | path]} = conn, _opts) when length(path) > 0 do
    # Get the watch directories from ConfigManager
    watch_directories = Atci.ConfigManager.get_config_value("watch_directories") || []

    if Enum.empty?(watch_directories) do
      Logger.warning("Watch directories not configured, cannot serve files")

      conn
      |> send_resp(404, "Not Found")
      |> halt()
    else
      # Join the path components to get the filename
      filename = Enum.join(path, "/")
      decoded_filename = Atci.Web.TranscriptController.decode_filename(filename)

      # Search for the video file across all watch directories
      case find_video_file_in_directories(watch_directories, decoded_filename) do
        nil ->
          conn
          |> send_resp(404, "Not Found")
          |> halt()

        found_path ->
          # Get file info for the found path
          case :prim_file.read_file_info(found_path) do
            {:ok, file_info(type: :regular) = file_info} ->
              # Get range header if present
              range = get_req_header(conn, "range")

              # Serve the file with range support
              conn
              |> put_resp_content_type(get_content_type(found_path))
              |> put_resp_header("accept-ranges", "bytes")
              |> serve_range(file_info, found_path, range)

            _ ->
              conn
              |> send_resp(404, "Not Found")
              |> halt()
          end
      end
    end
  end

  def call(conn, _opts), do: conn

  # Helper function to search for a video file across all watch directories
  defp find_video_file_in_directories(watch_directories, decoded_filename) do
    watch_directories
    |> Enum.find_value(fn watch_directory ->
      PathHelper.find_video_file(watch_directory, decoded_filename)
    end)
  end

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
end
