defmodule Autotranscript.Web.TranscriptController do
  use Autotranscript.Web, :controller

  def index(conn, _params) do
    # Get all .txt files in the watch directory (will be empty array if no config)
    txt_files =
      get_mp4_files()
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
      case System.shell("grep -Hni \"" <> search_text <> "\" *.txt", cd: watch_directory) do
      {output, 0} ->
        # Parse the output to extract filename and line numbers
        results = parse_grep_output(output)
        conn
        |> put_resp_content_type("application/json")
        |> send_resp(200, Jason.encode!(results))
      {output, exit_code} when exit_code > 1 ->
        conn
        |> put_status(:internal_server_error)
        |> put_resp_content_type("application/json")
        |> send_resp(500, Jason.encode!(%{error: "Error running grep: #{output}"}))
      {_output, 1} ->
        # grep returns 1 when no matches found, which is not an error
        conn
        |> put_resp_content_type("application/json")
        |> send_resp(200, Jason.encode!(%{}))
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
    conn
    |> put_resp_content_type("application/json")
    |> send_resp(200, Jason.encode!(queue_status))
  end

  def files(conn, _params) do
    mp4_files = get_mp4_files()

    conn
    |> put_resp_content_type("application/json")
    |> send_resp(200, Jason.encode!(mp4_files))
  end

  defp get_mp4_files do
    watch_directory = Autotranscript.ConfigManager.get_config_value("watch_directory")
    
    if watch_directory == nil or watch_directory == "" do
      []
    else
      # Get all .mp4 and .MP4 files in the watch directory
      Path.wildcard(Path.join(watch_directory, "*.{mp4,MP4}"))
      |> Enum.map(fn file_path ->
      case File.stat(file_path) do
        {:ok, stat} ->
          filename = Path.basename(file_path, Path.extname(file_path))
          display_name = Path.basename(file_path)
          txt_path = Path.join(watch_directory, "#{filename}.txt")

          # Check if transcript exists
          transcript_exists = File.exists?(txt_path)

          # If transcript exists, get line count and last modified time
          {line_count, last_generated} = if transcript_exists do
            case File.read(txt_path) do
              {:ok, content} ->
                line_count = length(String.split(content, "\n"))
                case File.stat(txt_path) do
                  {:ok, txt_stat} -> {line_count, txt_stat.mtime |> Autotranscript.Web.TranscriptHTML.format_datetime()}
                  {:error, _} -> {line_count, nil}
                end
              {:error, _} -> {0, nil}
            end
          else
            {0, nil}
          end

          # If transcript exists, try to read video length from meta file
          length = if transcript_exists do
            meta_path = Path.join(watch_directory, "#{filename}.meta")
            case File.read(meta_path) do
              {:ok, length_content} -> String.trim(length_content)
              {:error, _} -> nil
            end
          else
            nil
          end

          %{
            name: display_name,
            base_name: filename,
            created_at: stat.ctime |> Autotranscript.Web.TranscriptHTML.format_datetime(),
            line_count: line_count,
            full_path: file_path,
            transcript: transcript_exists,
            last_generated: last_generated,
            length: length
          }
        {:error, _} ->
          nil
      end
      end)
      |> Enum.reject(&is_nil/1)
      |> Enum.sort_by(& &1.created_at, :desc)
    end
  end

  def random_frame(conn, _params) do
    watch_directory = Autotranscript.ConfigManager.get_config_value("watch_directory")
    
    if watch_directory == nil or watch_directory == "" do
      conn
      |> put_status(:service_unavailable)
      |> put_resp_content_type("text/plain")
      |> send_resp(503, "Watch directory not configured. Please configure the application first.")
    else
      # Get all MP4 files in the watch directory
      mp4_files = Path.wildcard(Path.join(watch_directory, "*.{MP4,mp4}"))

    case mp4_files do
      [] ->
        conn
        |> put_status(:not_found)
        |> put_resp_content_type("text/plain")
        |> send_resp(404, "No MP4 files found in watch directory")

      files ->
        # Randomly select an MP4 file
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
    mp4_path = Path.join(watch_directory, "#{filename}.mp4")
    mp4_upper_path = Path.join(watch_directory, "#{filename}.MP4")

    video_exists = File.exists?(mp4_path) or File.exists?(mp4_upper_path)

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
        # Check if the video file exists
        mp4_path = Path.join(watch_directory, "#{filename}.mp4")
        mp4_upper_path = Path.join(watch_directory, "#{filename}.MP4")

        video_file = cond do
          File.exists?(mp4_path) -> mp4_path
          File.exists?(mp4_upper_path) -> mp4_upper_path
          true -> nil
        end

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
        mp4_path = Path.join(watch_directory, "#{filename}.mp4")
        mp4_upper_path = Path.join(watch_directory, "#{filename}.MP4")

        video_file = cond do
          File.exists?(mp4_path) -> mp4_path
          File.exists?(mp4_upper_path) -> mp4_upper_path
          true -> nil
        end

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
      mp4_path = Path.join(watch_directory, "#{filename}.mp4")
      mp4_upper_path = Path.join(watch_directory, "#{filename}.MP4")

      video_file = cond do
        File.exists?(mp4_path) -> mp4_path
        File.exists?(mp4_upper_path) -> mp4_upper_path
        true -> nil
      end

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
    IO.inspect(conn.body_params)
    case conn.body_params do
      %{"text" => replacement_text} ->
        case File.write(file_path, replacement_text) do
          :ok ->
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
end
