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
    directory = Application.get_env(:autotranscript, :watch_directory)

    # Get all MP4 files and add them to the processing queue
    Path.wildcard(Path.join(directory, "*.{MP4,mp4}"))
    |> Enum.each(fn video_path ->
      txt_path = String.replace_trailing(video_path, ".MP4", ".txt")
      txt_path = String.replace_trailing(txt_path, ".mp4", ".txt")

      unless File.exists?(txt_path) do
        Autotranscript.VideoProcessor.add_to_queue(video_path)
      end
    end)

    # Start watching directory for new files
    case watch_directory() do
      {:ok, pid} ->
        {:ok, %{file_system_pid: pid}}

      {:error, reason} ->
        Logger.error("Failed to start directory watcher: #{inspect(reason)}")
        {:stop, reason}
    end
  end

  @impl true
  def handle_info({:file_event, _pid, {path, events}}, state) do
    # Process new MP4 files here
    if String.ends_with?(path, [".MP4", ".mp4"]) && Enum.member?(events, :created) do
      Autotranscript.VideoProcessor.add_to_queue(path)
    end

    IO.inspect(events)
    # Process deleted TXT files - regenerate if corresponding MP4 exists
    if String.ends_with?(path, [".TXT", ".txt"]) && Enum.member?(events, :removed) do
      video_path = String.replace_trailing(path, ".TXT", ".MP4")
      video_path = String.replace_trailing(video_path, ".txt", ".MP4")

      if File.exists?(video_path) do
        Autotranscript.VideoProcessor.add_to_queue(video_path)
      end
    end

    {:noreply, state}
  end

  @impl true
  def handle_info({:file_event, _pid, :stop}, state) do
    Logger.info("File system watcher stopped")
    {:noreply, state}
  end

  @doc """
  Watches a directory for file changes.

  ## Parameters
    - directory: String path to the directory to watch

  ## Examples
      iex> Autotranscript.watch_directory("path/to/directory")
      {:ok, pid}
  """
  def watch_directory do
    directory = Application.get_env(:autotranscript, :watch_directory)

    case File.dir?(directory) do
      true ->
        {:ok, pid} = FileSystem.start_link(dirs: [directory])
        FileSystem.subscribe(pid)

        Logger.info("Watching: #{directory}")
        {:ok, pid}
      false ->
        {:error, :invalid_directory}
    end
  end
end
