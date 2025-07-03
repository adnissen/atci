defmodule Autotranscript.Transcriber do
  use GenServer
  require Logger

  alias Autotranscript.PathHelper

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

  @doc """
  Returns the current watching status.
  """
  def watching_status do
    GenServer.call(__MODULE__, :get_status)
  end

  @impl true
  def init(:ok) do
    # Check if configuration is already available
    case watch_directory() do
      {:ok, timer_ref} when timer_ref != nil ->
        # Configuration is available, start directory watching
        check_for_videos_with_missing_files_and_add_to_queue()
        Logger.info("Configuration found, directory watching started")
        {:ok, %{timer_ref: timer_ref, config_timer_ref: nil, watching: true}}

      _ ->
        # Configuration not available, start checking for configuration
        Logger.info("No configuration found, will check every 5 seconds until available")
        config_timer_ref = Process.send_after(self(), :check_config, 5000)
        {:ok, %{timer_ref: nil, config_timer_ref: config_timer_ref, watching: false}}
    end
  end

  @impl true
  def handle_info(:check_directory, %{watching: true} = state) do
    check_for_videos_with_missing_files_and_add_to_queue()

    # Schedule the next directory check in 2 seconds
    timer_ref = Process.send_after(self(), :check_directory, 2000)
    {:noreply, %{state | timer_ref: timer_ref}}
  end

  @impl true
  def handle_info(:check_config, %{watching: false} = state) do
    # Try to start directory watching
    case watch_directory() do
      {:ok, timer_ref} when timer_ref != nil ->
        # Configuration is now available, start directory watching
        check_for_videos_with_missing_files_and_add_to_queue()
        Logger.info("Configuration became available, directory watching started")

        # Cancel the config checking timer if it exists
        if state.config_timer_ref do
          Process.cancel_timer(state.config_timer_ref)
        end

        {:noreply, %{state | timer_ref: timer_ref, config_timer_ref: nil, watching: true}}

      _ ->
        # Configuration still not available, check again in 5 seconds
        config_timer_ref = Process.send_after(self(), :check_config, 5000)
        {:noreply, %{state | config_timer_ref: config_timer_ref}}
    end
  end

  # Ignore directory checks when not watching
  @impl true
  def handle_info(:check_directory, %{watching: false} = state) do
    {:noreply, state}
  end

  # Ignore config checks when already watching
  @impl true
  def handle_info(:check_config, %{watching: true} = state) do
    {:noreply, state}
  end

  @impl true
  def handle_call(:start_watching, _from, state) do
    # Try to start directory watching
    case watch_directory() do
      {:ok, timer_ref} when timer_ref != nil ->
        Logger.info("Directory watching started after configuration update")

        # Cancel any existing config checking timer
        if state.config_timer_ref do
          Process.cancel_timer(state.config_timer_ref)
        end

        # Start the initial directory check and video processing
        check_for_videos_with_missing_files_and_add_to_queue()

        {:reply, :ok, %{state | timer_ref: timer_ref, config_timer_ref: nil, watching: true}}

      {:error, reason} ->
        Logger.warning("Failed to start directory watching: #{inspect(reason)}")
        {:reply, {:error, reason}, state}

      _ ->
        Logger.warning("Directory watching could not be started (no valid configuration)")
        {:reply, {:error, :no_config}, state}
    end
  end

  @impl true
  def handle_call(:get_status, _from, state) do
    status = %{
      watching: state.watching,
      has_timer: state.timer_ref != nil,
      has_config_timer: state.config_timer_ref != nil
    }
    {:reply, status, state}
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

    Path.wildcard(Path.join(directory, "**/*.#{PathHelper.video_wildcard_pattern()}"))
    |> Enum.each(fn video_path ->
      unless PathHelper.find_txt_file_from_video(video_path) do
        # Check if the video file is at least 3 seconds old
        case File.stat(video_path, time: :posix) do
          {:ok, %{mtime: mtime}} ->
            if current_time - mtime >= 3 do
              Autotranscript.VideoProcessor.add_to_queue(video_path, :all)
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
