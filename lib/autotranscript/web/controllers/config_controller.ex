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
              # Restart all applications with the new configuration
              case restart_applications(validated_config) do
                :ok ->
                  Logger.info("Applications restarted with new configuration")
                  
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
                    message: "Configuration saved successfully and applications restarted",
                    config: validated_config
                  }))
                {:error, reason} ->
                  Logger.error("Failed to restart applications: #{inspect(reason)}")
                  conn
                  |> put_status(:internal_server_error)
                  |> put_resp_content_type("application/json")
                  |> send_resp(500, Jason.encode!(%{
                    success: false,
                    message: "Configuration saved but failed to restart applications: #{inspect(reason)}"
                  }))
              end
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
  
  defp restart_applications(new_config) do
    # Update the application environment with the new config
    Application.put_env(:autotranscript, :atconfig, new_config)
    
    supervisor_pid = Process.whereis(Autotranscript.Supervisor)
    
    if supervisor_pid do
      # Get the current children specs
      children_specs = Supervisor.which_children(Autotranscript.Supervisor)
      
      # Restart each child process
      Enum.reduce_while(children_specs, :ok, fn {child_id, _child_pid, _type, _modules}, _acc ->
        case Supervisor.restart_child(Autotranscript.Supervisor, child_id) do
          {:ok, _pid} ->
            Logger.info("Restarted #{child_id} with new configuration")
            {:cont, :ok}
          {:error, :not_found} ->
            # Child spec not found, try to terminate and restart manually
            case restart_child_manually(child_id, new_config) do
              :ok -> {:cont, :ok}
              {:error, reason} -> {:halt, {:error, reason}}
            end
          {:error, reason} ->
            Logger.error("Failed to restart #{child_id}: #{inspect(reason)}")
            {:halt, {:error, reason}}
        end
      end)
    else
      {:error, :supervisor_not_found}
    end
  end
  
  defp restart_child_manually(child_id, new_config) do
    # Terminate the child first
    case Supervisor.terminate_child(Autotranscript.Supervisor, child_id) do
      :ok ->
        # Delete the child spec
        case Supervisor.delete_child(Autotranscript.Supervisor, child_id) do
          :ok ->
            # Add the child back with new configuration
            child_spec = case child_id do
              Autotranscript.VideoProcessor -> {Autotranscript.VideoProcessor, [atconfig: new_config]}
              Autotranscript.Transcriber -> {Autotranscript.Transcriber, [atconfig: new_config]}
              Autotranscript.Web.Endpoint -> {Autotranscript.Web.Endpoint, []}
              _ -> {:error, :unknown_child}
            end
            
            case child_spec do
              {:error, reason} -> {:error, reason}
              spec ->
                case Supervisor.start_child(Autotranscript.Supervisor, spec) do
                  {:ok, _pid} ->
                    Logger.info("Manually restarted #{child_id}")
                    :ok
                  {:error, reason} ->
                    Logger.error("Failed to manually restart #{child_id}: #{inspect(reason)}")
                    {:error, reason}
                end
            end
          {:error, reason} ->
            Logger.error("Failed to delete child #{child_id}: #{inspect(reason)}")
            {:error, reason}
        end
      {:error, reason} ->
        Logger.error("Failed to terminate child #{child_id}: #{inspect(reason)}")
        {:error, reason}
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