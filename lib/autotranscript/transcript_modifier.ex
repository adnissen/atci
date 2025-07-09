defmodule Autotranscript.TranscriptModifier do
  @moduledoc """
  Utility module for modifying transcript files and updating source metadata.
  """

  require Logger

  @doc """
  Modifies a transcript file by removing the first line if it contains model information,
  and saves the source information to the corresponding meta file.

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
         {:ok, modified_content} <- remove_model_line_if_exists(content),
         :ok <- File.write(transcript_path, modified_content, [:utf8]),
         :ok <- update_meta_file_source(transcript_path, model_filename) do
      Logger.info("Successfully modified transcript file and updated meta: #{transcript_path}")
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
  Removes the first line if it contains model information.

  ## Parameters
    - content: String content of the transcript file

  ## Returns
    - {:ok, modified_content} if successful
  """
  def remove_model_line_if_exists(content) do
    lines = String.split(content, "\n")

    case lines do
      [] ->
        {:ok, ""}
      [first_line | rest] ->
        # Check if first line contains model information
        if String.starts_with?(first_line, "model: ") do
          # Remove the model line
          {:ok, Enum.join(rest, "\n")}
        else
          # Keep content as is
          {:ok, content}
        end
    end
  end

  defp update_meta_file_source(transcript_path, model_filename) do
    meta_path = String.replace_trailing(transcript_path, ".txt", ".meta")
    
    case Autotranscript.MetaFileHandler.update_meta_field(meta_path, "source", model_filename) do
      :ok -> :ok
      {:error, reason} -> 
        Logger.warning("Failed to update meta file with source: #{inspect(reason)}")
        :ok  # Still return ok since transcript was modified successfully
    end
  end
end
