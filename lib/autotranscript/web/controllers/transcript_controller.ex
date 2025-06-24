defmodule Autotranscript.Web.TranscriptController do
  use Autotranscript.Web, :controller

  def index(conn, _params) do
    watch_directory = Application.get_env(:autotranscript, :watch_directory)

    # Get all .txt files in the watch directory
    txt_files =
      get_txt_files()
      |> Jason.encode!()

    render(conn, :index, txt_files: txt_files)
  end

  def show(conn, %{"filename" => filename}) do
    watch_directory = Application.get_env(:autotranscript, :watch_directory)
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

  def grep(conn, %{"text" => search_text}) do
    watch_directory = Application.get_env(:autotranscript, :watch_directory)

    # Change to the watch directory and run grep
    case System.shell("grep -Hli \"" <> search_text <> "\" *.txt", cd: watch_directory) do
      {output, 0} ->
        conn
        |> put_resp_content_type("text/plain")
        |> send_resp(200, output)
      {output, exit_code} when exit_code > 1 ->
        conn
        |> put_status(:internal_server_error)
        |> put_resp_content_type("text/plain")
        |> send_resp(500, "Error running grep: #{output}")
      {_output, 1} ->
        # grep returns 1 when no matches found, which is not an error
        conn
        |> put_resp_content_type("text/plain")
        |> send_resp(200, "")
    end
  end

  def regenerate(conn, %{"filename" => filename}) do
    watch_directory = Application.get_env(:autotranscript, :watch_directory)
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
    txt_files = get_txt_files()

    conn
    |> put_resp_content_type("application/json")
    |> send_resp(200, Jason.encode!(txt_files))
  end

  defp get_txt_files do
    watch_directory = Application.get_env(:autotranscript, :watch_directory)

    # Get all .txt files in the watch directory
    Path.wildcard(Path.join(watch_directory, "*.txt"))
    |> Enum.map(fn file_path ->
      case File.stat(file_path) do
        {:ok, stat} ->
          filename = Path.basename(file_path, ".txt")

          # Count lines in the file
          line_count =
            case File.read(file_path) do
              {:ok, content} -> length(String.split(content, "\n"))
              {:error, _} -> 0
            end

          # Look for corresponding .mp4 or .MP4 file and use its created_at time
          mp4_path = Path.join(watch_directory, "#{filename}.mp4")
          mp4_upper_path = Path.join(watch_directory, "#{filename}.MP4")

          created_at =
            cond do
              File.exists?(mp4_path) ->
                case File.stat(mp4_path) do
                  {:ok, mp4_stat} -> mp4_stat.ctime |> Autotranscript.Web.TranscriptHTML.format_datetime()
                  {:error, _} -> stat.ctime |> Autotranscript.Web.TranscriptHTML.format_datetime()
                end
              File.exists?(mp4_upper_path) ->
                case File.stat(mp4_upper_path) do
                  {:ok, mp4_stat} -> mp4_stat.ctime |> Autotranscript.Web.TranscriptHTML.format_datetime()
                  {:error, _} -> stat.ctime |> Autotranscript.Web.TranscriptHTML.format_datetime()
                end
              true ->
                stat.ctime |> Autotranscript.Web.TranscriptHTML.format_datetime()
            end

          %{
            name: filename,
            created_at: created_at,
            line_count: line_count,
            full_path: String.replace_trailing(file_path, ".txt", ".mp4")
          }
        {:error, _} ->
          nil
      end
    end)
    |> Enum.reject(&is_nil/1)
    |> Enum.sort_by(& &1.created_at, :desc)
  end

  def random_frame(conn, _params) do
    watch_directory = Application.get_env(:autotranscript, :watch_directory)

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

  def player(conn, %{"filename" => filename} = params) do
    watch_directory = Application.get_env(:autotranscript, :watch_directory)

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
end
