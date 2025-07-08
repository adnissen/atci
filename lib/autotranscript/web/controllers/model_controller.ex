defmodule Autotranscript.Web.ModelController do
  use Autotranscript.Web, :controller
  require Logger
  
  alias Autotranscript.ModelManager
  
  @doc """
  Lists all available models and their download status.
  """
  def list(conn, _params) do
    models = ModelManager.list_models()
    
    conn
    |> put_resp_content_type("application/json")
    |> send_resp(200, Jason.encode!(%{models: models}))
  end
  
  @doc """
  Downloads a specific model.
  """
  def download(conn, %{"model_name" => model_name}) do
    case ModelManager.download_model(model_name) do
      {:ok, path} ->
        conn
        |> put_resp_content_type("application/json")
        |> send_resp(200, Jason.encode!(%{
          success: true,
          message: "Model downloaded successfully",
          path: path
        }))
      {:error, reason} ->
        conn
        |> put_status(:bad_request)
        |> put_resp_content_type("application/json")
        |> send_resp(400, Jason.encode!(%{
          success: false,
          message: "Failed to download model",
          error: to_string(reason)
        }))
    end
  end
  
  def download(conn, _params) do
    conn
    |> put_status(:bad_request)
    |> put_resp_content_type("application/json")
    |> send_resp(400, Jason.encode!(%{
      success: false,
      message: "Model name is required"
    }))
  end
end