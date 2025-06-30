defmodule Autotranscript.Web.ConfigController do
  use Autotranscript.Web, :controller

  alias Autotranscript.Config

  @doc """
  GET /api/config
  Returns the current configuration as JSON.
  """
  def show(conn, _params) do
    config = Config.get_all()
    
    response = Map.merge(config, %{
      valid: Config.valid?(),
      config_file_path: Config.config_file_path()
    })

    conn
    |> put_resp_content_type("application/json")
    |> send_resp(200, Jason.encode!(response))
  end

  @doc """
  POST /api/config
  Updates the configuration with provided values.
  """
  def update(conn, params) do
    # Extract config values from params
    config_updates = %{}
    |> maybe_put(:watch_directory, params["watch_directory"])
    |> maybe_put(:whispercli_path, params["whispercli_path"])
    |> maybe_put(:model_path, params["model_path"])

    case Config.save(config_updates) do
      :ok ->
        # Return updated config
        updated_config = Config.get_all()
        response = Map.merge(updated_config, %{
          valid: Config.valid?(),
          config_file_path: Config.config_file_path()
        })

        conn
        |> put_resp_content_type("application/json")
        |> send_resp(200, Jason.encode!(response))

      {:error, reason} ->
        conn
        |> put_status(:internal_server_error)
        |> put_resp_content_type("application/json")
        |> send_resp(500, Jason.encode!(%{error: "Failed to save configuration: #{reason}"}))
    end
  end

  @doc """
  GET /api/config/status
  Returns whether the configuration is valid and complete.
  """
  def status(conn, _params) do
    response = %{
      valid: Config.valid?(),
      config_file_path: Config.config_file_path(),
      config: Config.get_all()
    }

    conn
    |> put_resp_content_type("application/json")
    |> send_resp(200, Jason.encode!(response))
  end

  # Private helper function
  defp maybe_put(map, _key, nil), do: map
  defp maybe_put(map, _key, ""), do: map
  defp maybe_put(map, key, value) when is_binary(value) do
    Map.put(map, key, String.trim(value))
  end
  defp maybe_put(map, _key, _value), do: map
end