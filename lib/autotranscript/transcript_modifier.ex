defmodule Autotranscript.TranscriptModifier do
  @moduledoc """
  Utility module for modifying transcript files.
  """

  require Logger

  @doc """
  Modifies a transcript file by removing the first line and adding a new line
  at the start with "model: " followed by the model filename (without extension).

  ## Parameters
    - transcript_path: String path to the transcript file

  ## Returns
    - :ok if successful
    - {:error, reason} if there was an error

  ## Examples
      iex> Autotranscript.TranscriptModifier.modify_transcript_file("/path/to/transcript.txt")
      :ok
  """
  def modify_transcript_file(transcript_path) do
    with {:ok, model_filename} <- get_model_filename(),
         {:ok, content} <- File.read(transcript_path),
         {:ok, modified_content} <- modify_content(content, model_filename),
         :ok <- File.write(transcript_path, modified_content, [:utf8]) do
      Logger.info("Successfully modified transcript file: #{transcript_path}")
      :ok
    else
      {:error, reason} ->
        Logger.error("Failed to modify transcript file #{transcript_path}: #{inspect(reason)}")
        {:error, reason}
    end
  end

  @doc """
  Gets the model filename from the config manager without the extension.

  ## Returns
    - {:ok, filename} if successful
    - {:error, reason} if there was an error
  """
  def get_model_filename do
    case Autotranscript.ConfigManager.get_config_value("model_path") do
      nil ->
        {:error, "model_path not configured"}
      "" ->
        {:error, "model_path is empty"}
      model_path ->
        filename = model_path
                  |> Path.basename()
                  |> Path.rootname()
        {:ok, filename}
    end
  end

  @doc """
  Modifies the content by removing the first line and adding the model line.

  ## Parameters
    - content: String content of the transcript file
    - model_filename: String filename of the model (without extension)

  ## Returns
    - {:ok, modified_content} if successful
    - {:error, reason} if there was an error
  """
  def modify_content(content, model_filename) do
    lines = String.split(content, "\n")

    case lines do
      [] ->
        # Empty file, just add the model line
        {:ok, "model: #{model_filename}\n"}
      [_first_line | rest] ->
        # Remove first line and add model line at the beginning
        new_lines = ["model: #{model_filename}" | rest]
        {:ok, Enum.join(new_lines, "\n")}
    end
  end
end
