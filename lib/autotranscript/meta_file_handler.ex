defmodule Autotranscript.MetaFileHandler do
  @moduledoc """
  Handles reading and writing meta files that store metadata about video files.
  Meta files contain key-value pairs, one per line, in the format "key: value".
  """

  require Logger

  @doc """
  Reads a meta file and returns a map of its contents.

  ## Parameters
    - meta_path: String path to the meta file

  ## Returns
    - {:ok, map} where map contains the parsed key-value pairs
    - {:error, reason} if the file cannot be read
  """
  def read_meta_file(meta_path) do
    case File.read(meta_path) do
      {:ok, content} ->
        metadata = content
          |> String.split("\n", trim: true)
          |> Enum.reduce(%{}, fn line, acc ->
            case String.split(line, ": ", parts: 2) do
              [key, value] ->
                Map.put(acc, String.trim(key), String.trim(value))
              _ ->
                # Skip invalid lines
                acc
            end
          end)

        {:ok, metadata}
      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc """
  Writes a map of metadata to a meta file.

  ## Parameters
    - meta_path: String path to the meta file
    - metadata: Map of key-value pairs to write

  ## Returns
    - :ok if successful
    - {:error, reason} if the file cannot be written
  """
  def write_meta_file(meta_path, metadata) when is_map(metadata) do
    content = metadata
      |> Enum.map(fn {key, value} -> "#{key}: #{value}" end)
      |> Enum.sort()
      |> Enum.join("\n")

    case File.write(meta_path, content <> "\n", [:utf8]) do
      :ok ->
        Logger.info("Wrote meta file: #{meta_path}")
        :ok
      {:error, reason} ->
        Logger.error("Failed to write meta file #{meta_path}: #{inspect(reason)}")
        {:error, reason}
    end
  end

  @doc """
  Updates a specific field in a meta file.

  ## Parameters
    - meta_path: String path to the meta file
    - key: String key to update
    - value: String value to set

  ## Returns
    - :ok if successful
    - {:error, reason} if the operation fails
  """
  def update_meta_field(meta_path, key, value) do
    # Read existing metadata or start with empty map
    metadata = case read_meta_file(meta_path) do
      {:ok, existing} -> existing
      {:error, _} -> %{}
    end

    # Update the field
    updated_metadata = Map.put(metadata, key, value)

    # Write back
    write_meta_file(meta_path, updated_metadata)
  end

  @doc """
  Gets a specific field from a meta file.

  ## Parameters
    - meta_path: String path to the meta file
    - key: String key to retrieve

  ## Returns
    - {:ok, value} if the field exists
    - {:error, :not_found} if the field doesn't exist
    - {:error, reason} if the file cannot be read
  """
  def get_meta_field(meta_path, key) do
    case read_meta_file(meta_path) do
      {:ok, metadata} ->
        case Map.get(metadata, key) do
          nil -> {:error, :not_found}
          value -> {:ok, value}
        end
      {:error, reason} ->
        {:error, reason}
    end
  end
end
