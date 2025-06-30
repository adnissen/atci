defmodule Autotranscript.ConfigManager do
  @moduledoc """
  Manages configuration for Autotranscript application.

  Configuration is stored in a `.atconfig` file in JSON format.
  The file is looked for in the current directory first, then in the home directory.
  """

  require Logger

  @config_filename ".atconfig"

  @doc """
  Gets the current configuration.

  Returns a map with the configuration or an empty map if no config is found.
  """
  def get_config do
    case Cachex.get(:config_cache, "config") do
      {:ok, config} -> config
      {:error, :no_cache} -> do_get_config()
    end
  end

  defp do_get_config do
    case find_config_file() do
      {:ok, config_path} ->
        case File.read(config_path) do
          {:ok, content} ->
            case Jason.decode(content) do
              {:ok, config} ->
                Logger.info("Loaded configuration from: #{config_path}")
                Cachex.put(:config_cache, "config", config)
                config
              {:error, reason} ->
                Logger.error("Failed to parse config file #{config_path}: #{inspect(reason)}")
                %{}
            end
          {:error, reason} ->
            Logger.error("Failed to read config file #{config_path}: #{inspect(reason)}")
            %{}
        end
      {:error, :not_found} ->
        Logger.info("No configuration file found")
        %{}
    end
  end

  @doc """
  Saves configuration to a .atconfig file.

  Saves to the current directory by default.
  """
  def save_config(config) when is_map(config) do
    config_path = Path.join(File.cwd!(), @config_filename)

    case Jason.encode(config, pretty: true) do
      {:ok, json_content} ->
        case File.write(config_path, json_content) do
          :ok ->
            Logger.info("Configuration saved to: #{config_path}")
            {:ok, config_path}
          {:error, reason} ->
            Logger.error("Failed to write config file #{config_path}: #{inspect(reason)}")
            {:error, reason}
        end
      {:error, reason} ->
        Logger.error("Failed to encode config as JSON: #{inspect(reason)}")
        {:error, reason}
    end
  end

  @doc """
  Checks if the configuration is complete.

  Returns true if all required fields are present and valid.
  """
  def config_complete?(config) when is_map(config) do
    required_keys = ["watch_directory", "whispercli_path", "model_path"]

    Enum.all?(required_keys, fn key ->
      case Map.get(config, key) do
        nil -> false
        "" -> false
        path when is_binary(path) ->
          # Check if the path exists and is valid
          case key do
            "watch_directory" -> File.dir?(path)
            _ -> File.exists?(path)
          end
        _ -> false
      end
    end)
  end

  @doc """
  Gets a specific configuration value.

  Returns the value or nil if not found.
  """
  def get_config_value(key) when is_binary(key) do
    get_config()
    |> Map.get(key)
  end

  defp find_config_file do
    current_dir_config = Path.join(File.cwd!(), @config_filename)
    home_dir_config = Path.expand(Path.join("~", @config_filename))

    cond do
      File.exists?(current_dir_config) ->
        {:ok, current_dir_config}
      File.exists?(home_dir_config) ->
        {:ok, home_dir_config}
      true ->
        {:error, :not_found}
    end
  end
end
