defmodule Autotranscript.TranscriptModifier do
  @moduledoc """
  Utility module for modifying transcript files and updating source metadata.
  """

  require Logger

  @doc """
  Adds source information to the meta file for a transcript.

  ## Parameters
    - transcript_path: String path to the transcript file

  ## Returns
    - :ok if successful
    - {:error, reason} if there was an error

  ## Examples
      iex> Autotranscript.TranscriptModifier.add_source_to_meta("/path/to/transcript.txt")
      :ok
  """
  def add_source_to_meta(transcript_path) do
    with {:ok, model_filename} <- get_model_filename(),
         :ok <- update_meta_file_source(transcript_path, model_filename) do
      Logger.info("Successfully updated meta file for: #{transcript_path}")
      :ok
    else
      {:error, reason} ->
        Logger.error("Failed to update meta file for #{transcript_path}: #{inspect(reason)}")
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
