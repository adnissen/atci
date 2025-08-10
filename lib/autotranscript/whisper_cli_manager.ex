defmodule Autotranscript.WhisperCliManager do
  @moduledoc """
  Manages whisper-cli binaries, including downloading them for different platforms.
  """

  require Logger

  # Platform-specific download URLs
  # Note: These are placeholder URLs. Replace with actual URLs when available.
  @download_urls %{
    "windows" => %{
      "whisper-cli" => "https://example.com/whisper-cli-windows.exe"
    },
    "macos-arm" => %{
      "whisper-cli" => "https://autotranscript.s3.us-east-1.amazonaws.com/binaries/whisper-cli"
    },
    "macos-x86" => %{
      "whisper-cli" => "https://example.com/whisper-cli-macos-x86"
    },
    "linux" => %{
      "whisper-cli" => "https://example.com/whisper-cli-linux"
    }
  }

  @doc """
  Returns the directory where whisper-cli binaries are stored.
  """
  def binaries_directory do
    Path.expand("~/.autotranscript/whisper-cli")
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
      {:win32, _} ->
        "windows"

      {:unix, :darwin} ->
        case System.cmd("uname", ["-m"]) do
          {"arm64\n", 0} -> "macos-arm"
          _ -> "macos-x86"
        end

      {:unix, _} ->
        "linux"
    end
  end

  @doc """
  Lists whisper-cli with its download and availability status.
  """
  def list_tools do
    ensure_binaries_directory()
    platform = detect_platform()

    ["whisper-cli"]
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
    # Use whispercli_path to maintain compatibility with existing config
    config_path = Autotranscript.ConfigManager.get_config_value("whispercli_path")

    if config_path && config_path != "" do
      config_path
    else
      find_in_system_path(tool) || get_downloaded_path(tool)
    end
  end

  @doc """
  Downloads whisper-cli for the current platform.
  """
  def download_tool("whisper-cli" = tool) do
    platform = detect_platform()

    if platform == "unknown" do
      {:error, "Unsupported platform"}
    else
      ensure_binaries_directory()

      url = get_in(@download_urls, [platform, tool])
      destination = get_downloaded_path(tool)

      Logger.info("Downloading #{tool} for #{platform} from #{url}")

      case download_file(url, destination, platform) do
        :ok ->
          # Make the file executable on Unix-like systems
          if platform in ["macos-arm", "macos-x86", "linux"] do
            File.chmod(destination, 0o755)
          end

          # Handle macOS quarantine removal and code signing
          if platform in ["macos-arm", "macos-x86"] do
            case handle_macos_quarantine(destination, platform) do
              :ok ->
                Logger.info("Successfully handled macOS quarantine for #{tool}")

              {:error, reason} ->
                Logger.warning("Failed to handle macOS quarantine for #{tool}: #{reason}")
                # Don't fail the download, just log the warning
            end
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
    {:error, "Invalid tool name. Only 'whisper-cli' is supported."}
  end

  defp download_file(url, destination, platform) do
    Logger.info("Starting download from #{url}")

    options = [
      # 5 minutes timeout
      timeout: 300_000,
      # 5 minutes receive timeout
      recv_timeout: 300_000,
      follow_redirect: true
    ]

    case HTTPoison.get(url, [], options) do
      {:ok, %HTTPoison.Response{status_code: 200, body: body}} ->
        Logger.info("Download completed, processing for #{platform}")

        # For now, save directly. Adjust if whisper-cli comes in different packaging
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

  defp handle_macos_quarantine(executable_path, platform) do
    with :ok <- check_macos_version(),
         :ok <- remove_quarantine(executable_path),
         :ok <- handle_arm_mac_signing(executable_path, platform) do
      :ok
    else
      {:error, reason} -> {:error, reason}
    end
  end

  defp check_macos_version do
    case System.cmd("sw_vers", ["-productVersion"]) do
      {version_string, 0} ->
        version_string
        |> String.trim()
        |> String.split(".")
        |> case do
          [major, minor | _] ->
            major_int = String.to_integer(major)
            minor_int = String.to_integer(minor)

            if major_int > 10 or (major_int == 10 and minor_int >= 15) do
              :ok
            else
              {:error, "macOS version too old for quarantine handling"}
            end

          _ ->
            {:error, "Unable to parse macOS version"}
        end

      {_, _} ->
        {:error, "Unable to get macOS version"}
    end
  end

  defp remove_quarantine(executable_path) do
    Logger.info("Removing quarantine from #{executable_path}")

    case System.cmd("xattr", ["-dr", "com.apple.quarantine", executable_path]) do
      {_, 0} ->
        Logger.info("Successfully removed quarantine")
        :ok

      {output, exit_code} ->
        Logger.warning("xattr command failed with exit code #{exit_code}: #{output}")
        {:error, "Failed to remove quarantine: #{output}"}
    end
  end

  defp handle_arm_mac_signing(executable_path, platform) do
    if platform == "macos-arm" do
      Logger.info("Handling ARM Mac code signing for #{executable_path}")

      with :ok <- clear_extended_attributes(executable_path),
           :ok <- codesign_executable(executable_path) do
        :ok
      else
        {:error, reason} -> {:error, reason}
      end
    else
      :ok
    end
  end

  defp clear_extended_attributes(executable_path) do
    Logger.info("Clearing extended attributes")

    case System.cmd("xattr", ["-cr", executable_path]) do
      {_, 0} ->
        Logger.info("Successfully cleared extended attributes")
        :ok

      {output, exit_code} ->
        Logger.warning("xattr -cr command failed with exit code #{exit_code}: #{output}")
        {:error, "Failed to clear extended attributes: #{output}"}
    end
  end

  defp codesign_executable(executable_path) do
    Logger.info("Code signing executable")

    case System.cmd("codesign", ["-s", "-", executable_path]) do
      {_, 0} ->
        Logger.info("Successfully code signed executable")
        :ok

      {output, exit_code} ->
        Logger.warning("codesign command failed with exit code #{exit_code}: #{output}")
        {:error, "Failed to code sign executable: #{output}"}
    end
  end

  @doc """
  Sets the path for whisper-cli in the configuration.
  """
  def set_tool_path("whisper-cli" = tool, path) do
    current_config = Autotranscript.ConfigManager.get_config()
    # Use whispercli_path to maintain compatibility with existing config
    updated_config = Map.put(current_config, "whispercli_path", path)
    Autotranscript.ConfigManager.save_config(updated_config)
  end

  @doc """
  Uses the downloaded version of whisper-cli by updating the configuration.
  """
  def use_downloaded_version("whisper-cli" = tool) do
    downloaded_path = get_downloaded_path(tool)

    if File.exists?(downloaded_path) do
      set_tool_path(tool, downloaded_path)
    else
      {:error, "Downloaded version not found"}
    end
  end

  @doc """
  Clears the custom path for whisper-cli to use auto-detection.
  """
  def use_auto_detection("whisper-cli" = tool) do
    set_tool_path(tool, "")
  end
end
