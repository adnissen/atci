defmodule Autotranscript.Web.WhisperCliController do
  use Autotranscript.Web, :controller
  require Logger

  alias Autotranscript.WhisperCliManager

  @doc """
  Lists whisper-cli tools with their download and availability status.
  """
  def list(conn, _params) do
    tools = WhisperCliManager.list_tools()

    conn
    |> put_resp_content_type("application/json")
    |> send_resp(200, Jason.encode!(%{tools: tools}))
  end

  @doc """
  Downloads whisper-cli.
  """
  def download(conn, %{"tool_name" => tool_name}) do
    case WhisperCliManager.download_tool(tool_name) do
      {:ok, path} ->
        conn
        |> put_resp_content_type("application/json")
        |> send_resp(
          200,
          Jason.encode!(%{
            success: true,
            message: "#{tool_name} downloaded successfully",
            path: path
          })
        )

      {:error, reason} ->
        conn
        |> put_status(:bad_request)
        |> put_resp_content_type("application/json")
        |> send_resp(
          400,
          Jason.encode!(%{
            success: false,
            message: "Failed to download #{tool_name}",
            error: to_string(reason)
          })
        )
    end
  end

  def download(conn, _params) do
    conn
    |> put_status(:bad_request)
    |> put_resp_content_type("application/json")
    |> send_resp(
      400,
      Jason.encode!(%{
        success: false,
        message: "Tool name is required"
      })
    )
  end

  @doc """
  Sets the configuration to use the downloaded version of whisper-cli.
  """
  def use_downloaded(conn, %{"tool_name" => tool_name}) do
    case WhisperCliManager.use_downloaded_version(tool_name) do
      {:ok, config} ->
        conn
        |> put_resp_content_type("application/json")
        |> send_resp(
          200,
          Jason.encode!(%{
            success: true,
            message: "Now using downloaded #{tool_name}",
            config: config
          })
        )

      {:error, reason} ->
        conn
        |> put_status(:bad_request)
        |> put_resp_content_type("application/json")
        |> send_resp(
          400,
          Jason.encode!(%{
            success: false,
            message: "Failed to use downloaded #{tool_name}",
            error: to_string(reason)
          })
        )
    end
  end

  @doc """
  Sets the configuration to use auto-detection for whisper-cli.
  """
  def use_auto_detection(conn, %{"tool_name" => tool_name}) do
    case WhisperCliManager.use_auto_detection(tool_name) do
      {:ok, config} ->
        conn
        |> put_resp_content_type("application/json")
        |> send_resp(
          200,
          Jason.encode!(%{
            success: true,
            message: "Now using auto-detection for #{tool_name}",
            config: config
          })
        )

      {:error, reason} ->
        conn
        |> put_status(:bad_request)
        |> put_resp_content_type("application/json")
        |> send_resp(
          400,
          Jason.encode!(%{
            success: false,
            message: "Failed to set auto-detection for #{tool_name}",
            error: to_string(reason)
          })
        )
    end
  end
end
