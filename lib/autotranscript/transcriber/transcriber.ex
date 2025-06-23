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

    # Get all MP4 files and process them
    Path.wildcard(Path.join(directory, "*.{MP4,mp4}"))
    |> Enum.each(fn video_path ->
      txt_path = String.replace_trailing(video_path, ".MP4", ".txt")
      txt_path = String.replace_trailing(txt_path, ".mp4", ".txt")

      unless File.exists?(txt_path) do
        with :ok <- convert_to_mp3(video_path),
             mp3_path =
               String.replace_trailing(video_path, ".MP4", ".mp3")
               |> String.replace_trailing(".mp4", ".mp3"),
             :ok <- transcribe_audio(mp3_path),
             :ok <- delete_mp3(mp3_path) do
          IO.puts("Successfully processed #{video_path}")
        else
          error -> IO.puts("Error processing #{video_path}: #{inspect(error)}")
        end
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
      with :ok <- convert_to_mp3(path),
           mp3_path =
             String.replace_trailing(path, ".MP4", ".mp3")
             |> String.replace_trailing(".mp4", ".mp3"),
           :ok <- transcribe_audio(mp3_path),
           :ok <- delete_mp3(mp3_path) do
        IO.puts("Successfully processed #{path}")
      else
        error -> IO.puts("Error processing #{path}: #{inspect(error)}")
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

  @doc """
  Converts a video file to MP3 audio using ffmpeg.

  ## Parameters
    - path: String path to the video file

  ## Examples
      iex> Autotranscript.convert_to_mp3("video.mp4")
      :ok

      iex> Autotranscript.convert_to_mp3("not_video.txt")
      {:error, :invalid_file_type}
  """
  def convert_to_mp3(path) do
    if String.ends_with?(path, [".MP4", ".mp4"]) do
      output_path = String.replace_trailing(path, ".MP4", ".mp3")
      output_path = String.replace_trailing(output_path, ".mp4", ".mp3")

      System.cmd("ffmpeg", ["-i", path, "-q:a", "0", "-map", "a", output_path])
      :ok
    else
      {:error, :invalid_file_type}
    end
  end

  @doc """
  Transcribes an MP3 file using Whisper CLI.

  ## Parameters
    - path: String path to the MP3 file

  ## Examples
      iex> Autotranscript.transcribe_audio("audio.mp3")
      :ok

      iex> Autotranscript.transcribe_audio("not_audio.txt")
      {:error, :invalid_file_type}
  """
  def transcribe_audio(path) do
    if String.ends_with?(path, ".mp3") do
      whispercli = Application.get_env(:autotranscript, :whispercli_path)
      model = Application.get_env(:autotranscript, :model_path)

      System.cmd(whispercli, ["-m", model, "-np", "-otxt", "-f", path])
      txt_path = String.replace_trailing(path, ".mp3", ".txt")
      File.rename(path <> ".txt", txt_path)
      :ok
    else
      {:error, :invalid_file_type}
    end
  end

  @doc """
  Deletes an MP3 file after transcription is complete.

  ## Parameters
    - path: String path to the MP3 file to delete

  ## Returns
    - :ok if the file was deleted successfully
    - {:error, reason} if the file could not be deleted
  """
  def delete_mp3(path) do
    if String.ends_with?(path, ".mp3") do
      case File.rm(path) do
        :ok ->
          Logger.info("Deleted temporary MP3 file: #{path}")
          :ok

        {:error, reason} ->
          Logger.warning("Failed to delete MP3 file #{path}: #{inspect(reason)}")
          {:error, reason}
      end
    else
      {:error, :invalid_file_type}
    end
  end
end
