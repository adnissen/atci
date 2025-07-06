defmodule Autotranscript.VideoInfoCache do
  use GenServer
  require Logger

  alias Autotranscript.{ConfigManager, PathHelper}

  @moduledoc """
  A GenServer that maintains a cache of video file information.
  Updates the cache when videos are processed and provides fast access to video info.
  """

  def start_link(_opts) do
    GenServer.start_link(__MODULE__, :ok, name: __MODULE__)
  end

  @impl true
  def init(:ok) do
    # Initialize with empty cache and update it immediately
    send(self(), :update_cache)
    {:ok, %{video_files: []}}
  end

  @doc """
  Gets the cached video files information.
  """
  def get_video_files do
    GenServer.call(__MODULE__, :get_video_files)
  end

  @doc """
  Updates the video info cache by scanning the disk.
  """
  def update_video_info_cache do
    GenServer.cast(__MODULE__, :update_cache)
  end

  @impl true
  def handle_call(:get_video_files, _from, %{video_files: video_files} = state) do
    {:reply, video_files, state}
  end

  @impl true
  def handle_cast(:update_cache, state) do
    video_files = get_video_info_from_disk()
    {:noreply, %{state | video_files: video_files}}
  end

  @impl true
  def handle_info(:update_cache, state) do
    video_files = get_video_info_from_disk()
    {:noreply, %{state | video_files: video_files}}
  end

  # Scans the watch directory and returns video file information.
  # This is the renamed version of the original get_video_files method.
  defp get_video_info_from_disk do
    watch_directories = ConfigManager.get_config_value("watch_directories")

    if watch_directories == nil or watch_directories == [] do
      []
    else
      # Iterate over all watch directories and collect video info from each
      watch_directories
      |> Enum.flat_map(&get_video_info_from_directory/1)
      |> Enum.sort_by(& &1.created_at, :desc)
    end
  end

  # Helper function to get video info from a single directory
  defp get_video_info_from_directory(watch_directory) do
    if watch_directory == nil or watch_directory == "" do
      []
    else
      # Get all video files in the watch directory
      Path.wildcard(Path.join(watch_directory, "**/*.#{PathHelper.video_wildcard_pattern}"))
      |> Enum.map(fn file_path ->
      case File.stat(file_path) do
        {:ok, stat} ->
          # Get the relative path from watch_directory to the file
          relative_path = Path.relative_to(file_path, watch_directory)
          filename = Path.rootname(relative_path)
          display_name = relative_path
          txt_path = Path.join(watch_directory, "#{filename}.txt")

          # Check if transcript exists
          transcript_exists = File.exists?(txt_path)

          # If transcript exists, get line count and last modified time
          {line_count, last_generated} = if transcript_exists do
            case File.read(txt_path) do
              {:ok, content} ->
                line_count = length(String.split(content, "\n"))
                case File.stat(txt_path) do
                  {:ok, txt_stat} -> {line_count, txt_stat.mtime |> Autotranscript.Web.TranscriptHTML.format_datetime()}
                  {:error, _} -> {line_count, nil}
                end
              {:error, _} -> {0, nil}
            end
          else
            {0, nil}
          end

          # If transcript exists, try to read video length from meta file
          length = if transcript_exists do
            meta_path = Path.join(watch_directory, "#{filename}.meta")
            case File.read(meta_path) do
              {:ok, length_content} -> String.trim(length_content)
              {:error, _} -> nil
            end
          else
            nil
          end

          %{
            name: display_name,
            base_name: filename,
            created_at: stat.ctime |> Autotranscript.Web.TranscriptHTML.format_datetime(),
            line_count: line_count,
            full_path: file_path,
            transcript: transcript_exists,
            last_generated: last_generated,
            length: length
          }
        {:error, _} ->
          nil
      end
      end)
      |> Enum.reject(&is_nil/1)
    end
  end
end