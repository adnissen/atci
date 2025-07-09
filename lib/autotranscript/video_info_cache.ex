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

  # Helper function to extract source name from meta file
  defp extract_source_from_meta(meta_path) do
    case Autotranscript.MetaFileHandler.get_meta_field(meta_path, "source") do
      {:ok, source} -> source
      {:error, _} -> nil
    end
  end

  # Helper function to extract model name from transcript content (for backward compatibility)
  defp extract_model_from_transcript(content) do
    # Look for the first line that starts with "model: "
    case String.split(content, "\n", parts: 2) do
      [first_line | _] ->
        if String.starts_with?(first_line, "model: ") do
          String.replace_prefix(first_line, "model: ", "") |> String.trim()
        else
          nil
        end
      _ ->
        nil
    end
  end

  # Scans the watch directory and returns video file information.
  # This is the renamed version of the original get_video_files method.
  defp get_video_info_from_disk do
    watch_directory = ConfigManager.get_config_value("watch_directory")

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
          meta_path = Path.join(watch_directory, "#{filename}.meta")

          # If transcript exists, get line count, last modified time, and source/model
          {line_count, last_generated, model} = if transcript_exists do
            case File.read(txt_path) do
              {:ok, content} ->
                lines = String.split(content, "\n")
                line_count = length(lines)
                
                # Try to get source from meta file first, fall back to model from transcript
                source = extract_source_from_meta(meta_path)
                model_name = if source do
                  source
                else
                  # Backward compatibility: check for model in transcript
                  extract_model_from_transcript(content)
                end
                
                case File.stat(txt_path) do
                  {:ok, txt_stat} -> {line_count, txt_stat.mtime |> Autotranscript.Web.TranscriptHTML.format_datetime(), model_name}
                  {:error, _} -> {line_count, nil, model_name}
                end
              {:error, _} -> {0, nil, nil}
            end
          else
            {0, nil, nil}
          end

          # If transcript exists, try to read video length from meta file
          length = if transcript_exists do
            case Autotranscript.MetaFileHandler.get_meta_field(meta_path, "length") do
              {:ok, length_value} -> length_value
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
            length: length,
            model: model
          }
        {:error, _} ->
          nil
      end
      end)
      |> Enum.reject(&is_nil/1)
      |> Enum.sort_by(& &1.created_at, :desc)
    end
  end
end