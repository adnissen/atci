defmodule Autotranscript.Config do
  @moduledoc """
  Configuration management for Autotranscript.
  
  Reads configuration from `.atconfig` files, checking first in the current
  directory and then in the home directory. Falls back to application config
  if no config file is found.
  """

  require Logger

  @config_file ".atconfig"
  @required_keys [:watch_directory, :whispercli_path, :model_path]

  @doc """
  Gets a configuration value by key.
  
  Checks in this order:
  1. .atconfig file in current directory
  2. .atconfig file in home directory  
  
  Returns nil if no configuration is found.
  """
  def get(key, default \\ nil) do
    case get_from_config_file(key) do
      nil -> default
      value -> value
    end
  end

  @doc """
  Gets all configuration values as a map.
  """
  def get_all() do
    %{
      watch_directory: get(:watch_directory),
      whispercli_path: get(:whispercli_path),
      model_path: get(:model_path)
    }
  end

  @doc """
  Checks if all required configuration values are present and valid.
  """
  def valid?() do
    config = get_all()
    
    Enum.all?(@required_keys, fn key ->
      value = Map.get(config, key)
      value != nil and value != "" and String.trim(value) != ""
    end)
  end

  @doc """
  Saves configuration to a .atconfig file in the current directory.
  """
  def save(config) do
    config_content = Enum.map(config, fn {key, value} ->
      "#{key}=#{value}"
    end)
    |> Enum.join("\n")
    
    case File.write(@config_file, config_content) do
      :ok -> 
        Logger.info("Configuration saved to #{@config_file}")
        :ok
      {:error, reason} -> 
        Logger.error("Failed to save configuration: #{reason}")
        {:error, reason}
    end
  end

  @doc """
  Gets the path to the config file being used, or nil if none exists.
  """
  def config_file_path() do
    current_dir_config = Path.join(File.cwd!(), @config_file)
    home_dir_config = Path.join(System.user_home!(), @config_file)
    
    cond do
      File.exists?(current_dir_config) -> current_dir_config
      File.exists?(home_dir_config) -> home_dir_config
      true -> nil
    end
  end

  # Private functions

  defp get_from_config_file(key) do
    case config_file_path() do
      nil -> nil
      path -> 
        case read_config_file(path) do
          {:ok, config} -> Map.get(config, key)
          {:error, _} -> nil
        end
    end
  end

  defp read_config_file(path) do
    case File.read(path) do
      {:ok, content} ->
        config = content
        |> String.split("\n")
        |> Enum.map(&String.trim/1)
        |> Enum.reject(&(&1 == "" or String.starts_with?(&1, "#")))
        |> Enum.reduce(%{}, fn line, acc ->
          case String.split(line, "=", parts: 2) do
            [key, value] ->
              atom_key = String.to_atom(String.trim(key))
              Map.put(acc, atom_key, String.trim(value))
            _ -> acc
          end
        end)
        
        {:ok, config}
      {:error, reason} -> 
        Logger.error("Failed to read config file #{path}: #{reason}")
        {:error, reason}
    end
  end
end