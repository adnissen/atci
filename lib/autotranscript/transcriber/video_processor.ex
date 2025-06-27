defmodule Autotranscript.VideoProcessor do
  use GenServer
  require Logger

  @moduledoc """
  A GenServer that manages a queue of video files to be processed.
  Processes files one at a time to avoid overwhelming the system.
  """

  def start_link(_opts) do
    GenServer.start_link(__MODULE__, :ok, name: __MODULE__)
  end

  @impl true
  def init(:ok) do
    {:ok, %{queue: [], processing: false, current_file: nil}}
  end

  @doc """
  Adds a video file to the processing queue.
  """
  def add_to_queue(video_path) do
    GenServer.cast(__MODULE__, {:add_to_queue, video_path})
  end

  @doc """
  Gets the current queue status including the queue contents, processing state, and currently processing file.

  ## Returns
    - %{queue: [video_paths], processing: boolean, current_file: video_path | nil}

  ## Examples
      iex> Autotranscript.VideoProcessor.get_queue_status()
      %{queue: ["video1.mp4", "video2.mp4"], processing: true, current_file: "video1.mp4"}
  """
  def get_queue_status do
    GenServer.call(__MODULE__, :get_queue_status)
  end

  @impl true
  def handle_cast({:add_to_queue, video_path}, %{queue: queue, processing: processing, current_file: current_file} = state) do
    # Check if the video is already in the queue or currently being processed
    if video_path in queue or video_path == current_file do
      # Video is already queued or being processed, don't add it again
      {:noreply, state}
    else
      new_queue = [video_path | queue]

      if not processing do
        # Start processing if not already processing
        GenServer.cast(__MODULE__, :process_next)
        {:noreply, %{state | queue: new_queue, processing: true}}
      else
        # Just add to queue if already processing
        {:noreply, %{state | queue: new_queue}}
      end
    end
  end

  @impl true
  def handle_cast(:process_next, %{queue: [], processing: _processing, current_file: nil} = state) do
    # No more files to process
    {:noreply, %{state | processing: false}}
  end

  @impl true
  def handle_cast(:process_next, %{queue: [video_path | _rest], processing: _processing, current_file: nil} = state) do
    # Start processing the next file in the queue asynchronously
    # Keep the file in the queue until processing is complete
    spawn(fn ->
      case process_video_file(video_path) do
        :ok ->
          IO.puts("Successfully processed #{video_path}")
          GenServer.cast(__MODULE__, {:processing_complete, video_path, :ok})
        {:error, reason} ->
          IO.puts("Error processing #{video_path}: #{inspect(reason)}")
          GenServer.cast(__MODULE__, {:processing_complete, video_path, {:error, reason}})
      end
    end)

    {:noreply, %{state | current_file: video_path}}
  end

  @impl true
  def handle_cast({:processing_complete, video_path, _result}, %{queue: queue, processing: _processing, current_file: _current_file} = state) do
    # Remove the completed file from the queue
    new_queue = Enum.reject(queue, fn path -> path == video_path end)

    if new_queue == [] do
      # No more files to process
      {:noreply, %{state | queue: [], processing: false, current_file: nil}}
    else
      # Continue with next file
      GenServer.cast(__MODULE__, :process_next)
      {:noreply, %{state | queue: new_queue, processing: true, current_file: nil}}
    end
  end

  @impl true
  def handle_call(:get_queue_status, _from, %{queue: queue, processing: processing, current_file: current_file} = state) do
    {:reply, %{queue: queue, processing: processing, current_file: current_file}, state}
  end

  @doc """
  Processes a video file by converting it to MP3, transcribing the audio, and cleaning up.

  ## Parameters
    - video_path: String path to the video file

  ## Returns
    - :ok if the processing was successful
    - {:error, reason} if any step failed

  ## Examples
      iex> Autotranscript.VideoProcessor.process_video_file("video.mp4")
      :ok

      iex> Autotranscript.VideoProcessor.process_video_file("not_video.txt")
      {:error, :invalid_file_type}
  """
  def process_video_file(video_path) do
    with :ok <- convert_to_mp3(video_path),
         mp3_path =
           String.replace_trailing(video_path, ".MP4", ".mp3")
           |> String.replace_trailing(".mp4", ".mp3"),
         :ok <- transcribe_audio(mp3_path),
         :ok <- delete_mp3(mp3_path) do
      :ok
    end
  end

  @doc """
  Converts a video file to MP3 audio using ffmpeg.

  ## Parameters
    - path: String path to the video file

  ## Examples
      iex> Autotranscript.VideoProcessor.convert_to_mp3("video.mp4")
      :ok

      iex> Autotranscript.VideoProcessor.convert_to_mp3("not_video.txt")
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
      iex> Autotranscript.VideoProcessor.transcribe_audio("audio.mp3")
      :ok

      iex> Autotranscript.VideoProcessor.transcribe_audio("not_audio.txt")
      {:error, :invalid_file_type}
  """
  def transcribe_audio(path) do
    if String.ends_with?(path, ".mp3") do
      whispercli = Application.get_env(:autotranscript, :whispercli_path)
      model = Application.get_env(:autotranscript, :model_path)

      System.cmd(whispercli, ["-m", model, "-np", "-ovtt", "-f", path])
      vtt_path = String.replace_trailing(path, ".mp3", ".vtt")

      txt_path = String.replace_trailing(vtt_path, ".vtt", ".txt")
      File.rename(path <> ".vtt", txt_path)
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
