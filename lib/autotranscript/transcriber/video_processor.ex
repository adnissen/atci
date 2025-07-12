defmodule Autotranscript.VideoProcessor do
  use GenServer
  require Logger

  alias Autotranscript.{PathHelper, VideoInfoCache, TranscriptModifier, MetaFileHandler}
  alias Autotranscript.ConfigManager

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
  def add_to_queue(video_path, process_type \\ :all, opts \\ %{}) do
    time = Map.get(opts, :time, nil)
    job_info = %{path: video_path, time: time}
    GenServer.cast(__MODULE__, {:add_to_queue, {process_type, job_info}})
  end

  @doc """
  Gets the current queue status including the queue contents, processing state, and currently processing file.

  ## Returns
    - %{queue: [{process_type, %{path: path, time: time}}], processing: boolean, current_file: {process_type, %{path: path, time: time}} | nil}

  ## Examples
      iex> Autotranscript.VideoProcessor.get_queue_status()
      %{queue: [{:all, %{path: "video1.mp4", time: nil}}, {:length, %{path: "video2.mp4", time: nil}}], processing: true, current_file: {:all, %{path: "video1.mp4", time: nil}}}
  """
  def get_queue_status do
    GenServer.call(__MODULE__, :get_queue_status)
  end

  @impl true
  def handle_cast(
        {:add_to_queue, {_process_type, _job_info} = job_tuple},
        %{queue: queue, processing: processing, current_file: current_file} = state
      ) do
    # Check if the job is already in the queue or currently being processed
    if job_tuple in queue or job_tuple == current_file do
      # Job is already queued or being processed, don't add it again
      {:noreply, state}
    else
      new_queue = [job_tuple | queue]

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
  def handle_cast(
        :process_next,
        %{
          queue: [{process_type, %{path: video_path} = job_info} = job_tuple | _rest],
          processing: _processing,
          current_file: nil
        } = state
      ) do
    # Start processing the next file in the queue asynchronously
    # Keep the file in the queue until processing is complete
    spawn(fn ->
      case process_video_file(job_info, process_type) do
        :ok ->
          IO.puts("Successfully processed #{video_path} with type #{process_type}")
          GenServer.cast(__MODULE__, {:processing_complete, job_tuple, :ok})

        {:error, reason} ->
          IO.puts("Error processing #{video_path}: #{inspect(reason)}")
          GenServer.cast(__MODULE__, {:processing_complete, job_tuple, {:error, reason}})
      end
    end)

    {:noreply, %{state | current_file: job_tuple}}
  end

  @impl true
  def handle_cast(
        {:processing_complete, job_tuple, _result},
        %{queue: queue, processing: _processing, current_file: _current_file} = state
      ) do
    # Remove the completed job from the queue
    new_queue = Enum.reject(queue, fn tuple -> tuple == job_tuple end)

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
  def handle_call(
        :get_queue_status,
        _from,
        %{queue: queue, processing: processing, current_file: current_file} = state
      ) do
    {:reply, %{queue: queue, processing: processing, current_file: current_file}, state}
  end

  @doc """
  Processes a video file by converting it to MP3, transcribing the audio, and cleaning up.

  ## Parameters
    - job_info: Map with %{path: video_path, time: time} where time is optional
    - process_type: Atom indicating the type of processing (:all, :length, :partial)

  ## Returns
    - :ok if the processing was successful
    - {:error, reason} if any step failed

  ## Examples
      iex> Autotranscript.VideoProcessor.process_video_file(%{path: "video.mp4", time: nil}, :all)
      :ok

      iex> Autotranscript.VideoProcessor.process_video_file(%{path: "video.mp4", time: nil}, :length)
      :ok

      iex> Autotranscript.VideoProcessor.process_video_file(%{path: "video.mp4", time: "01:30:45"}, :partial)
      :ok
  """
  def process_video_file(%{path: video_path, time: time} = _job_info, process_type) do
    result =
      case process_type do
        :all ->
          # Check if video has subtitles first
          case check_and_extract_subtitles(video_path) do
            {:ok, :extracted} ->
              # Subtitles were found and extracted, just save video length
              save_video_length(video_path)

            {:ok, :no_subtitles} ->
              # No subtitles, proceed with normal audio extraction and transcription
              with {:ok, mp3_path} <- convert_to_mp3(video_path),
                   :ok <- transcribe_audio(mp3_path, video_path),
                   :ok <- save_video_length(video_path) do
                :ok
              end

            {:error, reason} ->
              # If subtitle extraction fails, fall back to normal processing
              Logger.warning(
                "Failed to check subtitles: #{inspect(reason)}, falling back to audio transcription"
              )

              with {:ok, mp3_path} <- convert_to_mp3(video_path),
                   :ok <- transcribe_audio(mp3_path, video_path) do
                :ok
              end
          end

        :length ->
          save_video_length(video_path)

        :partial ->
          process_partial_video(video_path, time)
      end

    # Update the video info cache after processing
    VideoInfoCache.update_video_info_cache()

    result
  end

  @doc """
  Checks if a video file has any audio streams using ffprobe.

  ## Parameters
    - video_path: String path to the video file

  ## Returns
    - {:ok, true} if audio streams are found
    - {:ok, false} if no audio streams are found
    - {:error, reason} if ffprobe fails
  """
  def check_audio_streams(video_path) do
    ffprobe_path = ConfigManager.get_config_value("ffprobe_path") || "ffprobe"

    case System.cmd(ffprobe_path, [
           "-v",
           "error",
           "-select_streams",
           "a",
           "-show_entries",
           "stream=index",
           "-of",
           "csv=p=0",
           video_path
         ]) do
      {output, 0} ->
        # If output is empty, there are no audio streams
        has_audio = String.trim(output) != ""
        {:ok, has_audio}

      {error_output, _exit_code} ->
        {:error, "ffprobe failed: #{error_output}"}
    end
  end

  @doc """
  Converts a video file to MP3 audio using ffmpeg.

  ## Parameters
    - path: String path to the video file

  ## Returns
    - {:ok, mp3_path} where mp3_path is the path to the created MP3 file in tmp directory
    - {:error, reason} if conversion fails

  ## Examples
      iex> Autotranscript.VideoProcessor.convert_to_mp3("video.mp4")
      {:ok, "/tmp/video_12345.mp3"}

      iex> Autotranscript.VideoProcessor.convert_to_mp3("not_video.txt")
      {:error, :invalid_file_type}
  """
  def convert_to_mp3(path) do
    if String.ends_with?(path, PathHelper.video_extensions_with_dots()) do
      # First check if the video has any audio streams
      case check_audio_streams(path) do
        {:ok, true} ->
          # Create unique filename in tmp directory
          base_name = Path.basename(path, Path.extname(path))
          tmp_filename = "#{base_name}.mp3"
          output_path = Path.join(System.tmp_dir(), tmp_filename)

          ffmpeg_path = ConfigManager.get_config_value("ffmpeg_path") || "ffmpeg"

          # Use more specific mapping to avoid multiple audio stream issues
          case System.cmd(ffmpeg_path, [
                 "-i",
                 path,
                 # Map first audio stream only
                 "-map",
                 "0:a:0",
                 "-q:a",
                 "0",
                 # Convert to mono to avoid channel issues
                 "-ac",
                 "1",
                 # Set sample rate for consistency
                 "-ar",
                 "16000",
                 # Override existing files
                 "-y",
                 output_path
               ]) do
            {_output, 0} -> {:ok, output_path}
            {error_output, _exit_code} -> {:error, "ffmpeg failed: #{error_output}"}
          end

        {:ok, false} ->
          {:error, "No audio stream found in video file"}

        {:error, reason} ->
          {:error, "Failed to check audio streams: #{reason}"}
      end
    else
      {:error, :invalid_file_type}
    end
  end

  @doc """
  Transcribes an MP3 file using Whisper CLI.

  ## Parameters
    - path: String path to the MP3 file
    - video_path: string path to the original video file (for getting prompt from meta)

  ## Examples

      iex> Autotranscript.VideoProcessor.transcribe_audio("audio.mp3", "video.mp4")
      :ok

      iex> Autotranscript.VideoProcessor.transcribe_audio("not_audio.txt")
      {:error, :invalid_file_type}
  """
  def transcribe_audio(path, video_path) do
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
          # Get prompt from meta file if available
          prompt = get_prompt_from_meta(video_path)

          # Build command arguments with optional prompt
          args = ["-m", model, "-np", "--max-context", "0", "-ovtt", "-f", path]

          args =
            if prompt do
              args ++ ["--prompt", prompt]
            else
              args
            end

          System.cmd(whispercli, args)
          vtt_path = String.replace_trailing(path, ".mp3", ".vtt")

          txt_path = String.replace_trailing(vtt_path, ".vtt", ".txt")
          File.rename(vtt_path, txt_path)

          # Move txt file to video directory
          video_dir = Path.dirname(video_path)
          new_txt_path = Path.join(video_dir, Path.basename(txt_path))
          File.cp(txt_path, new_txt_path)
          txt_path = new_txt_path

          # Modify the transcript file to add model information
          case TranscriptModifier.add_source_to_meta(txt_path) do
            :ok ->
              Logger.info("Transcript file modified successfully: #{txt_path}")

            {:error, reason} ->
              Logger.warning("Failed to modify transcript file #{txt_path}: #{inspect(reason)}")
          end

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
    if String.ends_with?(video_path, PathHelper.video_extensions_with_dots()) do
      case get_video_length(video_path) do
        {:ok, length} ->
          meta_path = PathHelper.replace_video_extension_with(video_path, ".meta")

          # Use MetaFileHandler to update the length field
          case Autotranscript.MetaFileHandler.update_meta_field(meta_path, "length", length) do
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
    ffmpeg_path = ConfigManager.get_config_value("ffmpeg_path") || "ffmpeg"

    case System.cmd(
           ffmpeg_path,
           [
             "-i",
             video_path
           ],
           stderr_to_stdout: true
         ) do
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

  # Gets the prompt text from a video file's meta file if it exists.
  defp get_prompt_from_meta(video_path) do
    case video_path do
      nil ->
        nil

      path ->
        meta_path = PathHelper.replace_video_extension_with(path, ".meta")

        case MetaFileHandler.get_meta_field(meta_path, "prompt") do
          {:ok, prompt} -> prompt
          {:error, _} -> nil
        end
    end
  end

  @doc """
  Processes a video file from a specific time by truncating the text file,
  creating a temporary video from the given time, converting to MP3, transcribing,
  and appending the new content to the original text file.

  ## Parameters
    - video_path: String path to the video file
    - time_str: String time in format "HH:MM:SS", "MM:SS", or seconds

  ## Returns
    - :ok if the processing was successful
    - {:error, reason} if any step failed
  """
  def process_partial_video(video_path, time_str) do
    # Use the existing partial reprocess logic from the controller
    txt_file_path = PathHelper.replace_video_extension_with(video_path, ".txt")

    case parse_time_to_seconds(time_str) do
      {:ok, time_seconds} ->
        do_partial_video_reprocess(txt_file_path, video_path, time_str, time_seconds)

      {:error, reason} ->
        {:error, "Invalid time format: #{reason}"}
    end
  end

  defp parse_time_to_seconds(time_str) do
    cond do
      # Format: HH:MM:SS or HH:MM:SS.ms
      Regex.match?(~r/^\d{1,2}:\d{2}:\d{2}(\.\d+)?$/, time_str) ->
        case String.split(time_str, ":") do
          [hours, minutes, seconds_part] ->
            with {h, ""} <- Integer.parse(hours),
                 {m, ""} <- Integer.parse(minutes),
                 {s, _} <- Float.parse(seconds_part) do
              total_seconds = h * 3600 + m * 60 + s
              {:ok, total_seconds}
            else
              _ -> {:error, "Invalid time components"}
            end

          _ ->
            {:error, "Invalid time format"}
        end

      # Format: MM:SS or MM:SS.ms
      Regex.match?(~r/^\d{1,2}:\d{2}(\.\d+)?$/, time_str) ->
        case String.split(time_str, ":") do
          [minutes, seconds_part] ->
            with {m, ""} <- Integer.parse(minutes),
                 {s, _} <- Float.parse(seconds_part) do
              total_seconds = m * 60 + s
              {:ok, total_seconds}
            else
              _ -> {:error, "Invalid time components"}
            end

          _ ->
            {:error, "Invalid time format"}
        end

      # Format: seconds only (integer or float)
      true ->
        case Float.parse(time_str) do
          {seconds, ""} when seconds >= 0 -> {:ok, seconds}
          _ -> {:error, "Invalid numeric time"}
        end
    end
  end

  defp do_partial_video_reprocess(txt_file_path, video_path, time_str, time_seconds) do
    with :ok <- truncate_txt_file_at_time(txt_file_path, time_str),
         {:ok, temp_video_path} <- create_temp_video_from_time(video_path, time_seconds),
         {:ok, temp_mp3_path} <- convert_temp_video_to_mp3_partial(temp_video_path),
         {:ok, temp_txt_path} <- transcribe_temp_mp3_partial(temp_mp3_path),
         :ok <- cleanup_temp_files([temp_video_path, temp_mp3_path]),
         :ok <- adjust_timestamps_in_temp_file(temp_txt_path, time_seconds),
         :ok <- append_temp_txt_to_original(temp_txt_path, txt_file_path),
         :ok <- File.rm(temp_txt_path) do
      :ok
    else
      {:error, reason} -> {:error, reason}
    end
  end

  defp truncate_txt_file_at_time(txt_file_path, time_str) do
    case File.read(txt_file_path) do
      {:ok, content} ->
        lines = String.split(content, "\n")

        # Find the first line that includes the time
        truncated_lines =
          Enum.take_while(lines, fn line ->
            not String.contains?(line, time_str)
          end)

        # Write the truncated content back
        truncated_content = Enum.join(truncated_lines, "\n")

        case File.write(txt_file_path, truncated_content, [:utf8]) do
          :ok -> :ok
          {:error, reason} -> {:error, "Failed to truncate txt file: #{reason}"}
        end

      {:error, reason} ->
        {:error, "Failed to read txt file: #{reason}"}
    end
  end

  defp create_temp_video_from_time(video_path, time_seconds) do
    temp_video_path = Path.join(System.tmp_dir(), "temp_video.mp4")

    ffmpeg_path = ConfigManager.get_config_value("ffmpeg_path") || "ffmpeg"

    case System.cmd(ffmpeg_path, [
           "-ss",
           "#{time_seconds}",
           "-i",
           video_path,
           "-c",
           "copy",
           "-avoid_negative_ts",
           "make_zero",
           "-y",
           temp_video_path
         ]) do
      {_output, 0} -> {:ok, temp_video_path}
      {error_output, _exit_code} -> {:error, "ffmpeg failed: #{error_output}"}
    end
  end

  defp convert_temp_video_to_mp3_partial(temp_video_path) do
    temp_mp3_path = Path.join(System.tmp_dir(), "temp_audio.mp3")

    ffmpeg_path = ConfigManager.get_config_value("ffmpeg_path") || "ffmpeg"

    case System.cmd(ffmpeg_path, [
           "-i",
           temp_video_path,
           "-q:a",
           "0",
           "-map",
           "a",
           "-y",
           temp_mp3_path
         ]) do
      {_output, 0} -> {:ok, temp_mp3_path}
      {error_output, _exit_code} -> {:error, "ffmpeg audio conversion failed: #{error_output}"}
    end
  end

  defp transcribe_temp_mp3_partial(temp_mp3_path) do
    whispercli = Autotranscript.ConfigManager.get_config_value("whispercli_path")
    model = Autotranscript.ConfigManager.get_config_value("model_path")

    cond do
      whispercli == nil or whispercli == "" ->
        {:error, "Whisper CLI path not configured"}

      model == nil or model == "" ->
        {:error, "Model path not configured"}

      not File.exists?(whispercli) ->
        {:error, "Whisper CLI not found at: #{whispercli}"}

      not File.exists?(model) ->
        {:error, "Model file not found at: #{model}"}

      true ->
        # Build command arguments (no prompt needed for partial transcription)
        args = ["-m", model, "-np", "-ovtt", "-f", temp_mp3_path]

        case System.cmd(whispercli, args) do
          {_output, 0} ->
            # Convert VTT to TXT
            vtt_path = temp_mp3_path <> ".vtt"
            txt_path = String.replace_trailing(temp_mp3_path, ".mp3", ".txt")

            case File.rename(vtt_path, txt_path) do
              :ok -> {:ok, txt_path}
              {:error, reason} -> {:error, "Failed to rename VTT to TXT: #{reason}"}
            end

          {error_output, _exit_code} ->
            {:error, "Whisper transcription failed: #{error_output}"}
        end
    end
  end

  defp cleanup_temp_files(file_paths) do
    Enum.each(file_paths, fn path ->
      case File.rm(path) do
        :ok -> :ok
        # File doesn't exist, that's fine
        {:error, :enoent} -> :ok
        {:error, reason} -> Logger.warning("Failed to delete temp file #{path}: #{reason}")
      end
    end)

    :ok
  end

  defp adjust_timestamps_in_temp_file(temp_txt_path, start_time_seconds) do
    case File.read(temp_txt_path) do
      {:ok, content} ->
        # Regex to match timestamp format HH:MM:SS.XXX --> HH:MM:SS.XXX
        timestamp_regex = ~r/(\d{2}:\d{2}:\d{2}\.\d{3}) --> (\d{2}:\d{2}:\d{2}\.\d{3})/

        adjusted_content =
          Regex.replace(timestamp_regex, content, fn full_match, start_ts, end_ts ->
            case {timestamp_to_seconds(start_ts), timestamp_to_seconds(end_ts)} do
              {{:ok, start_secs}, {:ok, end_secs}} ->
                adjusted_start = start_secs + start_time_seconds
                adjusted_end = end_secs + start_time_seconds

                "#{seconds_to_timestamp(adjusted_start)} --> #{seconds_to_timestamp(adjusted_end)}"

              _ ->
                # If parsing fails, return original match
                full_match
            end
          end)

        case File.write(temp_txt_path, adjusted_content, [:utf8]) do
          :ok -> :ok
          {:error, reason} -> {:error, "Failed to write adjusted timestamps: #{reason}"}
        end

      {:error, reason} ->
        {:error, "Failed to read temp txt file for timestamp adjustment: #{reason}"}
    end
  end

  defp timestamp_to_seconds(timestamp_str) do
    case String.split(timestamp_str, ":") do
      [hours, minutes, seconds_with_ms] ->
        case String.split(seconds_with_ms, ".") do
          [seconds, milliseconds] ->
            with {h, ""} <- Integer.parse(hours),
                 {m, ""} <- Integer.parse(minutes),
                 {s, ""} <- Integer.parse(seconds),
                 {ms, ""} <- Integer.parse(milliseconds) do
              total_seconds = h * 3600 + m * 60 + s + ms / 1000.0
              {:ok, total_seconds}
            else
              _ -> {:error, "Invalid timestamp components"}
            end

          _ ->
            {:error, "Invalid seconds format"}
        end

      _ ->
        {:error, "Invalid timestamp format"}
    end
  end

  defp seconds_to_timestamp(total_seconds) do
    hours = trunc(total_seconds / 3600)
    remaining_seconds = total_seconds - hours * 3600
    minutes = trunc(remaining_seconds / 60)
    seconds = remaining_seconds - minutes * 60

    # Split seconds into integer and fractional parts
    integer_seconds = trunc(seconds)
    milliseconds = trunc((seconds - integer_seconds) * 1000)

    # Format with leading zeros
    :io_lib.format("~2..0B:~2..0B:~2..0B.~3..0B", [hours, minutes, integer_seconds, milliseconds])
    |> List.to_string()
  end

  defp append_temp_txt_to_original(temp_txt_path, original_txt_path) do
    case File.read(temp_txt_path) do
      {:ok, content} ->
        lines = String.split(content, "\n")
        # Remove the first two lines and get the rest
        remaining_lines =
          case lines do
            [_first, _second | rest] -> rest
            [_first] -> []
            [] -> []
          end

        # Append to original file
        content_to_append =
          case remaining_lines do
            [] -> ""
            lines -> "\n" <> Enum.join(lines, "\n")
          end

        # Use File.write with :append and :utf8 options to handle Unicode content properly
        case File.write(original_txt_path, content_to_append, [:append, :utf8]) do
          :ok -> :ok
          {:error, reason} -> {:error, "Failed to append to original file: #{reason}"}
        end

      {:error, reason} ->
        {:error, "Failed to read temp txt file: #{reason}"}
    end
  end

  @doc """
  Checks if a video file has subtitles and extracts them if found.

  ## Parameters
    - video_path: String path to the video file

  ## Returns
    - {:ok, :extracted} if subtitles were found and extracted
    - {:ok, :no_subtitles} if no subtitles were found
    - {:error, reason} if there was an error
  """
  def check_and_extract_subtitles(video_path) do
    case get_subtitle_streams(video_path) do
      {:ok, []} ->
        {:ok, :no_subtitles}

      {:ok, subtitle_streams} ->
        # Extract the first/default subtitle stream
        first_stream = List.first(subtitle_streams)
        extract_subtitle_stream(video_path, first_stream)

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc """
  Gets information about subtitle streams in a video file using ffprobe.

  ## Parameters
    - video_path: String path to the video file

  ## Returns
    - {:ok, subtitle_streams} where subtitle_streams is a list of stream indices
    - {:error, reason} if ffprobe fails
  """
  def get_subtitle_streams(video_path) do
    ffprobe_path = ConfigManager.get_config_value("ffprobe_path") || "ffprobe"

    case System.cmd(ffprobe_path, [
           "-v",
           "error",
           "-select_streams",
           "s",
           "-show_entries",
           "stream=index,codec_name,codec_type",
           "-of",
           "csv=p=0",
           video_path
         ]) do
      {output, 0} ->
        # Parse the output to get subtitle stream indices
        streams =
          output
          |> String.trim()
          |> String.split("\n", trim: true)
          |> Enum.map(fn line ->
            case String.split(line, ",") do
              [index, _codec_name, "subtitle"] ->
                case Integer.parse(index) do
                  {idx, ""} -> idx
                  _ -> nil
                end

              _ ->
                nil
            end
          end)
          |> Enum.filter(&(&1 != nil))

        {:ok, streams}

      {error_output, _exit_code} ->
        {:error, "ffprobe failed: #{error_output}"}
    end
  end

  @doc """
  Extracts a subtitle stream from a video file and saves it as a text file.

  ## Parameters
    - video_path: String path to the video file
    - stream_index: Integer index of the subtitle stream to extract

  ## Returns
    - {:ok, :extracted} if successful
    - {:error, reason} if extraction failed
  """
  def extract_subtitle_stream(video_path, stream_index) do
    txt_path = PathHelper.replace_video_extension_with(video_path, ".txt")
    temp_srt_path = Path.join(System.tmp_dir(), "temp_subtitle.srt")

    # Extract subtitle to temporary SRT file
    ffmpeg_path = ConfigManager.get_config_value("ffmpeg_path") || "ffmpeg"

    case System.cmd(ffmpeg_path, [
           "-i",
           video_path,
           "-map",
           "0:#{stream_index}",
           "-c:s",
           "srt",
           "-y",
           temp_srt_path
         ]) do
      {_output, 0} ->
        # Convert SRT to plain text transcript format
        case convert_srt_to_transcript(temp_srt_path, txt_path) do
          :ok ->
            File.rm(temp_srt_path)
            Logger.info("Successfully extracted subtitles from #{video_path}")
            {:ok, :extracted}

          {:error, reason} ->
            File.rm(temp_srt_path)
            {:error, reason}
        end

      {error_output, _exit_code} ->
        {:error, "ffmpeg subtitle extraction failed: #{error_output}"}
    end
  end

  @doc """
  Converts an SRT subtitles to the transcript format used by the application.

  ## Parameters
    - srt_path: String path to the SRT file
    - txt_path: String path where the transcript will be saved

  ## Returns
    - :ok if successful
    - {:error, reason} if conversion failed
  """
  def convert_srt_to_transcript(srt_path, txt_path) do
    case File.read(srt_path) do
      {:ok, content} ->
        # Parse SRT format and convert to transcript format
        transcript_lines = parse_srt_content(content)

        # Write transcript without the model line
        final_content = Enum.join(transcript_lines, "\n")

        case File.write(txt_path, final_content, [:utf8]) do
          :ok ->
            # Save source information to meta file
            meta_path = String.replace_trailing(txt_path, ".txt", ".meta")

            case Autotranscript.MetaFileHandler.update_meta_field(
                   meta_path,
                   "source",
                   "subtitles"
                 ) do
              :ok ->
                :ok

              {:error, reason} ->
                Logger.warning("Failed to update meta file with source: #{inspect(reason)}")
                # Still return ok since transcript was written successfully
                :ok
            end

          {:error, reason} ->
            {:error, "Failed to write transcript: #{reason}"}
        end

      {:error, reason} ->
        {:error, "Failed to read SRT file: #{reason}"}
    end
  end

  defp parse_srt_content(content) do
    # Split content into subtitle blocks
    blocks =
      content
      |> String.trim()
      |> String.split(~r/\n\n+/)
      |> Enum.filter(&(&1 != ""))

    # Process each block and join with empty lines between entries
    blocks
    |> Enum.map(fn block ->
      lines = String.split(block, "\n")

      case lines do
        [_index, timestamp_line | text_lines] when text_lines != [] ->
          # Extract both start and end times from timestamp line (format: "00:00:00,000 --> 00:00:03,000")
          case Regex.run(
                 ~r/^(\d{2}:\d{2}:\d{2}),(\d{3}) --> (\d{2}:\d{2}:\d{2}),(\d{3})/,
                 timestamp_line
               ) do
            [_, start_time, start_millis, end_time, end_millis] ->
              # Convert to our format with period instead of comma
              start_timestamp = "#{start_time}.#{start_millis}"
              end_timestamp = "#{end_time}.#{end_millis}"
              text = Enum.join(text_lines, " ")
              "#{start_timestamp} --> #{end_timestamp}\n#{text}"

            _ ->
              nil
          end

        _ ->
          nil
      end
    end)
    |> Enum.filter(&(&1 != nil))
    |> Enum.join("\n\n")
    # Split back into lines for consistency with the rest of the code
    |> String.split("\n")
  end
end
