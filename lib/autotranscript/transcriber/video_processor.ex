defmodule Autotranscript.VideoProcessor do
  use GenServer
  require Logger

  alias Autotranscript.PathHelper

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
  def add_to_queue(video_path, process_type \\ :all) do
    GenServer.cast(__MODULE__, {:add_to_queue, {video_path, process_type}})
  end

  @doc """
  Gets the current queue status including the queue contents, processing state, and currently processing file.

  ## Returns
    - %{queue: [{video_path, process_type}], processing: boolean, current_file: {video_path, process_type} | nil}

  ## Examples
      iex> Autotranscript.VideoProcessor.get_queue_status()
      %{queue: [{"video1.mp4", :all}, {"video2.mp4", :length}], processing: true, current_file: {"video1.mp4", :all}}
  """
  def get_queue_status do
    GenServer.call(__MODULE__, :get_queue_status)
  end

  @impl true
  def handle_cast({:add_to_queue, {video_path, process_type} = video_tuple}, %{queue: queue, processing: processing, current_file: current_file} = state) do
    # Check if the video is already in the queue or currently being processed
    if video_tuple in queue or video_tuple == current_file do
      # Video is already queued or being processed, don't add it again
      {:noreply, state}
    else
      new_queue = [video_tuple | queue]

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
  def handle_cast(:process_next, %{queue: [{video_path, process_type} = video_tuple | _rest], processing: _processing, current_file: nil} = state) do
    # Start processing the next file in the queue asynchronously
    # Keep the file in the queue until processing is complete
    spawn(fn ->
      case process_video_file(video_path, process_type) do
        :ok ->
          IO.puts("Successfully processed #{video_path} with type #{process_type}")
          GenServer.cast(__MODULE__, {:processing_complete, video_tuple, :ok})
        {:error, reason} ->
          IO.puts("Error processing #{video_path}: #{inspect(reason)}")
          GenServer.cast(__MODULE__, {:processing_complete, video_tuple, {:error, reason}})
      end
    end)

    {:noreply, %{state | current_file: video_tuple}}
  end

  @impl true
  def handle_cast({:processing_complete, video_tuple, _result}, %{queue: queue, processing: _processing, current_file: _current_file} = state) do
    # Remove the completed file from the queue
    new_queue = Enum.reject(queue, fn tuple -> tuple == video_tuple end)

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
    - process_type: Atom indicating the type of processing (:all, :length)

  ## Returns
    - :ok if the processing was successful
    - {:error, reason} if any step failed

  ## Examples
      iex> Autotranscript.VideoProcessor.process_video_file("video.mp4", :all)
      :ok

      iex> Autotranscript.VideoProcessor.process_video_file("video.mp4", :length)
      :ok

      iex> Autotranscript.VideoProcessor.process_video_file("not_video.txt", :all)
      {:error, :invalid_file_type}
  """
  def process_video_file(video_path, process_type \\ :all) do
    case process_type do
      :all ->
        with :ok <- convert_to_mp3(video_path),
             mp3_path = PathHelper.find_mp3_file_from_video(video_path),
             :ok <- transcribe_audio(mp3_path),
             :ok <- delete_mp3(mp3_path),
             :ok <- save_video_length(video_path) do
          :ok
        end
      :length ->
        save_video_length(video_path)
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
    if String.ends_with?(path, PathHelper.video_extensions_with_dots) do
      output_path = PathHelper.replace_video_extension_with(path, ".mp3")

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
      whispercli = Autotranscript.ConfigManager.get_config_value("whispercli_path")
      model = Autotranscript.ConfigManager.get_config_value("model_path")

      cond do
        whispercli == nil or whispercli == "" ->
          Logger.error("Whisper CLI path not configured")
          {:error, :whispercli_not_configured}
        model == nil or model == "" ->
          Logger.error("Model path not configured")
          {:error, :model_not_configured}
        not File.exists?(whispercli) ->
          Logger.error("Whisper CLI not found at: #{whispercli}")
          {:error, :whispercli_not_found}
        not File.exists?(model) ->
          Logger.error("Model file not found at: #{model}")
          {:error, :model_not_found}
        true ->
          System.cmd(whispercli, ["-m", model, "-np", "-ovtt", "-f", path])
          vtt_path = String.replace_trailing(path, ".mp3", ".vtt")

          txt_path = String.replace_trailing(vtt_path, ".vtt", ".txt")
          File.rename(path <> ".vtt", txt_path)
          :ok
      end

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

  @doc """
  Gets the length of a video file using ffmpeg and saves it to a meta file.

  ## Parameters
    - video_path: String path to the video file

  ## Returns
    - :ok if the length was successfully determined and saved
    - {:error, reason} if any step failed
  """
  def save_video_length(video_path) do
    if String.ends_with?(video_path, PathHelper.video_extensions_with_dots) do
      case get_video_length(video_path) do
        {:ok, length} ->
          meta_path = PathHelper.replace_video_extension_with(video_path, ".meta")

          case File.write(meta_path, length) do
            :ok ->
              Logger.info("Saved video length #{length} to #{meta_path}")
              :ok
            {:error, reason} ->
              Logger.warning("Failed to write meta file #{meta_path}: #{inspect(reason)}")
              {:error, reason}
          end
        {:error, reason} ->
          Logger.warning("Failed to get video length for #{video_path}: #{inspect(reason)}")
          {:error, reason}
      end
    else
      {:error, :invalid_file_type}
    end
  end

  @doc """
  Gets the length of a video file using ffmpeg.

  ## Parameters
    - video_path: String path to the video file

  ## Returns
    - {:ok, length} where length is a string in format "hh:mm:ss"
    - {:error, reason} if ffmpeg fails
  """
  def get_video_length(video_path) do
    case System.cmd("ffmpeg", [
      "-i", video_path
    ], stderr_to_stdout: true) do
      {output, _exit_code} ->
        # Parse the duration from ffmpeg output
        case Regex.run(~r/Duration: (\d{2}:\d{2}:\d{2}\.\d{2})/, output) do
          [_, duration_with_ms] ->
            # Remove milliseconds to get hh:mm:ss format
            length = duration_with_ms |> String.split(".") |> List.first()
            {:ok, length}
          nil ->
            {:error, "Could not parse duration from ffmpeg output"}
        end
    end
  end
end
