defmodule Autotranscript.ModelManager do
  @moduledoc """
  Manages Whisper models, including listing available models and downloading them.
  """

  require Logger
  
  @model_names [
    "ggml-base-q5_1",
    "ggml-base-q8_0",
    "ggml-base",
    "ggml-base.en-q5_1",
    "ggml-base.en-q8_0",
    "ggml-base.en",
    "ggml-large-v1",
    "ggml-large-v2-q5_0",
    "ggml-large-v2-q8_0",
    "ggml-large-v2",
    "ggml-large-v3-q5_0",
    "ggml-large-v3-turbo-q5_0",
    "ggml-large-v3-turbo-q8_0",
    "ggml-large-v3-turbo",
    "ggml-large-v3",
    "ggml-medium-q5_0",
    "ggml-medium-q8_0",
    "ggml-medium",
    "ggml-medium.en-q5_0",
    "ggml-medium.en-q8_0",
    "ggml-medium.en",
    "ggml-small-q5_1",
    "ggml-small-q8_0",
    "ggml-small",
    "ggml-small.en-q5_1",
    "ggml-small.en-q8_0",
    "ggml-small.en",
    "ggml-tiny-q5_1",
    "ggml-tiny-q8_0",
    "ggml-tiny",
    "ggml-tiny.en-q5_1",
    "ggml-tiny.en-q8_0",
    "ggml-tiny.en"
  ]
  
  @huggingface_base_url "https://huggingface.co/ggerganov/whisper.cpp/resolve/main"
  
  @doc """
  Returns the directory where models are stored.
  """
  def models_directory do
    Path.expand("~/.autotranscript/models")
  end
  
  @doc """
  Ensures the models directory exists.
  """
  def ensure_models_directory do
    dir = models_directory()
    File.mkdir_p(dir)
  end
  
  @doc """
  Lists all available models with their download status.
  """
  def list_models do
    ensure_models_directory()
    
    Enum.map(@model_names, fn model_name ->
      path = Path.join(models_directory(), "#{model_name}.bin")
      %{
        name: model_name,
        downloaded: File.exists?(path),
        path: path
      }
    end)
  end
  
  @doc """
  Downloads a model from Hugging Face.
  """
  def download_model(model_name) do
    if model_name not in @model_names do
      {:error, "Invalid model name"}
    else
      ensure_models_directory()
      
      model_path = Path.join(models_directory(), "#{model_name}.bin")
      url = "#{@huggingface_base_url}/#{model_name}.bin"
      
      Logger.info("Downloading model #{model_name} from #{url}")
      
      # Use httpc to download the file
      case download_file(url, model_path) do
        :ok ->
          Logger.info("Successfully downloaded model #{model_name}")
          {:ok, model_path}
        {:error, reason} ->
          Logger.error("Failed to download model #{model_name}: #{inspect(reason)}")
          {:error, reason}
      end
    end
  end
  
  defp download_file(url, destination) do
    # Ensure httpc is started
    :inets.start()
    :ssl.start()
    
    # Create a temporary file to download to
    temp_file = "#{destination}.tmp"
    
    # Download with progress logging
    case :httpc.request(:get, {String.to_charlist(url), []}, 
                       [{:timeout, :infinity}, {:connect_timeout, 30_000}], 
                       [{:stream, String.to_charlist(temp_file)}]) do
      {:ok, :saved_to_file} ->
        # Move the temp file to the final destination
        case File.rename(temp_file, destination) do
          :ok -> :ok
          {:error, reason} -> 
            File.rm(temp_file)
            {:error, reason}
        end
      {:ok, {{_, status_code, _}, _, _}} ->
        File.rm(temp_file)
        {:error, "HTTP error: #{status_code}"}
      {:error, reason} ->
        File.rm(temp_file)
        {:error, reason}
    end
  end
  
  @doc """
  Gets the full list of available model names.
  """
  def get_model_names do
    @model_names
  end
end