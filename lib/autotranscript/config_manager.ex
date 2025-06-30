defmodule Autotranscript.ConfigManager do
  @moduledoc """
  Manages configuration for Autotranscript application using a GenServer.

  Configuration is stored in memory after being loaded from a `.atconfig` file in JSON format.
  The file is looked for in the current directory first, then in the home directory.
  """

  use GenServer
  require Logger

  @config_filename ".atconfig"

  # Client API

  @doc """
  Starts the ConfigManager GenServer.
  """
  def start_link(_opts) do
    GenServer.start_link(__MODULE__, %{}, name: :config_manager)
  end

  @doc """
  Gets the current configuration.

  Returns a map with the configuration or an empty map if no config is found.
  """
  def get_config do
    GenServer.call(:config_manager, :get_config)
  end

  @doc """
  Saves configuration to a .atconfig file and updates the in-memory state.

  Saves to the current directory by default.
  """
  def save_config(config) when is_map(config) do
    GenServer.call(:config_manager, {:save_config, config})
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
    GenServer.call(:config_manager, {:get_config_value, key})
  end

  @doc """
  Reloads configuration from file.
  """
  def reload_config do
    GenServer.call(:config_manager, :reload_config)
  end

  # Server callbacks

  @impl true
  def init(_args) do
    config = load_config_from_file()
    Logger.info("ConfigManager started with config keys: #{inspect(Map.keys(config))}")
    {:ok, config}
  end

  @impl true
  def handle_call(:get_config, _from, state) do
    {:reply, state, state}
  end

  @impl true
  def handle_call({:get_config_value, key}, _from, state) do
    value = Map.get(state, key)
    {:reply, value, state}
  end

  @impl true
  def handle_call({:save_config, config}, _from, _state) do
    case save_config_to_file(config) do
      {:ok, config_path} ->
        Logger.info("Configuration saved to: #{config_path}")
        {:reply, {:ok, config_path}, config}
      {:error, reason} = error ->
        {:reply, error, config}
    end
  end

  @impl true
  def handle_call(:reload_config, _from, _state) do
    new_config = load_config_from_file()
    Logger.info("Configuration reloaded")
    {:reply, :ok, new_config}
  end

  # Private functions

  defp load_config_from_file do
    case find_config_file() do
      {:ok, config_path} ->
        case File.read(config_path) do
          {:ok, content} ->
            case Jason.decode(content) do
              {:ok, config} ->
                Logger.info("Configuration loaded from: #{config_path}")
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
        Logger.info("No configuration file found, using empty config")
        %{}
    end
  end

  defp save_config_to_file(config) when is_map(config) do
    config_path = Path.join(File.cwd!(), @config_filename)

    case Jason.encode(config, pretty: true) do
      {:ok, json_content} ->
        case File.write(config_path, json_content) do
          :ok ->
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
