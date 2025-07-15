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

  # Helper function to search for a transcript file across all watch directories
  defp find_transcript_file(watch_directories, decoded_filename) do
    watch_directories
    |> Enum.find_value(fn watch_directory ->
      file_path = Path.join(watch_directory, "#{decoded_filename}.txt")
      if File.exists?(file_path), do: file_path, else: nil
    end)
  end

  # Helper function to search for a video file across all watch directories
  defp find_video_file_in_watch_directories(watch_directories, decoded_filename) do
    watch_directories
    |> Enum.find_value(fn watch_directory ->
      PathHelper.find_video_file(watch_directory, decoded_filename)
    end)
  end

  # Helper function to check if a video file exists in any of the watch directories
  defp video_file_exists_in_watch_directories?(watch_directories, decoded_filename) do
    watch_directories
    |> Enum.any?(fn watch_directory ->
      PathHelper.video_file_exists?(watch_directory, decoded_filename)
    end)
  end

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

    watch_directories = Autotranscript.ConfigManager.get_config_value("watch_directories") || []

    if Enum.empty?(watch_directories) do
      conn
      |> put_status(:service_unavailable)
      |> put_resp_content_type("text/plain")
      |> send_resp(
        503,
        "Watch directories not configured. Please configure the application first."
      )
    else
      # Search for the transcript file in all watch directories
      file_path = find_transcript_file(watch_directories, decoded_filename)

      case file_path do
        nil ->
          conn
          |> put_status(:not_found)
          |> put_resp_content_type("text/plain")
          |> send_resp(404, "Transcript file '#{decoded_filename}' not found")

        path ->
          case File.read(path) do
            {:ok, content} ->
              conn
              |> put_resp_content_type("text/plain")
              |> send_resp(200, content)

            {:error, reason} ->
              conn
              |> put_status(:internal_server_error)
              |> put_resp_content_type("text/plain")
              |> send_resp(500, "Error reading transcript file '#{decoded_filename}': #{reason}")
          end
      end
    end
  end

  def grep(conn, %{"text" => search_text}) do
    watch_directories = Autotranscript.ConfigManager.get_config_value("watch_directories") || []

    if Enum.empty?(watch_directories) do
      conn
      |> put_status(:service_unavailable)
      |> put_resp_content_type("application/json")
      |> send_resp(
        503,
        Jason.encode!(%{
          error: "Watch directories not configured. Please configure the application first."
        })
      )
    else
      # Search across all watch directories
      all_results =
        watch_directories
        |> Enum.map(fn watch_directory ->
          # Run grep commands for each directory
          {output1, exit_code1} =
            System.shell("grep -Hni \"" <> search_text <> "\" *.txt", cd: watch_directory)

          {output2, exit_code2} =
            System.shell("grep -Hni \"" <> search_text <> "\" **/*.txt", cd: watch_directory)

          # Parse outputs for this directory
          results1 = if exit_code1 <= 1, do: parse_grep_output(output1), else: %{}
          results2 = if exit_code2 <= 1, do: parse_grep_output(output2), else: %{}

          # Merge results for this directory
          Map.merge(results1, results2, fn _k, v1, v2 ->
            Enum.sort(Enum.uniq(v1 ++ v2))
          end)
        end)
        |> Enum.reduce(%{}, fn dir_results, acc ->
          # Merge all directory results together
          Map.merge(acc, dir_results, fn _k, v1, v2 ->
            Enum.sort(Enum.uniq(v1 ++ v2))
          end)
        end)

      conn
      |> put_resp_content_type("application/json")
      |> send_resp(200, Jason.encode!(all_results))
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
    watch_directories = Autotranscript.ConfigManager.get_config_value("watch_directories") || []

    if Enum.empty?(watch_directories) do
      conn
      |> put_status(:service_unavailable)
      |> put_resp_content_type("text/plain")
      |> send_resp(
        503,
        "Watch directories not configured. Please configure the application first."
      )
    else
      # Search for the transcript file in all watch directories
      file_path = find_transcript_file(watch_directories, decoded_filename)

      case file_path do
        nil ->
          conn
          |> put_status(:not_found)
          |> put_resp_content_type("text/plain")
          |> send_resp(404, "Transcript file '#{decoded_filename}.txt' not found")

        path ->
          case File.rm(path) do
            :ok ->
              conn
              |> put_resp_content_type("text/plain")
              |> send_resp(
                200,
                "Transcript file '#{decoded_filename}.txt' deleted for regeneration"
              )

            {:error, reason} ->
              conn
              |> put_status(:internal_server_error)
              |> put_resp_content_type("text/plain")
              |> send_resp(
                500,
                "Error deleting transcript file '#{decoded_filename}.txt': #{reason}"
              )
          end
      end
    end
  end

  def queue(conn, _params) do
    queue_status = Autotranscript.VideoProcessor.get_queue_status()

    # Transform tuples to maps for JSON serialization
    transformed_queue_status = %{
      queue:
        Enum.map(queue_status.queue, fn {process_type, %{path: video_path, time: time}} ->
          %{
            video_path: video_path,
            process_type: process_type,
            time: time
          }
        end),
      processing: queue_status.processing,
      current_file:
        case queue_status.current_file do
          {process_type, %{path: video_path, time: time}} ->
            %{video_path: video_path, process_type: process_type, time: time}

          nil ->
            nil
        end
    }

    conn
    |> put_resp_content_type("application/json")
    |> send_resp(200, Jason.encode!(transformed_queue_status))
  end

  def files(conn, params) do
    video_files = VideoInfoCache.get_video_files()
    # Filter by watch directories if provided
    filtered_files =
      case params["watch_directories"] do
        nil ->
          video_files

        "" ->
          video_files

        watch_dirs_param ->
          # Parse watch directories from comma-separated string
          watch_dirs =
            String.split(watch_dirs_param, ",")
            |> Enum.map(&String.trim/1)
            |> Enum.reject(&(&1 == ""))

          configured_watch_dirs =
            Autotranscript.ConfigManager.get_config_value("watch_directories") || []

          if Enum.empty?(watch_dirs) or Enum.empty?(configured_watch_dirs) do
            video_files
          else
            Enum.filter(video_files, fn file ->
              # Check which configured watch directory this file belongs to
              file_watch_dir =
                Enum.find(configured_watch_dirs, fn watch_dir ->
                  String.starts_with?(file.full_path, watch_dir)
                end)

              # Include file if its watch directory is in the filter
              file_watch_dir && Enum.member?(watch_dirs, file_watch_dir)
            end)
          end
      end

    # Filter by sources if provided
    final_filtered_files =
      case params["sources"] do
        nil ->
          filtered_files

        "" ->
          filtered_files

        sources_param ->
          # Parse sources from comma-separated string
          sources =
            String.split(sources_param, ",")
            |> Enum.map(&String.trim/1)
            |> Enum.reject(&(&1 == ""))

          if Enum.empty?(sources) do
            filtered_files
          else
            Enum.filter(filtered_files, fn file ->
              # Include file if its source (model field) is in the filter
              # Handle nil sources by checking if nil is in the filter
              file.model && Enum.member?(sources, file.model)
            end)
          end
      end

    conn
    |> put_resp_content_type("application/json")
    |> send_resp(200, Jason.encode!(final_filtered_files))
  end

  def sources(conn, _params) do
    video_files = VideoInfoCache.get_video_files()

    # Get unique sources from the model field, filtering out nil values
    unique_sources =
      video_files
      |> Enum.map(& &1.model)
      |> Enum.reject(&is_nil/1)
      |> Enum.uniq()
      |> Enum.sort()

    conn
    |> put_resp_content_type("application/json")
    |> send_resp(200, Jason.encode!(unique_sources))
  end

  def random_frame(conn, _params) do
    watch_directories = Autotranscript.ConfigManager.get_config_value("watch_directories") || []

    if Enum.empty?(watch_directories) do
      conn
      |> put_status(:service_unavailable)
      |> put_resp_content_type("text/plain")
      |> send_resp(
        503,
        "Watch directories not configured. Please configure the application first."
      )
    else
      # Get all video files from all watch directories
      mp4_files =
        watch_directories
        |> Enum.flat_map(fn watch_directory ->
          Path.wildcard(Path.join(watch_directory, "**/*.#{PathHelper.video_wildcard_pattern()}"))
        end)

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
          temp_frame_path =
            Path.join(System.tmp_dir(), "random_frame_#{:rand.uniform(10000)}.jpg")

          # Use ffmpeg to extract a random frame
          ffmpeg_path = ConfigManager.get_config_value("ffmpeg_path") || "ffmpeg"

          case System.cmd(ffmpeg_path, [
                 "-i",
                 selected_file,
                 "-vf",
                 "select='gte(n\\,1)'",
                 "-vframes",
                 "1",
                 "-f",
                 "image2",
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
    watch_directories = Autotranscript.ConfigManager.get_config_value("watch_directories") || []

    if Enum.empty?(watch_directories) do
      conn
      |> put_status(:service_unavailable)
      |> put_resp_content_type("text/plain")
      |> send_resp(
        503,
        "Watch directories not configured. Please configure the application first."
      )
    else
      # Check if the video file exists in any watch directory
      video_exists = video_file_exists_in_watch_directories?(watch_directories, decoded_filename)

      if video_exists do
        # Extract and validate the time parameter
        start_time =
          case Plug.Conn.fetch_query_params(conn) |> Map.get(:query_params) |> Map.get("time") do
            time_str when is_binary(time_str) ->
              case Float.parse(time_str) do
                {time, ""} when time >= 0 -> time
                _ -> nil
              end

            _ ->
              nil
          end

        render(conn, :player, filename: decoded_filename, start_time: start_time)
      else
        conn
        |> put_status(:not_found)
        |> put_resp_content_type("text/plain")
        |> send_resp(404, "Video file '#{decoded_filename}' not found")
      end
    end
  end

  def clip_player(conn, %{"filename" => filename} = _params) do
    decoded_filename = decode_filename(filename)
    watch_directories = Autotranscript.ConfigManager.get_config_value("watch_directories") || []

    if Enum.empty?(watch_directories) do
      conn
      |> put_status(:service_unavailable)
      |> put_resp_content_type("text/plain")
      |> send_resp(
        503,
        "Watch directories not configured. Please configure the application first."
      )
    else
      # Check if the video file exists in any watch directory
      video_exists = video_file_exists_in_watch_directories?(watch_directories, decoded_filename)

      if video_exists do
        # Load transcript data
        transcript_data = case find_transcript_file(watch_directories, decoded_filename) do
          nil -> ""
          path -> 
            case File.read(path) do
              {:ok, content} -> content
              {:error, _reason} -> ""
            end
        end

        # Extract query parameters
        query_params = Plug.Conn.fetch_query_params(conn) |> Map.get(:query_params)

        start_time = query_params["start_time"]
        end_time = query_params["end_time"]
        text = query_params["text"]
        font_size = query_params["font_size"]
        display_text = query_params["display_text"]
        format = query_params["format"] || "mp4"

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
            clip_params =
              if text && text != "", do: Map.put(clip_params, "text", text), else: clip_params

            clip_params =
              if font_size && font_size != "",
                do: Map.put(clip_params, "font_size", font_size),
                else: clip_params

            clip_params =
              if display_text && display_text != "",
                do: Map.put(clip_params, "display_text", display_text),
                else: clip_params

            clip_params =
              if format && format != "",
                do: Map.put(clip_params, "format", format),
                else: clip_params

            # Construct the clip URL
            clip_url = "/clip?" <> URI.encode_query(clip_params)

            render(conn, :clip_player,
              filename: decoded_filename,
              clip_url: clip_url,
              start_time: start_time,
              end_time: end_time,
              text: text || "",
              font_size: font_size || "",
              display_text: display_text || "",
              format: "mp4",
              transcript: transcript_data
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
  end

  def frame_at_time(conn, %{"filename" => filename, "time" => time_str}) do
    decoded_filename = decode_filename(filename)
    watch_directories = Autotranscript.ConfigManager.get_config_value("watch_directories") || []

    if Enum.empty?(watch_directories) do
      conn
      |> put_status(:service_unavailable)
      |> put_resp_content_type("text/plain")
      |> send_resp(
        503,
        "Watch directories not configured. Please configure the application first."
      )
    else
      # Parse and validate the time parameter
      case Float.parse(time_str) do
        {time, ""} when time >= 0 ->
          video_file = find_video_file_in_watch_directories(watch_directories, decoded_filename)

          case video_file do
            nil ->
              conn
              |> put_status(:not_found)
              |> put_resp_content_type("text/plain")
              |> send_resp(404, "Video file '#{decoded_filename}' not found")

            file_path ->
              # Extract and validate the text parameter
              query_params = Plug.Conn.fetch_query_params(conn) |> Map.get(:query_params)

              text_param =
                case Map.get(query_params, "text") do
                  text when is_binary(text) and text != "" -> text
                  _ -> nil
                end

              # Extract and validate the font_size parameter
              font_size_param =
                case Map.get(query_params, "font_size") do
                  size_str when is_binary(size_str) ->
                    case Integer.parse(size_str) do
                      {size, ""} when size > 0 and size <= 500 -> size
                      _ -> nil
                    end

                  _ ->
                    nil
                end

              # Generate a temporary filename for the extracted frame
              temp_frame_path = Path.join(System.tmp_dir(), "frame_at_time_#{UUID.uuid4()}.jpg")

              # Build ffmpeg command with optional drawtext filter
              {ffmpeg_args, temp_text_path} =
                case text_param do
                  nil ->
                    {[
                       "-ss",
                       "#{time}",
                       "-i",
                       file_path,
                       "-vframes",
                       "1",
                       "-q:v",
                       "2",
                       "-f",
                       "image2",
                       "-y",
                       temp_frame_path
                     ], nil}

                  text ->
                    # Create a temporary text file for the drawtext filter
                    temp_text_path = Path.join(System.tmp_dir(), "text_#{UUID.uuid4()}.txt")

                    # Write text to temporary file
                    case File.write(temp_text_path, text, [:utf8]) do
                      :ok ->
                        # Use provided font size or calculate based on video dimensions and text length
                        font_size =
                          case font_size_param do
                            nil ->
                              calculate_font_size_for_video(file_path, String.length(text))

                            size ->
                              size
                          end

                        {[
                           "-ss",
                           "#{time}",
                           "-i",
                           file_path,
                           "-vf",
                           "drawtext=textfile='#{temp_text_path}':fontcolor=white:fontsize=#{font_size}:box=1:boxcolor=black@0.5:boxborderw=5:x=(w-text_w)/2:y=h-th-10",
                           "-vframes",
                           "1",
                           "-q:v",
                           "2",
                           "-f",
                           "image2",
                           "-y",
                           temp_frame_path
                         ], temp_text_path}

                      {:error, reason} ->
                        # If we can't write the text file, fall back to no text overlay
                        Logger.warning("Failed to create temporary text file: #{reason}")

                        {[
                           "-ss",
                           "#{time}",
                           "-i",
                           file_path,
                           "-vframes",
                           "1",
                           "-q:v",
                           "2",
                           "-f",
                           "image2",
                           "-y",
                           temp_frame_path
                         ], nil}
                    end
                end

              # Use ffmpeg to extract a frame at the specified time
              ffmpeg_path = ConfigManager.get_config_value("ffmpeg_path") || "ffmpeg"

              case System.cmd(ffmpeg_path, ffmpeg_args) do
                {_output, 0} ->
                  # Successfully extracted frame, serve it
                  case File.read(temp_frame_path) do
                    {:ok, image_data} ->
                      # Clean up temp files
                      File.rm(temp_frame_path)
                      if temp_text_path, do: File.rm(temp_text_path)

                      conn
                      |> put_resp_content_type("image/jpeg")
                      |> send_resp(200, image_data)

                    {:error, reason} ->
                      # Clean up temp files on error
                      File.rm(temp_frame_path)
                      if temp_text_path, do: File.rm(temp_text_path)

                      conn
                      |> put_status(:internal_server_error)
                      |> put_resp_content_type("text/plain")
                      |> send_resp(500, "Error reading extracted frame: #{reason}")
                  end

                {error_output, _exit_code} ->
                  # Clean up temp files if they exist
                  File.rm(temp_frame_path)
                  if temp_text_path, do: File.rm(temp_text_path)

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
    format_param = query_params["format"] || "mp4"
    watch_directories = Autotranscript.ConfigManager.get_config_value("watch_directories") || []

    if Enum.empty?(watch_directories) do
      conn
      |> put_status(:service_unavailable)
      |> put_resp_content_type("text/plain")
      |> send_resp(
        503,
        "Watch directories not configured. Please configure the application first."
      )
    else
      # Parse and validate the time parameters
      case {Float.parse(start_time_str || ""), Float.parse(end_time_str || "")} do
        {{start_time, ""}, {end_time, ""}} when start_time >= 0 and end_time > start_time ->
          # Check if the video file exists in any watch directory
          video_file = find_video_file_in_watch_directories(watch_directories, decoded_filename)

          case video_file do
            nil ->
              conn
              |> put_status(:not_found)
              |> put_resp_content_type("text/plain")
              |> send_resp(404, "Video file '#{decoded_filename}' not found")

            file_path ->
              # Calculate duration
              duration = end_time - start_time

              # Generate a temporary filename for the clipped video, GIF, or MP3
              format =
                case format_param do
                  "gif" -> "gif"
                  "mp3" -> "mp3"
                  _ -> "mp4"
                end

              file_extension =
                case format do
                  "gif" -> ".gif"
                  "mp3" -> ".mp3"
                  _ -> ".mp4"
                end

              temp_clip_path =
                Path.join(System.tmp_dir(), "clip_#{UUID.uuid4()}#{file_extension}")

              # Check if source file needs audio re-encoding based on audio layout
              ffprobe_path = ConfigManager.get_config_value("ffprobe_path") || "ffprobe"

              needs_advanced_audio_reencoding =
                case System.cmd(ffprobe_path, [
                       "-v",
                       "error",
                       "-select_streams",
                       "a:0",
                       "-show_entries",
                       "stream=channel_layout",
                       "-of",
                       "csv=p=0",
                       file_path
                     ]) do
                  {output, 0} ->
                    layout = String.trim(output) |> String.downcase()
                    layout not in ["mono", "stereo"]

                  _ ->
                    # Default to false if ffprobe fails
                    false
                end

              # Check if source file needs at least some audio re-encoding (e.g., MKV files)
              source_extension = Path.extname(file_path) |> String.downcase()

              extension_needs_basic_audio_reencoding =
                source_extension in [".mkv", ".webm", ".avi", ".mov"]

              audio_codec_args =
                if extension_needs_basic_audio_reencoding do
                  if needs_advanced_audio_reencoding do
                    [
                      "-filter:a",
                      "channelmap=FL-FL|FR-FR|FC-FC|LFE-LFE|SL-BL|SR-BR:5.1",
                      "-c:a",
                      "aac",
                      "-b:a",
                      "256k"
                    ]
                  else
                    ["-c:a", "aac", "-b:a", "256k"]
                  end
                else
                  ["-c:a", "copy"]
                end

              # we might even need more though, if it's in 5.1 or 7.1, we need to re-encode it
              # Build ffmpeg command based on format and optional text overlay
              {ffmpeg_args, temp_text_path} =
                case {text_param, display_text_param, format} do
                  {text, "true", "mp4"} when is_binary(text) and text != "" ->
                    # MP4 video with text overlay
                    # Create a temporary text file for the drawtext filter
                    temp_text_path = Path.join(System.tmp_dir(), "text_#{UUID.uuid4()}.txt")

                    # Write text to temporary file
                    case File.write(temp_text_path, text, [:utf8]) do
                      :ok ->
                        font_size =
                          case font_size_param do
                            size_str when is_binary(size_str) ->
                              case Integer.parse(size_str) do
                                {size, ""} when size > 0 and size <= 500 -> size
                                _ -> nil
                              end

                            _ ->
                              nil
                          end

                        font_size =
                          case font_size do
                            nil ->
                              calculate_font_size_for_video(file_path, String.length(text))

                            size ->
                              size
                          end

                        {[
                           "-ss",
                           "#{start_time}",
                           "-t",
                           "#{duration}",
                           "-i",
                           file_path,
                           "-vf",
                           "drawtext=textfile='#{temp_text_path}':fontcolor=white:fontsize=#{font_size}:box=1:boxcolor=black@0.5:boxborderw=5:x=(w-text_w)/2:y=h-th-10",
                           "-c:v",
                           "libx264"
                         ] ++
                           audio_codec_args ++
                           [
                             "-crf",
                             "28",
                             "-preset",
                             "ultrafast",
                             "-movflags",
                             "faststart",
                             "-avoid_negative_ts",
                             "make_zero",
                             "-y",
                             "-map_chapters",
                             "-1",
                             temp_clip_path
                           ], temp_text_path}

                      {:error, reason} ->
                        # If we can't write the text file, fall back to no text overlay
                        Logger.warning("Failed to create temporary text file: #{reason}")

                        {[
                           "-ss",
                           "#{start_time}",
                           "-t",
                           "#{duration}",
                           "-i",
                           file_path,
                           "-c:v",
                           "libx264"
                         ] ++
                           audio_codec_args ++
                           [
                             "-crf",
                             "28",
                             "-preset",
                             "ultrafast",
                             "-movflags",
                             "faststart",
                             "-avoid_negative_ts",
                             "make_zero",
                             "-y",
                             "-map_chapters",
                             "-1",
                             temp_clip_path
                           ], nil}
                    end

                  {text, "true", "gif"} when is_binary(text) and text != "" ->
                    # GIF with text overlay - optimized for speed
                    # Create a temporary text file for the drawtext filter
                    temp_text_path = Path.join(System.tmp_dir(), "text_#{UUID.uuid4()}.txt")

                    # Write text to temporary file
                    case File.write(temp_text_path, text, [:utf8]) do
                      :ok ->
                        font_size =
                          case font_size_param do
                            size_str when is_binary(size_str) ->
                              case Integer.parse(size_str) do
                                {size, ""} when size > 0 and size <= 500 -> size
                                _ -> nil
                              end

                            _ ->
                              nil
                          end

                        font_size =
                          case font_size do
                            nil ->
                              calculate_font_size_for_video(file_path, String.length(text))

                            size ->
                              size
                          end

                        {[
                           "-ss",
                           "#{start_time}",
                           "-t",
                           "#{duration}",
                           "-i",
                           file_path,
                           "-vf",
                           "drawtext=textfile='#{temp_text_path}':fontcolor=white:fontsize=#{font_size}:box=1:boxcolor=black@0.5:boxborderw=5:x=(w-text_w)/2:y=h-th-10,fps=10,scale=480:-1:flags=lanczos,split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse",
                           "-loop",
                           "0",
                           "-y",
                           temp_clip_path
                         ], temp_text_path}

                      {:error, reason} ->
                        # If we can't write the text file, fall back to no text overlay
                        Logger.warning("Failed to create temporary text file: #{reason}")

                        {[
                           "-ss",
                           "#{start_time}",
                           "-t",
                           "#{duration}",
                           "-i",
                           file_path,
                           "-vf",
                           "fps=10,scale=480:-1:flags=lanczos,split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse",
                           "-loop",
                           "0",
                           "-y",
                           temp_clip_path
                         ], nil}
                    end

                  {_, _, "mp4"} ->
                    # MP4 video without text overlay - use stream copying for fast clipping
                    {[
                       "-ss",
                       "#{start_time}",
                       "-t",
                       "#{duration}",
                       "-i",
                       file_path,
                       "-c:v",
                       "libx264"
                     ] ++
                       audio_codec_args ++
                       [
                         "-crf",
                         "28",
                         "-preset",
                         "ultrafast",
                         "-movflags",
                         "faststart",
                         "-avoid_negative_ts",
                         "make_zero",
                         "-y",
                         "-map_chapters",
                         "-1",
                         temp_clip_path
                       ], nil}

                  {_, _, "gif"} ->
                    # GIF without text overlay
                    {[
                       "-ss",
                       "#{start_time}",
                       "-t",
                       "#{duration}",
                       "-i",
                       file_path,
                       "-vf",
                       "fps=8,scale=320:-1:flags=fast_bilinear,split[s0][s1];[s0]palettegen=max_colors=128:stats_mode=single[p];[s1][p]paletteuse=dither=bayer:bayer_scale=2",
                       "-loop",
                       "0",
                       "-y",
                       temp_clip_path
                     ], nil}

                  {_, _, "mp3"} ->
                    # MP3 audio extraction
                    {[
                       "-ss",
                       "#{start_time}",
                       "-t",
                       "#{duration}",
                       "-i",
                       file_path,
                       "-vn",
                       "-acodec",
                       "libmp3lame",
                       "-ar",
                       "44100",
                       "-ac",
                       "2",
                       "-b:a",
                       "256k",
                       "-y",
                       temp_clip_path
                     ], nil}
                end

              # Use ffmpeg to extract and convert the video clip to MP4, GIF, or MP3
              ffmpeg_path = ConfigManager.get_config_value("ffmpeg_path") || "ffmpeg"

              case System.cmd(ffmpeg_path, ffmpeg_args) do
                {_output, 0} ->
                  # Successfully created clip, serve it
                  case File.read(temp_clip_path) do
                    {:ok, clip_data} ->
                      # Clean up temp files
                      File.rm(temp_clip_path)
                      if temp_text_path, do: File.rm(temp_text_path)

                      content_type =
                        case format do
                          "gif" -> "image/gif"
                          "mp3" -> "audio/mpeg"
                          _ -> "video/mp4"
                        end

                      conn
                      |> put_resp_content_type(content_type)
                      |> send_resp(200, clip_data)

                    {:error, reason} ->
                      # Clean up temp files on error
                      File.rm(temp_clip_path)
                      if temp_text_path, do: File.rm(temp_text_path)

                      conn
                      |> put_status(:internal_server_error)
                      |> put_resp_content_type("text/plain")
                      |> send_resp(500, "Error reading clipped #{format}: #{reason}")
                  end

                {error_output, _exit_code} ->
                  # Clean up temp files if they exist
                  File.rm(temp_clip_path)
                  if temp_text_path, do: File.rm(temp_text_path)

                  conn
                  |> put_status(:internal_server_error)
                  |> put_resp_content_type("text/plain")
                  |> send_resp(500, "Error creating #{format} clip with ffmpeg: #{error_output}")
              end
          end

        _ ->
          conn
          |> put_status(:bad_request)
          |> put_resp_content_type("text/plain")
          |> send_resp(
            400,
            "Invalid time parameters. Start time must be non-negative and end time must be greater than start time."
          )
      end
    end
  end

  def watch_directories(conn, _params) do
    watch_directories = Autotranscript.ConfigManager.get_config_value("watch_directories") || []

    conn
    |> put_resp_content_type("application/json")
    |> send_resp(200, Jason.encode!(watch_directories))
  end

  def watch_directory(conn, _params) do
    watch_directory = Autotranscript.ConfigManager.get_config_value("watch_directory")

    conn
    |> put_resp_content_type("text/plain")
    |> send_resp(200, watch_directory || "")
  end

  def regenerate_meta(conn, %{"filename" => filename}) do
    decoded_filename = decode_filename(filename)
    watch_directories = Autotranscript.ConfigManager.get_config_value("watch_directories") || []

    if Enum.empty?(watch_directories) do
      conn
      |> put_status(:service_unavailable)
      |> put_resp_content_type("application/json")
      |> send_resp(
        503,
        Jason.encode!(%{
          error: "Watch directories not configured. Please configure the application first."
        })
      )
    else
      # Check if the video file exists in any watch directory
      video_file = find_video_file_in_watch_directories(watch_directories, decoded_filename)

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
          |> send_resp(
            200,
            Jason.encode!(%{
              message: "Meta file regeneration for '#{decoded_filename}' added to queue"
            })
          )
      end
    end
  end

  def replace_transcript(conn, %{"filename" => filename}) do
    decoded_filename = decode_filename(filename)
    watch_directories = Autotranscript.ConfigManager.get_config_value("watch_directories") || []

    if Enum.empty?(watch_directories) do
      conn
      |> put_status(:service_unavailable)
      |> put_resp_content_type("text/plain")
      |> send_resp(
        503,
        "Watch directories not configured. Please configure the application first."
      )
    else
      # Search for the transcript file in all watch directories
      file_path = find_transcript_file(watch_directories, decoded_filename)

      case file_path do
        nil ->
          conn
          |> put_status(:not_found)
          |> put_resp_content_type("text/plain")
          |> send_resp(404, "Transcript file '#{decoded_filename}.txt' not found")

        path ->
          case conn.body_params do
            %{"text" => replacement_text} ->
              case File.write(path, replacement_text, [:utf8]) do
                :ok ->
                  VideoInfoCache.update_video_info_cache()

                  conn
                  |> put_resp_content_type("text/plain")
                  |> send_resp(
                    200,
                    "Transcript file '#{decoded_filename}.txt' updated successfully"
                  )

                {:error, reason} ->
                  conn
                  |> put_status(:internal_server_error)
                  |> put_resp_content_type("text/plain")
                  |> send_resp(
                    500,
                    "Error updating transcript file '#{decoded_filename}.txt': #{reason}"
                  )
              end

            _ ->
              conn
              |> put_status(:bad_request)
              |> put_resp_content_type("text/plain")
              |> send_resp(400, "Missing required 'text' field in request body")
          end
      end
    end
  end

  def partial_reprocess(conn, %{"filename" => filename}) do
    decoded_filename = decode_filename(filename)
    watch_directories = Autotranscript.ConfigManager.get_config_value("watch_directories") || []

    if Enum.empty?(watch_directories) do
      conn
      |> put_status(:service_unavailable)
      |> put_resp_content_type("application/json")
      |> send_resp(
        503,
        Jason.encode!(%{
          error: "Watch directories not configured. Please configure the application first."
        })
      )
    else
      case conn.body_params do
        %{"time" => time_str} ->
          case parse_time_to_seconds(time_str) do
            {:ok, time_seconds} ->
              execute_partial_reprocess(
                conn,
                decoded_filename,
                time_str,
                time_seconds,
                watch_directories
              )

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
          |> send_resp(
            400,
            Jason.encode!(%{error: "Missing required 'time' field in request body"})
          )
      end
    end
  end

  defp execute_partial_reprocess(
         conn,
         decoded_filename,
         time_str,
         _time_seconds,
         watch_directories
       ) do
    txt_file_path = find_transcript_file(watch_directories, decoded_filename)
    video_file = find_video_file_in_watch_directories(watch_directories, decoded_filename)

    case video_file do
      nil ->
        conn
        |> put_status(:not_found)
        |> put_resp_content_type("application/json")
        |> send_resp(404, Jason.encode!(%{error: "Video file '#{decoded_filename}' not found"}))

      video_path ->
        case txt_file_path do
          nil ->
            conn
            |> put_status(:not_found)
            |> put_resp_content_type("application/json")
            |> send_resp(
              404,
              Jason.encode!(%{error: "Transcript file '#{decoded_filename}.txt' not found"})
            )

          _ ->
            # Add the partial reprocessing job to the queue
            Autotranscript.VideoProcessor.add_to_queue(video_path, :partial, %{time: time_str})

            conn
            |> put_resp_content_type("application/json")
            |> send_resp(
              200,
              Jason.encode!(%{
                message:
                  "Partial reprocessing for '#{decoded_filename}' from time #{time_str} has been added to the queue"
              })
            )
        end
    end
  end

  defp calculate_font_size_for_video(video_path, text_length) do
    # Get video dimensions using ffprobe
    ffprobe_path = ConfigManager.get_config_value("ffprobe_path") || "ffprobe"

    case System.cmd(ffprobe_path, [
           "-v",
           "error",
           "-select_streams",
           "v:0",
           "-show_entries",
           "stream=width,height",
           "-of",
           "csv=p=0",
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
                length_factor =
                  cond do
                    # Very long text - smaller
                    text_length > 100 -> 0.6
                    # Long text - slightly smaller
                    text_length > 50 -> 0.8
                    # Medium text - base size
                    text_length > 25 -> 1.0
                    # Short text - larger
                    true -> 1.4
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
      # Very long text
      text_length > 100 -> 48
      # Long text
      text_length > 50 -> 72
      # Medium text
      text_length > 25 -> 96
      # Short text
      true -> 144
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

  def set_line(conn, %{"filename" => filename}) do
    decoded_filename = decode_filename(filename)
    watch_directories = Autotranscript.ConfigManager.get_config_value("watch_directories") || []

    if Enum.empty?(watch_directories) do
      conn
      |> put_status(:service_unavailable)
      |> put_resp_content_type("application/json")
      |> send_resp(
        503,
        Jason.encode!(%{
          error: "Watch directories not configured. Please configure the application first."
        })
      )
    else
      case conn.body_params do
        %{"line_number" => line_number_str, "text" => new_text} ->
          case Integer.parse(line_number_str) do
            {line_number, ""} when line_number > 0 ->
              file_path = find_transcript_file(watch_directories, decoded_filename)

              case file_path do
                nil ->
                  conn
                  |> put_status(:not_found)
                  |> put_resp_content_type("application/json")
                  |> send_resp(
                    404,
                    Jason.encode!(%{error: "Transcript file '#{decoded_filename}.txt' not found"})
                  )

                path ->
                  case File.read(path) do
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
                            |> send_resp(
                              200,
                              Jason.encode!(%{
                                message:
                                  "Line #{line_number} in transcript '#{decoded_filename}.txt' updated successfully"
                              })
                            )

                          {:error, reason} ->
                            conn
                            |> put_status(:internal_server_error)
                            |> put_resp_content_type("application/json")
                            |> send_resp(
                              500,
                              Jason.encode!(%{
                                error:
                                  "Error updating transcript file '#{decoded_filename}.txt': #{reason}"
                              })
                            )
                        end
                      else
                        conn
                        |> put_status(:bad_request)
                        |> put_resp_content_type("application/json")
                        |> send_resp(
                          400,
                          Jason.encode!(%{
                            error:
                              "Line number #{line_number} is out of bounds. File has #{length(lines)} lines."
                          })
                        )
                      end

                    {:error, :enoent} ->
                      conn
                      |> put_status(:not_found)
                      |> put_resp_content_type("application/json")
                      |> send_resp(
                        404,
                        Jason.encode!(%{
                          error: "Transcript file '#{decoded_filename}.txt' not found"
                        })
                      )

                    {:error, reason} ->
                      conn
                      |> put_status(:internal_server_error)
                      |> put_resp_content_type("application/json")
                      |> send_resp(
                        500,
                        Jason.encode!(%{
                          error:
                            "Error reading transcript file '#{decoded_filename}.txt': #{reason}"
                        })
                      )
                  end
              end

            _ ->
              conn
              |> put_status(:bad_request)
              |> put_resp_content_type("application/json")
              |> send_resp(
                400,
                Jason.encode!(%{error: "Invalid line number. Must be a positive integer."})
              )
          end

        _ ->
          conn
          |> put_status(:bad_request)
          |> put_resp_content_type("application/json")
          |> send_resp(
            400,
            Jason.encode!(%{
              error: "Missing required 'line_number' and 'text' fields in request body"
            })
          )
      end
    end
  end

  def get_meta_file(conn, %{"filename" => filename}) do
    decoded_filename = decode_filename(filename)
    watch_directories = Autotranscript.ConfigManager.get_config_value("watch_directories") || []

    if Enum.empty?(watch_directories) do
      conn
      |> put_status(:service_unavailable)
      |> put_resp_content_type("application/json")
      |> send_resp(
        503,
        Jason.encode!(%{
          error: "Watch directories not configured. Please configure the application first."
        })
      )
    else
      # Find the video file first in any watch directory
      video_file = find_video_file_in_watch_directories(watch_directories, decoded_filename)

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
    watch_directories = Autotranscript.ConfigManager.get_config_value("watch_directories") || []

    if Enum.empty?(watch_directories) do
      conn
      |> put_status(:service_unavailable)
      |> put_resp_content_type("application/json")
      |> send_resp(
        503,
        Jason.encode!(%{
          error: "Watch directories not configured. Please configure the application first."
        })
      )
    else
      case conn.body_params do
        %{"content" => content} ->
          # Find the video file first in any watch directory
          video_file = find_video_file_in_watch_directories(watch_directories, decoded_filename)

          case video_file do
            nil ->
              conn
              |> put_status(:not_found)
              |> put_resp_content_type("application/json")
              |> send_resp(
                404,
                Jason.encode!(%{error: "Video file '#{decoded_filename}' not found"})
              )

            file_path ->
              # Get the meta file path
              meta_path = PathHelper.replace_video_extension_with(file_path, ".meta")

              # Write the content to the meta file
              case File.write(meta_path, content, [:utf8]) do
                :ok ->
                  VideoInfoCache.update_video_info_cache()

                  conn
                  |> put_resp_content_type("application/json")
                  |> send_resp(
                    200,
                    Jason.encode!(%{
                      message: "Meta file '#{decoded_filename}.meta' updated successfully"
                    })
                  )

                {:error, reason} ->
                  conn
                  |> put_status(:internal_server_error)
                  |> put_resp_content_type("application/json")
                  |> send_resp(
                    500,
                    Jason.encode!(%{
                      error: "Error updating meta file '#{decoded_filename}.meta': #{reason}"
                    })
                  )
              end
          end

        _ ->
          conn
          |> put_status(:bad_request)
          |> put_resp_content_type("application/json")
          |> send_resp(
            400,
            Jason.encode!(%{error: "Missing required 'content' field in request body"})
          )
      end
    end
  end

  def rename(conn, %{"filename" => filename}) do
    decoded_filename = decode_filename(filename)
    watch_directories = Autotranscript.ConfigManager.get_config_value("watch_directories") || []

    if Enum.empty?(watch_directories) do
      conn
      |> put_status(:service_unavailable)
      |> put_resp_content_type("application/json")
      |> send_resp(
        503,
        Jason.encode!(%{
          error: "Watch directories not configured. Please configure the application first."
        })
      )
    else
      case conn.body_params do
        %{"new_filename" => new_filename} ->
          # Validate new filename (no path separators)
          if String.contains?(new_filename, ["/", "\\"]) do
            conn
            |> put_status(:bad_request)
            |> put_resp_content_type("application/json")
            |> send_resp(
              400,
              Jason.encode!(%{error: "New filename cannot contain path separators"})
            )
          else
            # Find the original video file in any watch directory
            video_file = find_video_file_in_watch_directories(watch_directories, decoded_filename)

            case video_file do
              nil ->
                conn
                |> put_status(:not_found)
                |> put_resp_content_type("application/json")
                |> send_resp(
                  404,
                  Jason.encode!(%{error: "Video file '#{decoded_filename}' not found"})
                )

              old_video_path ->
                # Get the video file extension and directory
                video_extension = Path.extname(old_video_path)
                directory = Path.dirname(old_video_path)

                # Build new file paths
                new_video_path = Path.join(directory, "#{new_filename}#{video_extension}")
                old_txt_path = PathHelper.replace_video_extension_with(old_video_path, ".txt")
                new_txt_path = Path.join(directory, "#{new_filename}.txt")
                old_meta_path = PathHelper.replace_video_extension_with(old_video_path, ".meta")
                new_meta_path = Path.join(directory, "#{new_filename}.meta")

                # Check if any target files already exist
                cond do
                  File.exists?(new_video_path) ->
                    conn
                    |> put_status(:conflict)
                    |> put_resp_content_type("application/json")
                    |> send_resp(
                      409,
                      Jason.encode!(%{
                        error: "A video file with name '#{new_filename}' already exists"
                      })
                    )

                  File.exists?(new_txt_path) ->
                    conn
                    |> put_status(:conflict)
                    |> put_resp_content_type("application/json")
                    |> send_resp(
                      409,
                      Jason.encode!(%{
                        error: "A transcript file with name '#{new_filename}' already exists"
                      })
                    )

                  File.exists?(new_meta_path) ->
                    conn
                    |> put_status(:conflict)
                    |> put_resp_content_type("application/json")
                    |> send_resp(
                      409,
                      Jason.encode!(%{
                        error: "A meta file with name '#{new_filename}' already exists"
                      })
                    )

                  true ->
                    # Perform the rename operations
                    rename_results = [
                      {:video, File.rename(old_video_path, new_video_path)},
                      {:txt,
                       if(File.exists?(old_txt_path),
                         do: File.rename(old_txt_path, new_txt_path),
                         else: :ok
                       )},
                      {:meta,
                       if(File.exists?(old_meta_path),
                         do: File.rename(old_meta_path, new_meta_path),
                         else: :ok
                       )}
                    ]

                    # Check if all renames were successful
                    failed_renames =
                      Enum.filter(rename_results, fn {_type, result} -> result != :ok end)

                    case failed_renames do
                      [] ->
                        # All renames successful, update cache
                        VideoInfoCache.update_video_info_cache()

                        conn
                        |> put_resp_content_type("application/json")
                        |> send_resp(
                          200,
                          Jason.encode!(%{
                            message:
                              "Successfully renamed '#{decoded_filename}' to '#{new_filename}'"
                          })
                        )

                      failures ->
                        # Some renames failed, attempt to rollback successful ones
                        Logger.error("Rename operation failed: #{inspect(failures)}")

                        # Attempt rollback of successful renames (best effort)
                        if File.exists?(new_video_path),
                          do: File.rename(new_video_path, old_video_path)

                        if File.exists?(new_txt_path), do: File.rename(new_txt_path, old_txt_path)

                        if File.exists?(new_meta_path),
                          do: File.rename(new_meta_path, old_meta_path)

                        failed_types = Enum.map(failures, fn {type, _} -> type end)

                        conn
                        |> put_status(:internal_server_error)
                        |> put_resp_content_type("application/json")
                        |> send_resp(
                          500,
                          Jason.encode!(%{
                            error:
                              "Failed to rename files. Failed operations: #{Enum.join(failed_types, ", ")}"
                          })
                        )
                    end
                end
            end
          end

        _ ->
          conn
          |> put_status(:bad_request)
          |> put_resp_content_type("application/json")
          |> send_resp(
            400,
            Jason.encode!(%{error: "Missing required 'new_filename' field in request body"})
          )
      end
    end
  end
end
