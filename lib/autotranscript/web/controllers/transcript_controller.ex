defmodule Autotranscript.Web.TranscriptController do
  use Autotranscript.Web, :controller
  require Logger

  alias Autotranscript.{PathHelper, VideoInfoCache, ConfigManager}

  # Helper function to decode URL-encoded filenames
  def decode_filename(filename) when is_binary(filename) do
    filename
    |> URI.decode()
    |> String.trim()
  end
  def decode_filename(filename), do: filename
  def index(conn, _params) do
    # Get all .txt files in the watch directory (will be empty array if no config)
    txt_files =
      VideoInfoCache.get_video_files()
      |> Jason.encode!()

    render(conn, :index, txt_files: txt_files)
  end

  def show(conn, %{"filename" => filename}) do
    # Decode the URL-encoded filename
    decoded_filename = decode_filename(filename)

    watch_directory = Autotranscript.ConfigManager.get_config_value("watch_directory")

    if watch_directory == nil or watch_directory == "" do
      conn
      |> put_status(:service_unavailable)
      |> put_resp_content_type("text/plain")
      |> send_resp(503, "Watch directory not configured. Please configure the application first.")
    else
      file_path = Path.join(watch_directory, "#{decoded_filename}.txt")

          case File.read(file_path) do
        {:ok, content} ->
          conn
          |> put_resp_content_type("text/plain")
          |> send_resp(200, content)
        {:error, :enoent} ->
          conn
          |> put_status(:not_found)
          |> put_resp_content_type("text/plain")
          |> send_resp(404, "Transcript file '#{decoded_filename}' not found")
        {:error, reason} ->
          conn
          |> put_status(:internal_server_error)
          |> put_resp_content_type("text/plain")
          |> send_resp(500, "Error reading transcript file '#{decoded_filename}': #{reason}")
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
    decoded_filename = decode_filename(filename)
    watch_directory = Autotranscript.ConfigManager.get_config_value("watch_directory")
    file_path = Path.join(watch_directory, "#{decoded_filename}.txt")

    case File.rm(file_path) do
      :ok ->
        conn
        |> put_resp_content_type("text/plain")
        |> send_resp(200, "Transcript file '#{decoded_filename}.txt' deleted for regeneration")
      {:error, :enoent} ->
        conn
        |> put_status(:not_found)
        |> put_resp_content_type("text/plain")
        |> send_resp(404, "Transcript file '#{decoded_filename}.txt' not found")
      {:error, reason} ->
        conn
        |> put_status(:internal_server_error)
        |> put_resp_content_type("text/plain")
        |> send_resp(500, "Error deleting transcript file '#{decoded_filename}.txt': #{reason}")
    end
  end

  def queue(conn, _params) do
    queue_status = Autotranscript.VideoProcessor.get_queue_status()

    # Transform tuples to maps for JSON serialization
    transformed_queue_status = %{
      queue: Enum.map(queue_status.queue, fn {process_type, %{path: video_path, time: time}} ->
        %{
          video_path: video_path,
          process_type: process_type,
          time: time
        }
      end),
      processing: queue_status.processing,
      current_file: case queue_status.current_file do
        {process_type, %{path: video_path, time: time}} -> %{video_path: video_path, process_type: process_type, time: time}
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
        ffmpeg_path = ConfigManager.get_config_value("ffmpeg_path") || "ffmpeg"
        case System.cmd(ffmpeg_path, [
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
    decoded_filename = decode_filename(filename)
    watch_directory = Autotranscript.ConfigManager.get_config_value("watch_directory")

    # Check if the video file exists
    video_exists = PathHelper.video_file_exists?(watch_directory, decoded_filename)

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
      render(conn, :player, filename: decoded_filename, start_time: start_time)
    else
      conn
      |> put_status(:not_found)
      |> put_resp_content_type("text/plain")
      |> send_resp(404, "Video file '#{decoded_filename}' not found")
    end
  end

  def clip_player(conn, %{"filename" => filename} = _params) do
    decoded_filename = decode_filename(filename)
    watch_directory = Autotranscript.ConfigManager.get_config_value("watch_directory")

    # Check if the video file exists
    video_exists = PathHelper.video_file_exists?(watch_directory, decoded_filename)

    if video_exists do
      # Extract query parameters
      query_params = Plug.Conn.fetch_query_params(conn) |> Map.get(:query_params)

      start_time = query_params["start_time"]
      end_time = query_params["end_time"]
      text = query_params["text"]
      font_size = query_params["font_size"]
      display_text = query_params["display_text"]

      # Validate required parameters
      case {start_time, end_time} do
        {start_str, end_str} when is_binary(start_str) and is_binary(end_str) ->
          # Build the clip URL
          clip_params = %{
            "filename" => decoded_filename,
            "start_time" => start_str,
            "end_time" => end_str
          }

          # Add optional parameters if present
          clip_params = if text && text != "", do: Map.put(clip_params, "text", text), else: clip_params
          clip_params = if font_size && font_size != "", do: Map.put(clip_params, "font_size", font_size), else: clip_params
          clip_params = if display_text && display_text != "", do: Map.put(clip_params, "display_text", display_text), else: clip_params

          # Construct the clip URL
          clip_url = "/clip?" <> URI.encode_query(clip_params)

          render(conn, :clip_player,
            filename: decoded_filename,
            clip_url: clip_url,
            start_time: start_time,
            end_time: end_time,
            text: text || "",
            font_size: font_size || "",
            display_text: display_text || ""
          )

        _ ->
          conn
          |> put_status(:bad_request)
          |> put_resp_content_type("text/plain")
          |> send_resp(400, "Missing required start_time and end_time parameters")
      end
    else
      conn
      |> put_status(:not_found)
      |> put_resp_content_type("text/plain")
      |> send_resp(404, "Video file '#{decoded_filename}' not found")
    end
  end

  def frame_at_time(conn, %{"filename" => filename, "time" => time_str}) do
    decoded_filename = decode_filename(filename)
    watch_directory = Autotranscript.ConfigManager.get_config_value("watch_directory")

    # Parse and validate the time parameter
    case Float.parse(time_str) do
      {time, ""} when time >= 0 ->
        video_file = PathHelper.find_video_file(watch_directory, decoded_filename)

        case video_file do
          nil ->
            conn
            |> put_status(:not_found)
            |> put_resp_content_type("text/plain")
            |> send_resp(404, "Video file '#{decoded_filename}' not found")

          file_path ->
            # Extract and validate the text parameter
            query_params = Plug.Conn.fetch_query_params(conn) |> Map.get(:query_params)
            text_param = case Map.get(query_params, "text") do
              text when is_binary(text) and text != "" -> text
              _ -> nil
            end

            # Extract and validate the font_size parameter
            font_size_param = case Map.get(query_params, "font_size") do
              size_str when is_binary(size_str) ->
                case Integer.parse(size_str) do
                  {size, ""} when size > 0 and size <= 500 -> size
                  _ -> nil
                end
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

                # Use provided font size or calculate based on video dimensions and text length
                font_size = case font_size_param do
                  nil ->
                    calculate_font_size_for_video(file_path, String.length(text))
                  size -> size
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
            ffmpeg_path = ConfigManager.get_config_value("ffmpeg_path") || "ffmpeg"
            case System.cmd(ffmpeg_path, ffmpeg_args) do
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
    decoded_filename = decode_filename(filename || "")

    start_time_str = query_params["start_time"]
    end_time_str = query_params["end_time"]
    text_param = query_params["text"]
    display_text_param = query_params["display_text"]
    font_size_param = query_params["font_size"]
    watch_directory = Autotranscript.ConfigManager.get_config_value("watch_directory")

    # Parse and validate the time parameters
    case {Float.parse(start_time_str || ""), Float.parse(end_time_str || "")} do
      {{start_time, ""}, {end_time, ""}} when start_time >= 0 and end_time > start_time ->
        # Check if the video file exists
        video_file = PathHelper.find_video_file(watch_directory, decoded_filename)

        case video_file do
          nil ->
            conn
            |> put_status(:not_found)
            |> put_resp_content_type("text/plain")
            |> send_resp(404, "Video file '#{decoded_filename}' not found")

          file_path ->
            # Calculate duration
            duration = end_time - start_time

            # Generate a temporary filename for the clipped video
            temp_clip_path = Path.join(System.tmp_dir(), "clip_#{UUID.uuid4()}.mp4")

            # Build ffmpeg command with optional text overlay (only if display_text is true)
            ffmpeg_args = case {text_param, display_text_param} do
              {text, "true"} when is_binary(text) and text != "" ->
                # Escape special characters in text for ffmpeg
                escaped_text = text
                  |> :unicode.characters_to_binary(:utf8)

                # Use provided font size or calculate based on text length
                font_size = case font_size_param do
                  size_str when is_binary(size_str) ->
                    case Integer.parse(size_str) do
                      {size, ""} when size > 0 and size <= 500 -> size
                      _ -> nil
                    end
                  _ -> nil
                end

                font_size = case font_size do
                  nil ->
                    calculate_font_size_for_video(file_path, String.length(text))
                  size -> size
                end

                [
                  "-ss", "#{start_time}",
                  "-i", file_path,
                  "-t", "#{duration}",
                  "-vf", "drawtext=text='#{escaped_text}':fontcolor=white:fontsize=#{font_size}:box=1:boxcolor=black@0.5:boxborderw=5:x=(w-text_w)/2:y=h-th-10",
                  "-c:v", "libx264",
                  "-c:a", "aac",
                  "-profile:a", "aac_low",
                  "-ar", "44100",
                  "-ac", "2",
                  "-b:a", "256k",
                  "-crf", "23",
                  "-preset", "medium",
                  "-movflags", "faststart",
                  "-avoid_negative_ts", "make_zero",
                  "-y",
                  temp_clip_path
                ]
              _ ->
                [
                  "-ss", "#{start_time}",
                  "-i", file_path,
                  "-t", "#{duration}",
                  "-c:v", "libx264",
                  "-c:a", "aac",
                  "-profile:a", "aac_low",
                  "-ar", "44100",
                  "-ac", "2",
                  "-b:a", "256k",
                  "-crf", "23",
                  "-preset", "medium",
                  "-movflags", "faststart",
                  "-avoid_negative_ts", "make_zero",
                  "-y",
                  temp_clip_path
                ]
            end

            # Use ffmpeg to extract and convert the video clip to MP4
            ffmpeg_path = ConfigManager.get_config_value("ffmpeg_path") || "ffmpeg"
            case System.cmd(ffmpeg_path, ffmpeg_args) do
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
    decoded_filename = decode_filename(filename)
    watch_directory = Autotranscript.ConfigManager.get_config_value("watch_directory")

    if watch_directory == nil or watch_directory == "" do
      conn
      |> put_status(:service_unavailable)
      |> put_resp_content_type("application/json")
      |> send_resp(503, Jason.encode!(%{error: "Watch directory not configured. Please configure the application first."}))
    else
      # Check if the video file exists
      video_file = PathHelper.find_video_file(watch_directory, decoded_filename)

      case video_file do
        nil ->
          conn
          |> put_status(:not_found)
          |> put_resp_content_type("application/json")
          |> send_resp(404, Jason.encode!(%{error: "Video file '#{decoded_filename}' not found"}))

        file_path ->
          Autotranscript.VideoProcessor.add_to_queue(file_path, :length)
          conn
          |> put_resp_content_type("application/json")
          |> send_resp(200, Jason.encode!(%{message: "Meta file regeneration for '#{decoded_filename}' added to queue"}))
      end
    end
  end

  def replace_transcript(conn, %{"filename" => filename}) do
    decoded_filename = decode_filename(filename)
    watch_directory = Autotranscript.ConfigManager.get_config_value("watch_directory")
    file_path = Path.join(watch_directory, "#{decoded_filename}.txt")
    case conn.body_params do
      %{"text" => replacement_text} ->
        case File.write(file_path, replacement_text, [:utf8]) do
          :ok ->
            VideoInfoCache.update_video_info_cache
            conn
            |> put_resp_content_type("text/plain")
            |> send_resp(200, "Transcript file '#{decoded_filename}.txt' updated successfully")
          {:error, reason} ->
            conn
            |> put_status(:internal_server_error)
            |> put_resp_content_type("text/plain")
            |> send_resp(500, "Error updating transcript file '#{decoded_filename}.txt': #{reason}")
        end
      _ ->
        conn
        |> put_status(:bad_request)
        |> put_resp_content_type("text/plain")
        |> send_resp(400, "Missing required 'text' field in request body")
    end
  end

  def partial_reprocess(conn, %{"filename" => filename}) do
    decoded_filename = decode_filename(filename)
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
              execute_partial_reprocess(conn, decoded_filename, time_str, time_seconds, watch_directory)
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

  defp execute_partial_reprocess(conn, decoded_filename, time_str, _time_seconds, watch_directory) do
    txt_file_path = Path.join(watch_directory, "#{decoded_filename}.txt")
    video_file = PathHelper.find_video_file(watch_directory, decoded_filename)

    case video_file do
      nil ->
        conn
        |> put_status(:not_found)
        |> put_resp_content_type("application/json")
        |> send_resp(404, Jason.encode!(%{error: "Video file '#{decoded_filename}' not found"}))

      video_path ->
        case File.exists?(txt_file_path) do
          false ->
            conn
            |> put_status(:not_found)
            |> put_resp_content_type("application/json")
            |> send_resp(404, Jason.encode!(%{error: "Transcript file '#{decoded_filename}.txt' not found"}))

          true ->
            # Add the partial reprocessing job to the queue
            Autotranscript.VideoProcessor.add_to_queue(video_path, :partial, %{time: time_str})

            conn
            |> put_resp_content_type("application/json")
            |> send_resp(200, Jason.encode!(%{message: "Partial reprocessing for '#{decoded_filename}' from time #{time_str} has been added to the queue"}))
        end
    end
  end

  defp calculate_font_size_for_video(video_path, text_length) do
    # Get video dimensions using ffprobe
    ffprobe_path = ConfigManager.get_config_value("ffprobe_path") || "ffprobe"
    case System.cmd(ffprobe_path, [
      "-v", "error",
      "-select_streams", "v:0",
      "-show_entries", "stream=width,height",
      "-of", "csv=p=0",
      video_path
    ]) do
      {output, 0} ->
        case String.trim(output) |> String.split(",") do
          [width_str, height_str] ->
            case {Integer.parse(width_str), Integer.parse(height_str)} do
              {{_width, ""}, {height, ""}} ->
                # Calculate font size based on video dimensions and text length
                # Base font size is proportional to video height
                base_font_size = max(24, trunc(height / 20))

                # Adjust based on text length
                length_factor = cond do
                  text_length > 100 -> 0.6   # Very long text - smaller
                  text_length > 50 -> 0.8    # Long text - slightly smaller
                  text_length > 25 -> 1.0    # Medium text - base size
                  true -> 1.4               # Short text - larger
                end

                # Ensure font size is within reasonable bounds
                font_size = trunc(base_font_size * length_factor)
                max(24, min(font_size, 200))
              _ ->
                # Fallback to old logic if parsing fails
                fallback_font_size_by_text_length(text_length)
            end
          _ ->
            # Fallback to old logic if output format is unexpected
            fallback_font_size_by_text_length(text_length)
        end
      _ ->
        # Fallback to old logic if ffprobe fails
        fallback_font_size_by_text_length(text_length)
    end
  end

  defp fallback_font_size_by_text_length(text_length) do
    cond do
      text_length > 100 -> 48  # Very long text
      text_length > 50 -> 72   # Long text
      text_length > 25 -> 96   # Medium text
      true -> 144              # Short text
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

  def set_line(conn, %{"filename" => filename}) do
    decoded_filename = decode_filename(filename)
    watch_directory = Autotranscript.ConfigManager.get_config_value("watch_directory")

    if watch_directory == nil or watch_directory == "" do
      conn
      |> put_status(:service_unavailable)
      |> put_resp_content_type("application/json")
      |> send_resp(503, Jason.encode!(%{error: "Watch directory not configured. Please configure the application first."}))
    else
      case conn.body_params do
        %{"line_number" => line_number_str, "text" => new_text} ->
          case Integer.parse(line_number_str) do
            {line_number, ""} when line_number > 0 ->
              file_path = Path.join(watch_directory, "#{decoded_filename}.txt")

              case File.read(file_path) do
                {:ok, content} ->
                  lines = String.split(content, "\n")

                  # Check if line number is within bounds
                  if line_number <= length(lines) do
                    # Replace the line (convert to 0-based indexing)
                    updated_lines = List.replace_at(lines, line_number - 1, new_text)
                    updated_content = Enum.join(updated_lines, "\n")

                    case File.write(file_path, updated_content, [:utf8]) do
                      :ok ->
                        VideoInfoCache.update_video_info_cache()
                        conn
                        |> put_resp_content_type("application/json")
                        |> send_resp(200, Jason.encode!(%{message: "Line #{line_number} in transcript '#{decoded_filename}.txt' updated successfully"}))
                      {:error, reason} ->
                        conn
                        |> put_status(:internal_server_error)
                        |> put_resp_content_type("application/json")
                        |> send_resp(500, Jason.encode!(%{error: "Error updating transcript file '#{decoded_filename}.txt': #{reason}"}))
                    end
                  else
                    conn
                    |> put_status(:bad_request)
                    |> put_resp_content_type("application/json")
                    |> send_resp(400, Jason.encode!(%{error: "Line number #{line_number} is out of bounds. File has #{length(lines)} lines."}))
                  end
                {:error, :enoent} ->
                  conn
                  |> put_status(:not_found)
                  |> put_resp_content_type("application/json")
                  |> send_resp(404, Jason.encode!(%{error: "Transcript file '#{decoded_filename}.txt' not found"}))
                {:error, reason} ->
                  conn
                  |> put_status(:internal_server_error)
                  |> put_resp_content_type("application/json")
                  |> send_resp(500, Jason.encode!(%{error: "Error reading transcript file '#{decoded_filename}.txt': #{reason}"}))
              end
            _ ->
              conn
              |> put_status(:bad_request)
              |> put_resp_content_type("application/json")
              |> send_resp(400, Jason.encode!(%{error: "Invalid line number. Must be a positive integer."}))
          end
        _ ->
          conn
          |> put_status(:bad_request)
          |> put_resp_content_type("application/json")
          |> send_resp(400, Jason.encode!(%{error: "Missing required 'line_number' and 'text' fields in request body"}))
      end
    end
  end

  def get_meta_file(conn, %{"filename" => filename}) do
    decoded_filename = decode_filename(filename)
    watch_directory = Autotranscript.ConfigManager.get_config_value("watch_directory")
    
    if watch_directory == nil or watch_directory == "" do
      conn
      |> put_status(:service_unavailable)
      |> put_resp_content_type("application/json")
      |> send_resp(503, Jason.encode!(%{error: "Watch directory not configured. Please configure the application first."}))
    else
      # Find the video file first
      video_file = PathHelper.find_video_file(watch_directory, decoded_filename)
      
      case video_file do
        nil ->
          conn
          |> put_status(:not_found)
          |> put_resp_content_type("application/json")
          |> send_resp(404, Jason.encode!(%{error: "Video file '#{decoded_filename}' not found"}))
          
        file_path ->
          # Get the meta file path
          meta_path = PathHelper.replace_video_extension_with(file_path, ".meta")
          
          # Read the meta file content as raw text
          case File.read(meta_path) do
            {:ok, content} ->
              conn
              |> put_resp_content_type("application/json")
              |> send_resp(200, Jason.encode!(%{content: content}))
              
            {:error, :enoent} ->
              # Meta file doesn't exist, return empty content
              conn
              |> put_resp_content_type("application/json")
              |> send_resp(200, Jason.encode!(%{content: ""}))
              
            {:error, reason} ->
              conn
              |> put_status(:internal_server_error)
              |> put_resp_content_type("application/json")
              |> send_resp(500, Jason.encode!(%{error: "Error reading meta file: #{reason}"}))
          end
      end
    end
  end
  
  def set_meta_file(conn, %{"filename" => filename}) do
    decoded_filename = decode_filename(filename)
    watch_directory = Autotranscript.ConfigManager.get_config_value("watch_directory")
    
    if watch_directory == nil or watch_directory == "" do
      conn
      |> put_status(:service_unavailable)
      |> put_resp_content_type("application/json")
      |> send_resp(503, Jason.encode!(%{error: "Watch directory not configured. Please configure the application first."}))
    else
      case conn.body_params do
        %{"content" => content} ->
          # Find the video file first
          video_file = PathHelper.find_video_file(watch_directory, decoded_filename)
          
          case video_file do
            nil ->
              conn
              |> put_status(:not_found)
              |> put_resp_content_type("application/json")
              |> send_resp(404, Jason.encode!(%{error: "Video file '#{decoded_filename}' not found"}))
              
            file_path ->
              # Get the meta file path
              meta_path = PathHelper.replace_video_extension_with(file_path, ".meta")
              
              # Write the content to the meta file
              case File.write(meta_path, content, [:utf8]) do
                :ok ->
                  VideoInfoCache.update_video_info_cache()
                  conn
                  |> put_resp_content_type("application/json")
                  |> send_resp(200, Jason.encode!(%{message: "Meta file '#{decoded_filename}.meta' updated successfully"}))
                  
                {:error, reason} ->
                  conn
                  |> put_status(:internal_server_error)
                  |> put_resp_content_type("application/json")
                  |> send_resp(500, Jason.encode!(%{error: "Error updating meta file '#{decoded_filename}.meta': #{reason}"}))
              end
          end
          
        _ ->
          conn
          |> put_status(:bad_request)
          |> put_resp_content_type("application/json")
          |> send_resp(400, Jason.encode!(%{error: "Missing required 'content' field in request body"}))
      end
    end
  end
end
