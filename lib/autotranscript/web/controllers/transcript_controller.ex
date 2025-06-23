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
              created_at: stat.ctime,
              line_count: line_count,
              full_path: file_path
            }
          {:error, _} ->
            nil
        end
      end)
      |> Enum.reject(&is_nil/1)
      |> Enum.sort_by(& &1.created_at, :desc)

    render(conn, :index, txt_files: txt_files)
  end
end
