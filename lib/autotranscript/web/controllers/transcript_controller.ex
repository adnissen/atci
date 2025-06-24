defmodule Autotranscript.Web.TranscriptController do
  use Autotranscript.Web, :controller

  def index(conn, _params) do
    watch_directory = Application.get_env(:autotranscript, :watch_directory)

    # Get all .txt files in the watch directory
    txt_files =
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

            %{
              name: filename,
              created_at: stat.ctime |> Autotranscript.Web.TranscriptHTML.format_datetime(),
              line_count: line_count,
              full_path: file_path
            }
          {:error, _} ->
            nil
        end
      end)
      |> Enum.reject(&is_nil/1)
      |> Enum.sort_by(& &1.created_at, :desc)
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
    watch_directory = Application.get_env(:autotranscript, :watch_directory)

    # Get all .txt files in the watch directory
    txt_files =
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

            %{
              name: filename,
              created_at: stat.ctime |> Autotranscript.Web.TranscriptHTML.format_datetime(),
              line_count: line_count,
              full_path: file_path
            }
          {:error, _} ->
            nil
        end
      end)
      |> Enum.reject(&is_nil/1)
      |> Enum.sort_by(& &1.created_at, :desc)

    conn
    |> put_resp_content_type("application/json")
    |> send_resp(200, Jason.encode!(txt_files))
  end
end
