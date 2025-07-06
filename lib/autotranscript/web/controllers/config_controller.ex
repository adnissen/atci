defmodule Autotranscript.Web.ConfigController do
  use Autotranscript.Web, :controller
  require Logger
  
  alias Autotranscript.ConfigManager
  
  @doc """
  Returns the current configuration status and values.
  """
  def show(conn, _params) do
    config = ConfigManager.get_config()
    is_complete = ConfigManager.config_complete?(config)
    
    response = %{
      config: config,
      is_complete: is_complete
    }
    
    conn
    |> put_resp_content_type("application/json")
    |> send_resp(200, Jason.encode!(response))
  end
  
  @doc """
  Updates the configuration with new values.
  """
  def update(conn, params) do
    # Extract the config parameters with backward compatibility
    config_params = %{
      "watch_directories" => extract_watch_directories(params),
      "whispercli_path" => params["whispercli_path"],
      "model_path" => params["model_path"]
    }
    
    # Validate the parameters
    case validate_config(config_params) do
        {:ok, validated_config} ->
          case ConfigManager.save_config(validated_config) do
            {:ok, _config_path} ->
              # Try to start directory watching now that configuration is available
              case Autotranscript.Transcriber.start_watching() do
                :ok ->
                  Logger.info("Directory watching started after configuration update")
                {:error, reason} ->
                  Logger.warning("Could not start directory watching: #{inspect(reason)}")
              end
              
              conn
              |> put_resp_content_type("application/json")
              |> send_resp(200, Jason.encode!(%{
                success: true,
                message: "Configuration saved successfully",
                config: validated_config
              }))
            {:error, reason} ->
              conn
              |> put_status(:internal_server_error)
              |> put_resp_content_type("application/json")
              |> send_resp(500, Jason.encode!(%{
                success: false,
                message: "Failed to save configuration: #{inspect(reason)}"
              }))
          end
      {:error, errors} ->
        conn
        |> put_status(:bad_request)
        |> put_resp_content_type("application/json")
        |> send_resp(400, Jason.encode!(%{
          success: false,
          message: "Invalid configuration",
          errors: errors
        }))
    end
  end
  
  # Extract watch directories with backward compatibility
  defp extract_watch_directories(params) do
    cond do
      # New format: watch_directories as array
      is_list(params["watch_directories"]) ->
        params["watch_directories"]
      
      # Backward compatibility: single watch_directory
      is_binary(params["watch_directory"]) and params["watch_directory"] != "" ->
        [params["watch_directory"]]
      
      # Default to empty list
      true ->
        []
    end
  end
  
  defp validate_config(config) do
    errors = []
    
    # Validate watch_directories
    errors = case config["watch_directories"] do
      nil -> ["watch_directories is required" | errors]
      [] -> ["watch_directories cannot be empty" | errors]
      directories when is_list(directories) ->
        validate_watch_directories(directories, errors)
      _ -> ["watch_directories must be a list" | errors]
    end
    
    # Validate whispercli_path
    errors = case config["whispercli_path"] do
      nil -> ["whispercli_path is required" | errors]
      "" -> ["whispercli_path cannot be empty" | errors]
      path when is_binary(path) ->
        if File.exists?(path) do
          errors
        else
          ["whispercli_path must be a valid file path" | errors]
        end
      _ -> ["whispercli_path must be a string" | errors]
    end
    
    # Validate model_path
    errors = case config["model_path"] do
      nil -> ["model_path is required" | errors]
      "" -> ["model_path cannot be empty" | errors]
      path when is_binary(path) ->
        if File.exists?(path) do
          errors
        else
          ["model_path must be a valid file path" | errors]
        end
      _ -> ["model_path must be a string" | errors]
    end
    
    case errors do
      [] -> {:ok, config}
      _ -> {:error, Enum.reverse(errors)}
    end
  end
  
  defp validate_watch_directories(directories, errors) do
    # Check if all directories are valid strings and exist
    string_errors = Enum.reduce(directories, errors, fn dir, acc ->
      case dir do
        path when is_binary(path) and path != "" ->
          if File.dir?(path) do
            acc
          else
            ["watch_directory '#{path}' must be a valid directory path" | acc]
          end
        _ ->
          ["all watch_directories must be non-empty strings" | acc]
      end
    end)
    
    # Check for subdirectories
    subdirectory_errors = if has_subdirectories?(directories) do
      ["watch_directories cannot contain subdirectories of other watch directories" | string_errors]
    else
      string_errors
    end
    
    subdirectory_errors
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
end