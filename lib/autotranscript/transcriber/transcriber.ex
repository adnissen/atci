defmodule Autotranscript.Transcriber do
  use GenServer
  require Logger

  @moduledoc """
  Documentation for `Autotranscript`.
  """

  def start_link(_opts) do
    GenServer.start_link(__MODULE__, :ok, name: __MODULE__)
  end

  @doc """
  Starts directory watching after configuration is available.
  Called when configuration is set through the web interface.
  """
  def start_watching do
    GenServer.call(__MODULE__, :start_watching)
  end

  @impl true
  def init(:ok) do
    check_for_videos_with_missing_files_and_add_to_queue()

    # Start timer to check for new files every 2 seconds
    case watch_directory() do
      {:ok, timer_ref} ->
        {:ok, %{timer_ref: timer_ref}}

      {:error, reason} ->
        Logger.warning("Directory watcher not started: #{inspect(reason)}. Waiting for configuration.")
        # Don't stop the GenServer, just start without a timer
        # The web app needs to be available for configuration
        {:ok, %{timer_ref: nil}}
    end
  end

  @impl true
  def handle_info(:check_directory, state) do
    check_for_videos_with_missing_files_and_add_to_queue()
    
    # Only schedule the next check if we have a valid configuration
    case watch_directory() do
      {:ok, timer_ref} when timer_ref != nil ->
        {:noreply, %{state | timer_ref: timer_ref}}
      _ ->
        # Configuration not available, don't schedule next check
        {:noreply, %{state | timer_ref: nil}}
    end
  end

  @impl true
  def handle_call(:start_watching, _from, state) do
    # Try to start directory watching
    case watch_directory() do
      {:ok, timer_ref} ->
        Logger.info("Directory watching started after configuration update")
        {:reply, :ok, %{state | timer_ref: timer_ref}}
      {:error, reason} ->
        Logger.warning("Failed to start directory watching: #{inspect(reason)}")
        {:reply, {:error, reason}, state}
    end
  end

  def check_for_videos_with_missing_files_and_add_to_queue do
    directory = Autotranscript.ConfigManager.get_config_value("watch_directory")
    
    if directory == nil or directory == "" do
      Logger.warning("Watch directory not configured, skipping video check")
      :ok
    else
      do_check_for_videos(directory)
    end
  end
  
  defp do_check_for_videos(directory) do
    
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
    directory = Autotranscript.ConfigManager.get_config_value("watch_directory")
    
    case directory do
      nil ->
        Logger.warning("Watch directory not configured")
        {:ok, nil}
      "" ->
        Logger.warning("Watch directory is empty")
        {:ok, nil}
      _ ->
        case File.dir?(directory) do
          true ->
            Logger.info("Watching: #{directory} (checking every 2 seconds)")
            timer_ref = Process.send_after(self(), :check_directory, 2000)
            {:ok, timer_ref}
          false ->
            Logger.error("Watch directory does not exist: #{directory}")
            {:error, :invalid_directory}
        end
    end
  end
end
