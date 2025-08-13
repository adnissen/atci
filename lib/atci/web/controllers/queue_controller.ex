defmodule Atci.Web.QueueController do
  use Atci.Web, :controller
  require Logger

  alias Atci.VideoProcessor

  @doc """
  Gets the current queue status including the queue contents, processing state, and currently processing file.
  """
  def status(conn, _params) do
    queue_status = VideoProcessor.get_queue_status()

    # Transform tuples to maps for JSON serialization
    transformed_queue_status = %{
      queue:
        Enum.map(queue_status.queue, fn {process_type, %{path: video_path, time: time}} ->
          %{
            path: video_path,
            process_type: process_type,
            time: time
          }
        end),
      processing_state: if(queue_status.processing, do: "processing", else: "idle"),
      current_processing:
        case queue_status.current_file do
          {process_type, %{path: video_path, time: time}} ->
            %{path: video_path, process_type: process_type, time: time}

          nil ->
            nil
        end
    }

    conn
    |> put_resp_content_type("application/json")
    |> send_resp(200, Jason.encode!(transformed_queue_status))
  end

  @spec remove_job(Plug.Conn.t(), any()) :: Plug.Conn.t()
  @doc """
  Removes a specific job from the queue.
  Expects JSON body with process_type, path, and optional time.
  """
  def remove_job(conn, params) do
    case extract_job_tuple_from_params(params) do
      {:ok, job_tuple} ->
        case VideoProcessor.remove_from_queue(job_tuple) do
          :ok ->
            conn
            |> put_resp_content_type("application/json")
            |> send_resp(
              200,
              Jason.encode!(%{
                success: true,
                message: "Job removed from queue successfully"
              })
            )

          {:error, :not_found} ->
            conn
            |> put_status(:not_found)
            |> put_resp_content_type("application/json")
            |> send_resp(
              404,
              Jason.encode!(%{
                success: false,
                message: "Job not found in queue"
              })
            )
        end

      {:error, reason} ->
        conn
        |> put_status(:bad_request)
        |> put_resp_content_type("application/json")
        |> send_resp(
          400,
          Jason.encode!(%{
            success: false,
            message: "Invalid job parameters",
            error: reason
          })
        )
    end
  end

  @doc """
  Reorders the queue to match the provided list of job tuples.
  Expects JSON body with "queue" containing an array of job objects.
  """
  def reorder(conn, %{"queue" => queue_params}) when is_list(queue_params) do
    case extract_job_tuples_from_list(queue_params) do
      {:ok, job_tuples} ->
        :ok = VideoProcessor.reorder_queue(job_tuples)

        conn
        |> put_resp_content_type("application/json")
        |> send_resp(
          200,
          Jason.encode!(%{
            success: true,
            message: "Queue reordered successfully"
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
            message: "Invalid queue parameters",
            error: reason
          })
        )
    end
  end

  def reorder(conn, _params) do
    conn
    |> put_status(:bad_request)
    |> put_resp_content_type("application/json")
    |> send_resp(
      400,
      Jason.encode!(%{
        success: false,
        message: "Missing 'queue' parameter"
      })
    )
  end

  @doc """
  Cancels the currently processing job and moves to the next job in the queue.
  """
  def cancel_current(conn, _params) do
    case VideoProcessor.cancel_current_job() do
      :ok ->
        conn
        |> put_resp_content_type("application/json")
        |> send_resp(
          200,
          Jason.encode!(%{
            success: true,
            message: "Current job cancelled successfully"
          })
        )

      {:error, :no_current_job} ->
        conn
        |> put_status(:not_found)
        |> put_resp_content_type("application/json")
        |> send_resp(
          404,
          Jason.encode!(%{
            success: false,
            message: "No job is currently processing"
          })
        )
    end
  end

  # Helper function to extract a job tuple from request parameters
  defp extract_job_tuple_from_params(%{"process_type" => process_type, "path" => path} = params) do
    case parse_process_type(process_type) do
      {:ok, parsed_process_type} ->
        time = Map.get(params, "time")
        job_info = %{path: path, time: time}
        {:ok, {parsed_process_type, job_info}}

      {:error, reason} ->
        {:error, reason}
    end
  end

  defp extract_job_tuple_from_params(_params) do
    {:error, "Missing required parameters: process_type and path"}
  end

  # Helper function to extract job tuples from a list of parameters
  defp extract_job_tuples_from_list(queue_params) do
    try do
      job_tuples =
        Enum.map(queue_params, fn job_params ->
          case extract_job_tuple_from_params(job_params) do
            {:ok, job_tuple} -> job_tuple
            {:error, reason} -> throw({:error, reason})
          end
        end)

      {:ok, job_tuples}
    catch
      {:error, reason} -> {:error, reason}
    end
  end

  # Helper function to parse and validate process_type
  defp parse_process_type("all"), do: {:ok, :all}
  defp parse_process_type("length"), do: {:ok, :length}
  defp parse_process_type("partial"), do: {:ok, :partial}
  defp parse_process_type(:all), do: {:ok, :all}
  defp parse_process_type(:length), do: {:ok, :length}
  defp parse_process_type(:partial), do: {:ok, :partial}

  defp parse_process_type(invalid) do
    {:error, "Invalid process_type: #{inspect(invalid)}. Must be 'all', 'length', or 'partial'"}
  end
end
