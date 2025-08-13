defmodule Atci.ConfigManager do
  @moduledoc """
  Manages configuration for Atci application using a GenServer.

  Configuration is stored in memory after being loaded from a `.atciconfig` file in JSON format.
  The file is looked for only in the home directory.
  """

  use GenServer
  require Logger

  @config_filename ".atciconfig"

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
  Saves configuration to a .atciconfig file and updates the in-memory state.

  Saves to the home directory.
  """
  def save_config(config) when is_map(config) do
    GenServer.call(:config_manager, {:save_config, config})
  end

  @doc """
  Checks if the configuration is complete.

  Returns true if all required fields are present and valid.
  """
  def config_complete?(config) when is_map(config) do
    required_keys = ["watch_directories", "whispercli_path", "ffmpeg_path", "ffprobe_path"]

    base_complete =
      Enum.all?(required_keys, fn key ->
        case Map.get(config, key) do
          nil ->
            false

          "" ->
            false

          path when is_binary(path) ->
            # Check if the path exists and is valid
            case key do
              _ -> File.exists?(path)
            end

          directories when is_list(directories) and key == "watch_directories" ->
            # Validate watch directories
            validate_watch_directories(directories)

          _ ->
            false
        end
      end)

    # Check if we have either model_path or model_name
    model_complete =
      case {Map.get(config, "model_path"), Map.get(config, "model_name")} do
        {nil, nil} ->
          false

        {"", ""} ->
          false

        {"", nil} ->
          false

        {nil, ""} ->
          false

        {path, _} when is_binary(path) and path != "" ->
          File.exists?(path)

        {_, name} when is_binary(name) and name != "" ->
          # Check if the model is downloaded
          model_path = Path.join([Path.expand("~/.atci/models"), "#{name}.bin"])
          File.exists?(model_path)

        _ ->
          false
      end

    base_complete and model_complete
  end

  @doc """
  Gets a specific configuration value.

  Returns the value or nil if not found.
  For backward compatibility, "watch_directory" returns the first watch directory.
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

  @doc """
  Gets the effective model path, resolving model_name to a path if needed.
  """
  def get_effective_model_path do
    config = get_config()

    case {Map.get(config, "model_path"), Map.get(config, "model_name")} do
      {path, _} when is_binary(path) and path != "" ->
        # Use the explicit path if provided
        path

      {_, name} when is_binary(name) and name != "" ->
        # Convert model name to path
        Path.join([Path.expand("~/.atci/models"), "#{name}.bin"])

      _ ->
        nil
    end
  end

  # Server callbacks

  @impl true
  def init(_args) do
    config = load_config_from_file()

    # Auto-detect ffmpeg and ffprobe if not configured
    config = auto_detect_executables(config)

    # Save config if executables were auto-detected
    if Map.get(config, "ffmpeg_path") != Map.get(load_config_from_file(), "ffmpeg_path") or
         Map.get(config, "ffprobe_path") != Map.get(load_config_from_file(), "ffprobe_path") do
      save_config_to_file(config)
    end

    Logger.info("ConfigManager started with config keys: #{inspect(Map.keys(config))}")
    {:ok, config}
  end

  @impl true
  def handle_call(:get_config, _from, state) do
    {:reply, state, state}
  end

  @impl true
  def handle_call({:get_config_value, key}, _from, state) do
    value =
      case key do
        "watch_directory" ->
          # For backward compatibility, return the first watch directory
          case Map.get(state, "watch_directories") do
            [first_dir | _] -> first_dir
            _ -> nil
          end

        "model_path" ->
          # Return the effective model path
          case {Map.get(state, "model_path"), Map.get(state, "model_name")} do
            {path, _} when is_binary(path) and path != "" ->
              path

            {_, name} when is_binary(name) and name != "" ->
              Path.join([Path.expand("~/.atci/models"), "#{name}.bin"])

            _ ->
              nil
          end

        _ ->
          Map.get(state, key)
      end

    {:reply, value, state}
  end

  @impl true
  def handle_call({:save_config, config}, _from, _state) do
    case save_config_to_file(config) do
      {:ok, config_path} ->
        Logger.info("Configuration saved to: #{config_path}")
        {:reply, {:ok, config_path}, config}

      {:error, _reason} = error ->
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

  defp validate_watch_directories(directories) when is_list(directories) do
    # Check if the list is not empty
    if Enum.empty?(directories) do
      false
    else
      # Check if all directories exist and are valid
      valid_directories =
        Enum.all?(directories, fn dir ->
          is_binary(dir) and File.dir?(dir)
        end)

      # Check that no directory is a subdirectory of another
      no_subdirectories = not has_subdirectories?(directories)

      valid_directories and no_subdirectories
    end
  end

  defp has_subdirectories?(directories) when is_list(directories) do
    normalized_dirs = Enum.map(directories, &Path.expand/1)

    # Check each directory against every other directory
    Enum.any?(normalized_dirs, fn dir1 ->
      Enum.any?(normalized_dirs, fn dir2 ->
        dir1 != dir2 and String.starts_with?(dir1, dir2 <> "/")
      end)
    end)
  end

  defp migrate_config_format(config) when is_map(config) do
    # Convert old format (watch_directory string) to new format (watch_directories array)
    case {Map.get(config, "watch_directory"), Map.get(config, "watch_directories")} do
      {watch_dir, nil} when is_binary(watch_dir) and watch_dir != "" ->
        # Convert single watch_directory to watch_directories array
        config
        |> Map.put("watch_directories", [watch_dir])
        |> Map.delete("watch_directory")

      {_, _} ->
        # Already in new format or no watch directory specified
        config
    end
  end

  defp load_config_from_file do
    case find_config_file() do
      {:ok, config_path} ->
        case File.read(config_path) do
          {:ok, content} ->
            case Jason.decode(content) do
              {:ok, config} ->
                Logger.info("Configuration loaded from: #{config_path}")
                migrate_config_format(config)

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
    config_path = Path.expand(Path.join("~", @config_filename))

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
    home_dir_config = Path.expand(Path.join("~", @config_filename))

    if File.exists?(home_dir_config) do
      {:ok, home_dir_config}
    else
      {:error, :not_found}
    end
  end

  defp auto_detect_executables(config) do
    config
    |> auto_detect_executable("ffmpeg_path", "ffmpeg")
    |> auto_detect_executable("ffprobe_path", "ffprobe")
  end

  defp auto_detect_executable(config, config_key, executable_name) do
    case Map.get(config, config_key) do
      nil ->
        detect_and_set_executable(config, config_key, executable_name)

      "" ->
        detect_and_set_executable(config, config_key, executable_name)

      existing_path ->
        # Verify the existing path is still valid
        if File.exists?(existing_path) do
          config
        else
          Logger.warning("Configured #{executable_name} path no longer exists: #{existing_path}")
          detect_and_set_executable(config, config_key, executable_name)
        end
    end
  end

  defp detect_and_set_executable(config, config_key, executable_name) do
    case System.find_executable(executable_name) do
      nil ->
        Logger.warning("#{executable_name} not found in PATH")
        config

      path ->
        Logger.info("Auto-detected #{executable_name} at: #{path}")
        Map.put(config, config_key, path)
    end
  end
end
