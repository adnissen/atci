defmodule Autotranscript.FFmpegManagerTest do
  use ExUnit.Case, async: true

  alias Autotranscript.FFmpegManager

  describe "detect_platform/0" do
    test "returns a valid platform string" do
      platform = FFmpegManager.detect_platform()
      assert platform in ["windows", "macos", "linux", "unknown"]
    end
  end

  describe "binaries_directory/0" do
    test "returns the expected directory path" do
      expected = Path.expand("~/.autotranscript/ffmpeg")
      assert FFmpegManager.binaries_directory() == expected
    end
  end

  describe "get_downloaded_path/1" do
    test "returns correct path for ffmpeg on unix systems" do
      # This test will vary based on the platform
      path = FFmpegManager.get_downloaded_path("ffmpeg")
      assert String.ends_with?(path, "ffmpeg") or String.ends_with?(path, "ffmpeg.exe")
    end

    test "returns correct path for ffprobe on unix systems" do
      path = FFmpegManager.get_downloaded_path("ffprobe")
      assert String.ends_with?(path, "ffprobe") or String.ends_with?(path, "ffprobe.exe")
    end
  end

  describe "list_tools/0" do
    test "returns a list of ffmpeg and ffprobe with their status" do
      tools = FFmpegManager.list_tools()

      assert length(tools) == 2
      assert Enum.all?(tools, &(&1.name in ["ffmpeg", "ffprobe"]))

      # Check that all required fields are present
      Enum.each(tools, fn tool ->
        assert Map.has_key?(tool, :name)
        assert Map.has_key?(tool, :platform)
        assert Map.has_key?(tool, :downloaded)
        assert Map.has_key?(tool, :downloaded_path)
        assert Map.has_key?(tool, :system_available)
        assert Map.has_key?(tool, :system_path)
        assert Map.has_key?(tool, :current_path)
      end)
    end
  end

  describe "download_tool/1" do
    test "rejects invalid tool names" do
      assert {:error, _} = FFmpegManager.download_tool("invalid_tool")
    end

    test "returns error for unknown platform" do
      # This test will only pass if we can mock the platform detection
      # For now, we'll skip it unless we're on an unknown platform
      if FFmpegManager.detect_platform() == "unknown" do
        assert {:error, "Unsupported platform"} = FFmpegManager.download_tool("ffmpeg")
      end
    end
  end
end
