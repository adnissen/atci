defmodule Autotranscript.Web.ConfigController do
  use Autotranscript.Web, :controller
  
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
    # Extract the config parameters
    config_params = %{
      "watch_directory" => params["watch_directory"],
      "whispercli_path" => params["whispercli_path"],
      "model_path" => params["model_path"]
    }
    
    # Validate the parameters
    case validate_config(config_params) do
      {:ok, validated_config} ->
        case ConfigManager.save_config(validated_config) do
          {:ok, _config_path} ->
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
  
  defp validate_config(config) do
    errors = []
    
    # Validate watch_directory
    errors = case config["watch_directory"] do
      nil -> ["watch_directory is required" | errors]
      "" -> ["watch_directory cannot be empty" | errors]
      path when is_binary(path) ->
        if File.dir?(path) do
          errors
        else
          ["watch_directory must be a valid directory path" | errors]
        end
      _ -> ["watch_directory must be a string" | errors]
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
end