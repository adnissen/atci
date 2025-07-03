defmodule Autotranscript.Web.TranscriptController do
  use Autotranscript.Web, :controller
  require Logger

  alias Autotranscript.{PathHelper, VideoInfoCache}
  def index(conn, _params) do
    # Get all .txt files in the watch directory (will be empty array if no config)
    txt_files =
      VideoInfoCache.get_video_files()
      |> Jason.encode!()

    render(conn, :index, txt_files: txt_files)
  end

  def show(conn, %{"filename" => filename}) do
    watch_directory = Autotranscript.ConfigManager.get_config_value("watch_directory")

    if watch_directory == nil or watch_directory == "" do
      conn
      |> put_status(:service_unavailable)
      |> put_resp_content_type("text/plain")
      |> send_resp(503, "Watch directory not configured. Please configure the application first.")
    else
      file_path = Path.join(watch_directory, "#{filename}.txt")

          case File.read(file_path) do
        {:ok, content} ->
          conn
          |> put_resp_content_type("text/plain")
          |> send_resp(200, content)
        {:error, :enoent} ->
          conn
          |> put_status(:not_found)
          |> put_resp_content_type("text/plain")
          |> send_resp(404, "Transcript file '#{filename}' not found")
        {:error, reason} ->
          conn
          |> put_status(:internal_server_error)
          |> put_resp_content_type("text/plain")
          |> send_resp(500, "Error reading transcript file '#{filename}': #{reason}")
      end
    end
  end
  def grep(conn, %{"text" => search_text}) do
    watch_directory = Autotranscript.ConfigManager.get_config_value("watch_directory")

    if watch_directory == nil or watch_directory == "" do
      conn
      |> put_status(:service_unavailable)
      |> put_resp_content_type("application/json")
      |> send_resp(503, Jason.encode!(%{error: "Watch directory not configured. Please configure the application first."}))
    else
      # Change to the watch directory and run grep with line numbers
      # Run first grep command for files in root directory
      {output1, exit_code1} = System.shell("grep -Hni \"" <> search_text <> "\" *.txt", cd: watch_directory)

      # Run second grep command for files in subdirectories
      {output2, exit_code2} = System.shell("grep -Hni \"" <> search_text <> "\" **/*.txt", cd: watch_directory)

      cond do
        # Both commands failed with error (exit code > 1)
        exit_code1 > 1 and exit_code2 > 1 ->
          conn
          |> put_status(:internal_server_error)
          |> put_resp_content_type("application/json")
          |> send_resp(500, Jason.encode!(%{error: "Error running grep: #{output1} #{output2}"}))

        # At least one command succeeded (exit code 0) or found no matches (exit code 1)
        true ->
          # Parse outputs, defaulting to empty string if command errored
          results1 = if exit_code1 <= 1, do: parse_grep_output(output1), else: %{}
          results2 = if exit_code2 <= 1, do: parse_grep_output(output2), else: %{}

          # Merge results, combining line numbers for any overlapping files
          combined_results = Map.merge(results1, results2, fn _k, v1, v2 ->
            Enum.sort(Enum.uniq(v1 ++ v2))
          end)

          conn
          |> put_resp_content_type("application/json")
          |> send_resp(200, Jason.encode!(combined_results))
      end
    end
  end

  defp parse_grep_output(output) do
    output
    |> String.split("\n")
    |> Enum.reject(&(&1 == ""))
    |> Enum.reduce(%{}, fn line, acc ->
      case Regex.run(~r/([^:]+)\.txt:(\d+):(.+)/, line) do
        [_, filename, line_num, _content] ->
          filename = String.trim(filename)
          line_num = String.to_integer(line_num)

          Map.update(acc, filename, [line_num], fn existing_lines ->
            [line_num | existing_lines]
          end)
        _ ->
          acc
      end
    end)
    |> Map.new(fn {filename, lines} -> {filename, Enum.sort(Enum.uniq(lines))} end)
  end

  def regenerate(conn, %{"filename" => filename}) do
    watch_directory = Autotranscript.ConfigManager.get_config_value("watch_directory")
    file_path = Path.join(watch_directory, "#{filename}.txt")

    case File.rm(file_path) do
      :ok ->
        conn
        |> put_resp_content_type("text/plain")
        |> send_resp(200, "Transcript file '#{filename}.txt' deleted for regeneration")
      {:error, :enoent} ->
        conn
        |> put_status(:not_found)
        |> put_resp_content_type("text/plain")
        |> send_resp(404, "Transcript file '#{filename}.txt' not found")
      {:error, reason} ->
        conn
        |> put_status(:internal_server_error)
        |> put_resp_content_type("text/plain")
        |> send_resp(500, "Error deleting transcript file '#{filename}.txt': #{reason}")
    end
  end

  def queue(conn, _params) do
    queue_status = Autotranscript.VideoProcessor.get_queue_status()

    # Transform tuples to maps for JSON serialization
    transformed_queue_status = %{
      queue: Enum.map(queue_status.queue, fn {video_path, process_type} ->
        %{
          video_path: video_path,
          process_type: process_type
        }
      end),
      processing: queue_status.processing,
      current_file: case queue_status.current_file do
        {video_path, process_type} -> %{video_path: video_path, process_type: process_type}
        nil -> nil
      end
    }

    conn
    |> put_resp_content_type("application/json")
    |> send_resp(200, Jason.encode!(transformed_queue_status))
  end

  def files(conn, _params) do
    video_files = VideoInfoCache.get_video_files()

    conn
    |> put_resp_content_type("application/json")
    |> send_resp(200, Jason.encode!(video_files))
  end



  def random_frame(conn, _params) do
    watch_directory = Autotranscript.ConfigManager.get_config_value("watch_directory")

    if watch_directory == nil or watch_directory == "" do
      conn
      |> put_status(:service_unavailable)
      |> put_resp_content_type("text/plain")
      |> send_resp(503, "Watch directory not configured. Please configure the application first.")
    else
      # Get all video files in the watch directory
      mp4_files = Path.wildcard(Path.join(watch_directory, "**/*.#{PathHelper.video_wildcard_pattern}"))

    case mp4_files do
      [] ->
        conn
        |> put_status(:not_found)
        |> put_resp_content_type("text/plain")
        |> send_resp(404, "No MP4 files found in watch directory")

      files ->
        # Randomly select an video file
        selected_file = Enum.random(files)

        # Generate a temporary filename for the extracted frame
        temp_frame_path = Path.join(System.tmp_dir(), "random_frame_#{:rand.uniform(10000)}.jpg")

        # Use ffmpeg to extract a random frame
        case System.cmd("ffmpeg", [
          "-i", selected_file,
          "-vf", "select='gte(n\\,1)'",
          "-vframes", "1",
          "-f", "image2",
          "-y",
          temp_frame_path
        ]) do
          {_output, 0} ->
            # Successfully extracted frame, serve it
            case File.read(temp_frame_path) do
              {:ok, image_data} ->
                # Clean up temp file
                File.rm(temp_frame_path)

                conn
                |> put_resp_content_type("image/jpeg")
                |> send_resp(200, image_data)

              {:error, reason} ->
                conn
                |> put_status(:internal_server_error)
                |> put_resp_content_type("text/plain")
                |> send_resp(500, "Error reading extracted frame: #{reason}")
            end

          {error_output, _exit_code} ->
            # Clean up temp file if it exists
            File.rm(temp_frame_path)

            conn
            |> put_status(:internal_server_error)
            |> put_resp_content_type("text/plain")
            |> send_resp(500, "Error extracting frame with ffmpeg: #{error_output}")
        end
      end
    end
  end

  def player(conn, %{"filename" => filename} = _params) do
    watch_directory = Autotranscript.ConfigManager.get_config_value("watch_directory")

    # Check if the video file exists
    video_exists = PathHelper.video_file_exists?(watch_directory, filename)

    if video_exists do
      # Extract and validate the time parameter
      start_time = case Plug.Conn.fetch_query_params(conn) |> Map.get(:query_params) |> Map.get("time") do
        time_str when is_binary(time_str) ->
          case Float.parse(time_str) do
            {time, ""} when time >= 0 -> time
            _ -> nil
          end
        _ -> nil
      end
      render(conn, :player, filename: filename, start_time: start_time)
    else
      conn
      |> put_status(:not_found)
      |> put_resp_content_type("text/plain")
      |> send_resp(404, "Video file '#{filename}' not found")
    end
  end

  def frame_at_time(conn, %{"filename" => filename, "time" => time_str}) do
    watch_directory = Autotranscript.ConfigManager.get_config_value("watch_directory")

    # Parse and validate the time parameter
    case Float.parse(time_str) do
      {time, ""} when time >= 0 ->
        video_file = PathHelper.find_video_file(watch_directory, filename)

        case video_file do
          nil ->
            conn
            |> put_status(:not_found)
            |> put_resp_content_type("text/plain")
            |> send_resp(404, "Video file '#{filename}' not found")

          file_path ->
            # Extract and validate the text parameter
            text_param = case Plug.Conn.fetch_query_params(conn) |> Map.get(:query_params) |> Map.get("text") do
              text when is_binary(text) and text != "" -> text
              _ -> nil
            end

            # Generate a temporary filename for the extracted frame
            temp_frame_path = Path.join(System.tmp_dir(), "frame_at_time_#{UUID.uuid4()}.jpg")

            # Build ffmpeg command with optional drawtext filter
            ffmpeg_args = case text_param do
              nil ->
                [
                  "-ss", "#{time}",
                  "-i", file_path,
                  "-vframes", "1",
                  "-q:v", "2",
                  "-f", "image2",
                  "-y",
                  temp_frame_path
                ]
              text ->
                # Escape special characters in text for ffmpeg
                escaped_text = text
                  |> :unicode.characters_to_binary(:utf8)

                # Calculate font size based on text length to ensure it fits
                font_size = cond do
                  String.length(text) > 100 -> 48  # Very long text
                  String.length(text) > 50 -> 72   # Long text
                  String.length(text) > 25 -> 96   # Medium text
                  true -> 144                      # Short text
                end

                [
                  "-ss", "#{time}",
                  "-i", file_path,
                  "-vf", "drawtext=text='#{escaped_text}':fontcolor=white:fontsize=#{font_size}:box=1:boxcolor=black@0.5:boxborderw=5:x=(w-text_w)/2:y=h-th-10",
                  "-vframes", "1",
                  "-q:v", "2",
                  "-f", "image2",
                  "-y",
                  temp_frame_path
                ]
            end

            # Use ffmpeg to extract a frame at the specified time
            case System.cmd("ffmpeg", ffmpeg_args) do
              {_output, 0} ->
                # Successfully extracted frame, serve it
                case File.read(temp_frame_path) do
                  {:ok, image_data} ->
                    # Clean up temp file
                    File.rm(temp_frame_path)

                    conn
                    |> put_resp_content_type("image/jpeg")
                    |> send_resp(200, image_data)

                  {:error, reason} ->
                    conn
                    |> put_status(:internal_server_error)
                    |> put_resp_content_type("text/plain")
                    |> send_resp(500, "Error reading extracted frame: #{reason}")
                end

              {error_output, _exit_code} ->
                # Clean up temp file if it exists
                File.rm(temp_frame_path)

                conn
                |> put_status(:internal_server_error)
                |> put_resp_content_type("text/plain")
                |> send_resp(500, "Error extracting frame with ffmpeg: #{error_output}")
            end
        end

      _ ->
        conn
        |> put_status(:bad_request)
        |> put_resp_content_type("text/plain")
        |> send_resp(400, "Invalid time parameter. Must be a non-negative number.")
    end
  end

  def clip(conn, _params) do
    query_params = Plug.Conn.fetch_query_params(conn) |> Map.get(:query_params)
    filename = query_params["filename"]
    start_time_str = query_params["start_time"]
    end_time_str = query_params["end_time"]
    watch_directory = Autotranscript.ConfigManager.get_config_value("watch_directory")

    # Parse and validate the time parameters
    case {Float.parse(start_time_str || ""), Float.parse(end_time_str || "")} do
      {{start_time, ""}, {end_time, ""}} when start_time >= 0 and end_time > start_time ->
        # Check if the video file exists
        video_file = PathHelper.find_video_file(watch_directory, filename)

        case video_file do
          nil ->
            conn
            |> put_status(:not_found)
            |> put_resp_content_type("text/plain")
            |> send_resp(404, "Video file '#{filename}' not found")

          file_path ->
            # Calculate duration
            duration = end_time - start_time

            # Generate a temporary filename for the clipped video
            temp_clip_path = Path.join(System.tmp_dir(), "clip_#{UUID.uuid4()}.mp4")

            # Use ffmpeg to extract the video clip
            case System.cmd("ffmpeg", [
              "-ss", "#{start_time}",
              "-i", file_path,
              "-t", "#{duration}",
              "-c", "copy",
              "-avoid_negative_ts", "make_zero",
              "-y",
              temp_clip_path
            ]) do
              {_output, 0} ->
                # Successfully created clip, serve it
                case File.read(temp_clip_path) do
                  {:ok, video_data} ->
                    # Clean up temp file
                    File.rm(temp_clip_path)

                    conn
                    |> put_resp_content_type("video/mp4")
                    |> send_resp(200, video_data)

                  {:error, reason} ->
                    conn
                    |> put_status(:internal_server_error)
                    |> put_resp_content_type("text/plain")
                    |> send_resp(500, "Error reading clipped video: #{reason}")
                end

              {error_output, _exit_code} ->
                # Clean up temp file if it exists
                File.rm(temp_clip_path)

                conn
                |> put_status(:internal_server_error)
                |> put_resp_content_type("text/plain")
                |> send_resp(500, "Error creating video clip with ffmpeg: #{error_output}")
            end
        end

      _ ->
        conn
        |> put_status(:bad_request)
        |> put_resp_content_type("text/plain")
        |> send_resp(400, "Invalid time parameters. Start time must be non-negative and end time must be greater than start time.")
    end
  end

  def watch_directory(conn, _params) do
    watch_directory = Autotranscript.ConfigManager.get_config_value("watch_directory")
    conn
    |> put_resp_content_type("text/plain")
    |> send_resp(200, watch_directory || "")
  end

  def regenerate_meta(conn, %{"filename" => filename}) do
    watch_directory = Autotranscript.ConfigManager.get_config_value("watch_directory")

    if watch_directory == nil or watch_directory == "" do
      conn
      |> put_status(:service_unavailable)
      |> put_resp_content_type("application/json")
      |> send_resp(503, Jason.encode!(%{error: "Watch directory not configured. Please configure the application first."}))
    else
      # Check if the video file exists
      video_file = PathHelper.find_video_file(watch_directory, filename)

      case video_file do
        nil ->
          conn
          |> put_status(:not_found)
          |> put_resp_content_type("application/json")
          |> send_resp(404, Jason.encode!(%{error: "Video file '#{filename}' not found"}))

        file_path ->
          Autotranscript.VideoProcessor.add_to_queue(file_path, :length)
          conn
          |> put_resp_content_type("application/json")
          |> send_resp(200, Jason.encode!(%{message: "Meta file regeneration for '#{filename}' added to queue"}))
      end
    end
  end

  def replace_transcript(conn, %{"filename" => filename}) do
    watch_directory = Autotranscript.ConfigManager.get_config_value("watch_directory")
    file_path = Path.join(watch_directory, "#{filename}.txt")
    case conn.body_params do
      %{"text" => replacement_text} ->
        case File.write(file_path, replacement_text) do
          :ok ->
            VideoInfoCache.update_video_info_cache
            conn
            |> put_resp_content_type("text/plain")
            |> send_resp(200, "Transcript file '#{filename}.txt' updated successfully")
          {:error, reason} ->
            conn
            |> put_status(:internal_server_error)
            |> put_resp_content_type("text/plain")
            |> send_resp(500, "Error updating transcript file '#{filename}.txt': #{reason}")
        end
      _ ->
        conn
        |> put_status(:bad_request)
        |> put_resp_content_type("text/plain")
        |> send_resp(400, "Missing required 'text' field in request body")
    end
  end

  def partial_reprocess(conn, %{"filename" => filename}) do
    watch_directory = Autotranscript.ConfigManager.get_config_value("watch_directory")

    if watch_directory == nil or watch_directory == "" do
      conn
      |> put_status(:service_unavailable)
      |> put_resp_content_type("application/json")
      |> send_resp(503, Jason.encode!(%{error: "Watch directory not configured. Please configure the application first."}))
    else
      case conn.body_params do
        %{"time" => time_str} ->
          case parse_time_to_seconds(time_str) do
            {:ok, time_seconds} ->
              execute_partial_reprocess(conn, filename, time_str, time_seconds, watch_directory)
            {:error, reason} ->
              conn
              |> put_status(:bad_request)
              |> put_resp_content_type("application/json")
              |> send_resp(400, Jason.encode!(%{error: "Invalid time format: #{reason}"}))
          end
        _ ->
          conn
          |> put_status(:bad_request)
          |> put_resp_content_type("application/json")
          |> send_resp(400, Jason.encode!(%{error: "Missing required 'time' field in request body"}))
      end
    end
  end

  defp execute_partial_reprocess(conn, filename, time_str, time_seconds, watch_directory) do
    txt_file_path = Path.join(watch_directory, "#{filename}.txt")
    video_file = PathHelper.find_video_file(watch_directory, filename)

    case video_file do
      nil ->
        conn
        |> put_status(:not_found)
        |> put_resp_content_type("application/json")
        |> send_resp(404, Jason.encode!(%{error: "Video file '#{filename}' not found"}))

      video_path ->
        case File.exists?(txt_file_path) do
          false ->
            conn
            |> put_status(:not_found)
            |> put_resp_content_type("application/json")
            |> send_resp(404, Jason.encode!(%{error: "Transcript file '#{filename}.txt' not found"}))

          true ->
            case do_partial_reprocess(txt_file_path, video_path, time_str, time_seconds) do
              :ok ->
                VideoInfoCache.update_video_info_cache()
                conn
                |> put_resp_content_type("application/json")
                |> send_resp(200, Jason.encode!(%{message: "Partial reprocessing completed successfully for '#{filename}' from time #{time_str}"}))

              {:error, reason} ->
                conn
                |> put_status(:internal_server_error)
                |> put_resp_content_type("application/json")
                |> send_resp(500, Jason.encode!(%{error: "Error during partial reprocessing: #{reason}"}))
            end
        end
    end
  end

  defp do_partial_reprocess(txt_file_path, video_path, time_str, time_seconds) do
    with :ok <- truncate_txt_file_at_time(txt_file_path, time_str),
         {:ok, temp_video_path} <- create_temp_video_from_time(video_path, time_seconds),
         {:ok, temp_mp3_path} <- convert_temp_video_to_mp3(temp_video_path),
         {:ok, temp_txt_path} <- transcribe_temp_mp3(temp_mp3_path),
         :ok <- cleanup_temp_files([temp_video_path, temp_mp3_path]),
         :ok <- append_temp_txt_to_original(temp_txt_path, txt_file_path),
         :ok <- File.rm(temp_txt_path) do
      :ok
    else
      {:error, reason} -> {:error, reason}
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
          _ -> {:error, "Invalid time format"}
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
          _ -> {:error, "Invalid time format"}
        end

      # Format: seconds only (integer or float)
      true ->
        case Float.parse(time_str) do
          {seconds, ""} when seconds >= 0 -> {:ok, seconds}
          _ -> {:error, "Invalid numeric time"}
        end
    end
  end

  defp truncate_txt_file_at_time(txt_file_path, time_str) do
    case File.read(txt_file_path) do
      {:ok, content} ->
        lines = String.split(content, "\n")
        
        # Find the first line that includes the time
        truncated_lines = Enum.take_while(lines, fn line ->
          not String.contains?(line, time_str)
        end)
        
        # Write the truncated content back
        truncated_content = Enum.join(truncated_lines, "\n")
        case File.write(txt_file_path, truncated_content) do
          :ok -> :ok
          {:error, reason} -> {:error, "Failed to truncate txt file: #{reason}"}
        end

      {:error, reason} -> {:error, "Failed to read txt file: #{reason}"}
    end
  end

  defp create_temp_video_from_time(video_path, time_seconds) do
    temp_video_path = Path.join(System.tmp_dir(), "temp_video_#{UUID.uuid4()}.mp4")
    
    case System.cmd("ffmpeg", [
      "-ss", "#{time_seconds}",
      "-i", video_path,
      "-c", "copy",
      "-avoid_negative_ts", "make_zero",
      "-y",
      temp_video_path
    ]) do
      {_output, 0} -> {:ok, temp_video_path}
      {error_output, _exit_code} -> {:error, "ffmpeg failed: #{error_output}"}
    end
  end

  defp convert_temp_video_to_mp3(temp_video_path) do
    temp_mp3_path = Path.join(System.tmp_dir(), "temp_audio_#{UUID.uuid4()}.mp3")
    
    case System.cmd("ffmpeg", [
      "-i", temp_video_path,
      "-q:a", "0",
      "-map", "a",
      "-y",
      temp_mp3_path
    ]) do
      {_output, 0} -> {:ok, temp_mp3_path}
      {error_output, _exit_code} -> {:error, "ffmpeg audio conversion failed: #{error_output}"}
    end
  end

  defp transcribe_temp_mp3(temp_mp3_path) do
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
        case System.cmd(whispercli, ["-m", model, "-np", "-ovtt", "-f", temp_mp3_path]) do
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
        {:error, :enoent} -> :ok  # File doesn't exist, that's fine
        {:error, reason} -> Logger.warning("Failed to delete temp file #{path}: #{reason}")
      end
    end)
    :ok
  end

  defp append_temp_txt_to_original(temp_txt_path, original_txt_path) do
    case File.read(temp_txt_path) do
      {:ok, content} ->
        lines = String.split(content, "\n")
        # Remove the first line and get the rest
        remaining_lines = case lines do
          [_first | rest] -> rest
          [] -> []
        end
        
        # Append to original file
        content_to_append = case remaining_lines do
          [] -> ""
          lines -> "\n" <> Enum.join(lines, "\n")
        end
        
        case File.open(original_txt_path, [:append]) do
          {:ok, file} ->
            IO.write(file, content_to_append)
            File.close(file)
            :ok
          {:error, reason} -> {:error, "Failed to append to original file: #{reason}"}
        end

      {:error, reason} -> {:error, "Failed to read temp txt file: #{reason}"}
    end
  end
end
