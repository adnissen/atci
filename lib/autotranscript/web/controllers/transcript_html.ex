defmodule Autotranscript.Web.TranscriptHTML do
  use Autotranscript.Web, :html

  embed_templates "transcript_html/*"

  def format_datetime(datetime) when is_tuple(datetime) do
    case datetime do
      {{year, month, day}, {hour, minute, second}} ->
        :io_lib.format("~4..0B-~2..0B-~2..0B ~2..0B:~2..0B:~2..0B",
                      [year, month, day, hour, minute, second])
        |> to_string()
      _ ->
        "Unknown"
    end
  end

  def format_datetime(_), do: "Unknown"
end
