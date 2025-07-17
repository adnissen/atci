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
    "macos-arm" => %{
      "ffmpeg" => "https://www.osxexperts.net/ffmpeg711arm.zip",
      "ffprobe" => "https://www.osxexperts.net/ffprobe711arm.zip"
    },
    "macos-x86" => %{
      "ffmpeg" => "https://www.osxexperts.net/ffmpeg71intel.zip",
      "ffprobe" => "https://www.osxexperts.net/ffprobe71intel.zip"
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
      {:unix, :darwin} ->
        case System.cmd("uname", ["-m"]) do
          {"arm64\n", 0} -> "macos-arm"
          _ -> "macos-x86"
        end
      {:unix, _} -> "linux"
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
    {:error, "Invalid tool name. Only 'ffmpeg' and 'ffprobe' are supported."}
  end

  defp download_file(url, destination, platform) do
    Logger.info("Starting download from #{url}")

    options = [
      timeout: 300_000,      # 5 minutes timeout
      recv_timeout: 300_000, # 5 minutes receive timeout
      follow_redirect: true
    ]

    case HTTPoison.get(url, [], options) do
      {:ok, %HTTPoison.Response{status_code: 200, body: body}} ->
        Logger.info("Download completed, processing for #{platform}")

        if platform in ["macos-arm", "macos-x86"] do
          # For macOS, the downloaded file is a zip that needs to be extracted
          extract_zip_file(body, destination)
        else
          # For other platforms, save directly
          case File.write(destination, body) do
            :ok ->
              Logger.info("Successfully saved to #{destination}")
              :ok

            {:error, reason} ->
              Logger.error("Failed to write file: #{inspect(reason)}")
              {:error, "Failed to write file: #{inspect(reason)}"}
          end
        end

      {:ok, %HTTPoison.Response{status_code: status_code}} ->
        Logger.error("HTTP request failed with status code: #{status_code}")
        {:error, "HTTP error: #{status_code}"}

      {:error, %HTTPoison.Error{reason: reason}} ->
        Logger.error("HTTP request failed: #{inspect(reason)}")
        {:error, "Download failed: #{inspect(reason)}"}
    end
  end

  defp validate_extracted_binary(binary_path, expected_tool) do
    # Make the binary executable first
    File.chmod(binary_path, 0o755)

    # Run the binary with --version to check what it actually is
    case System.cmd(binary_path, ["--version"], stderr_to_stdout: true) do
      {output, 0} ->
        output_lower = String.downcase(output)

        # Check if the output contains the expected tool name
        if String.contains?(output_lower, expected_tool) do
          # Also check that it doesn't contain the OTHER tool name
          other_tool = if expected_tool == "ffmpeg", do: "ffprobe", else: "ffmpeg"

          if String.contains?(output_lower, other_tool) and not String.contains?(output_lower, expected_tool) do
            {:error, "Binary reports as #{other_tool}, not #{expected_tool}"}
          else
            :ok
          end
        else
          {:error, "Binary does not report as #{expected_tool}"}
        end

      {output, _} ->
        {:error, "Failed to run binary: #{output}"}
    end
  end

  defp extract_zip_file(zip_data, destination) do
    # Create a temporary file for the zip
    temp_zip_path = destination <> ".zip"

    case File.write(temp_zip_path, zip_data) do
      :ok ->
        Logger.info("Extracting zip file to #{destination}")

        # Extract the zip file
        case System.cmd("unzip", ["-o", temp_zip_path, "-d", Path.dirname(destination)]) do
          {_, 0} ->
            Logger.info("Successfully extracted zip file to #{destination}")
            :ok
          {output, _exit_code} ->
            File.rm(temp_zip_path)
            Logger.error("Failed to extract zip: #{output}")
            {:error, "Failed to extract zip: #{output}"}
        end
      {:error, reason} ->
        Logger.error("Failed to write temporary zip file: #{inspect(reason)}")
        {:error, "Failed to write temporary zip file: #{inspect(reason)}"}
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
