defmodule Autotranscript.Transcriber do
  use GenServer
  require Logger

  @moduledoc """
  Documentation for `Autotranscript`.
  """

  def start_link(_opts) do
    GenServer.start_link(__MODULE__, :ok, name: __MODULE__)
  end

  @impl true
  def init(:ok) do
    check_for_videos_with_missing_files_and_add_to_queue()

    # Start timer to check for new files every 2 seconds
    case watch_directory() do
      {:ok, timer_ref} ->
        {:ok, %{timer_ref: timer_ref}}

      {:error, reason} ->
        Logger.error("Failed to start directory watcher: #{inspect(reason)}")
        {:stop, reason}
    end
  end

  @impl true
  def handle_info(:check_directory, state) do
    check_for_videos_with_missing_files_and_add_to_queue()
    # Schedule the next check in 2 seconds
    timer_ref = Process.send_after(self(), :check_directory, 2000)
    {:noreply, %{state | timer_ref: timer_ref}}
  end

  def check_for_videos_with_missing_files_and_add_to_queue do
    directory = Autotranscript.Config.get(:watch_directory)
    current_time = System.system_time(:second)

    Path.wildcard(Path.join(directory, "*.{MP4,mp4}"))
    |> Enum.each(fn video_path ->
      txt_path = String.replace_trailing(video_path, ".MP4", ".txt")
      txt_path = String.replace_trailing(txt_path, ".mp4", ".txt")

      unless File.exists?(txt_path) do
        # Check if the video file is at least 3 seconds old
        case File.stat(video_path, time: :posix) do
          {:ok, %{mtime: mtime}} ->
            if current_time - mtime >= 3 do
              Autotranscript.VideoProcessor.add_to_queue(video_path)
            end
          {:error, _reason} ->
            # If we can't get file stats, skip this file
            Logger.warning("Could not get file stats for #{video_path}")
        end
      end
    end)
  end

  @doc """
  Starts a timer to check the directory for new files every 2 seconds.

  ## Examples
      iex> Autotranscript.watch_directory()
      {:ok, timer_ref}
  """
  def watch_directory do
    directory = Autotranscript.Config.get(:watch_directory)

    case File.dir?(directory) do
      true ->
        Logger.info("Watching: #{directory} (checking every 2 seconds)")
        timer_ref = Process.send_after(self(), :check_directory, 2000)
        {:ok, timer_ref}
      false ->
        {:error, :invalid_directory}
    end
  end
end
