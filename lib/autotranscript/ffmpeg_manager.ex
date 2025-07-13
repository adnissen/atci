defmodule Autotranscript.FFmpegManager do
  @moduledoc """
  Manages FFmpeg and FFprobe binaries, including downloading them for different platforms.
  """

  require Logger

  # Platform-specific download URLs
  # Note: These are placeholder URLs as requested. Replace with actual URLs when available.
  @download_urls %{
    "windows" => %{
      "ffmpeg" => "https://example.com/ffmpeg-windows.exe",
      "ffprobe" => "https://example.com/ffprobe-windows.exe"
    },
    "macos" => %{
      "ffmpeg" => "https://example.com/ffmpeg-macos",
      "ffprobe" => "https://example.com/ffprobe-macos"
    },
    "linux" => %{
      "ffmpeg" => "https://example.com/ffmpeg-linux",
      "ffprobe" => "https://example.com/ffprobe-linux"
    }
  }

  @doc """
  Returns the directory where FFmpeg binaries are stored.
  """
  def binaries_directory do
    Path.expand("~/.autotranscript/ffmpeg")
  end

  @doc """
  Ensures the binaries directory exists.
  """
  def ensure_binaries_directory do
    dir = binaries_directory()
    File.mkdir_p(dir)
  end

  @doc """
  Detects the current platform.
  """
  def detect_platform do
    case :os.type() do
      {:win32, _} -> "windows"
      {:unix, :darwin} -> "macos"
      {:unix, _} -> "linux"
      _ -> "unknown"
    end
  end

  @doc """
  Lists FFmpeg and FFprobe with their download and availability status.
  """
  def list_tools do
    ensure_binaries_directory()
    platform = detect_platform()
    
    ["ffmpeg", "ffprobe"]
    |> Enum.map(fn tool ->
      downloaded_path = get_downloaded_path(tool)
      system_path = find_in_system_path(tool)
      
      %{
        name: tool,
        platform: platform,
        downloaded: File.exists?(downloaded_path),
        downloaded_path: downloaded_path,
        system_available: system_path != nil,
        system_path: system_path,
        current_path: get_current_path(tool)
      }
    end)
  end

  @doc """
  Gets the path where a tool would be stored if downloaded.
  """
  def get_downloaded_path(tool) do
    ext = if detect_platform() == "windows", do: ".exe", else: ""
    Path.join(binaries_directory(), "#{tool}#{ext}")
  end

  @doc """
  Finds a tool in the system PATH.
  """
  def find_in_system_path(tool) do
    case System.find_executable(tool) do
      nil -> nil
      path -> path
    end
  end

  @doc """
  Gets the current path being used for a tool (from config or auto-detected).
  """
  def get_current_path(tool) do
    config_path = Autotranscript.ConfigManager.get_config_value("#{tool}_path")
    
    if config_path && config_path != "" do
      config_path
    else
      find_in_system_path(tool) || get_downloaded_path(tool)
    end
  end

  @doc """
  Downloads a specific tool (ffmpeg or ffprobe) for the current platform.
  """
  def download_tool(tool) when tool in ["ffmpeg", "ffprobe"] do
    platform = detect_platform()
    
    if platform == "unknown" do
      {:error, "Unsupported platform"}
    else
      ensure_binaries_directory()
      
      url = get_in(@download_urls, [platform, tool])
      destination = get_downloaded_path(tool)
      
      Logger.info("Downloading #{tool} for #{platform} from #{url}")
      
      case download_file(url, destination) do
        :ok ->
          # Make the file executable on Unix-like systems
          if platform in ["macos", "linux"] do
            File.chmod(destination, 0o755)
          end
          
          Logger.info("Successfully downloaded #{tool}")
          {:ok, destination}
          
        {:error, reason} ->
          Logger.error("Failed to download #{tool}: #{inspect(reason)}")
          {:error, reason}
      end
    end
  end

  def download_tool(_tool) do
    {:error, "Invalid tool name. Only 'ffmpeg' and 'ffprobe' are supported."}
  end

  defp download_file(url, destination) do
    Logger.info("Starting download from #{url}")

    options = [
      timeout: 300_000,      # 5 minutes timeout
      recv_timeout: 300_000, # 5 minutes receive timeout
      follow_redirect: true
    ]

    case HTTPoison.get(url, [], options) do
      {:ok, %HTTPoison.Response{status_code: 200, body: body}} ->
        Logger.info("Download completed, saving to #{destination}")

        case File.write(destination, body) do
          :ok ->
            Logger.info("Successfully saved to #{destination}")
            :ok

          {:error, reason} ->
            Logger.error("Failed to write file: #{inspect(reason)}")
            {:error, "Failed to write file: #{inspect(reason)}"}
        end

      {:ok, %HTTPoison.Response{status_code: status_code}} ->
        Logger.error("HTTP request failed with status code: #{status_code}")
        {:error, "HTTP error: #{status_code}"}

      {:error, %HTTPoison.Error{reason: reason}} ->
        Logger.error("HTTP request failed: #{inspect(reason)}")
        {:error, "Download failed: #{inspect(reason)}"}
    end
  end

  @doc """
  Sets the path for a tool in the configuration.
  """
  def set_tool_path(tool, path) when tool in ["ffmpeg", "ffprobe"] do
    current_config = Autotranscript.ConfigManager.get_config()
    updated_config = Map.put(current_config, "#{tool}_path", path)
    Autotranscript.ConfigManager.save_config(updated_config)
  end

  @doc """
  Uses the downloaded version of a tool by updating the configuration.
  """
  def use_downloaded_version(tool) when tool in ["ffmpeg", "ffprobe"] do
    downloaded_path = get_downloaded_path(tool)
    
    if File.exists?(downloaded_path) do
      set_tool_path(tool, downloaded_path)
    else
      {:error, "Downloaded version not found"}
    end
  end

  @doc """
  Clears the custom path for a tool to use auto-detection.
  """
  def use_auto_detection(tool) when tool in ["ffmpeg", "ffprobe"] do
    set_tool_path(tool, "")
  end
end