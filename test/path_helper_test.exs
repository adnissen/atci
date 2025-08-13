defmodule Atci.PathHelperTest do
  use ExUnit.Case
  alias Atci.PathHelper

  describe "video_extensions/0" do
    test "includes mkv in supported extensions" do
      extensions = PathHelper.video_extensions()
      assert "mkv" in extensions
      assert "mp4" in extensions
      assert "mov" in extensions
    end
  end

  describe "video_extensions_with_dots/0" do
    test "includes mkv extensions with dots" do
      extensions = PathHelper.video_extensions_with_dots()
      assert ".mkv" in extensions
      assert ".MKV" in extensions
      assert ".mp4" in extensions
      assert ".MP4" in extensions
      assert ".mov" in extensions
      assert ".MOV" in extensions
    end
  end

  describe "video_wildcard_pattern/0" do
    test "includes mkv in wildcard pattern" do
      pattern = PathHelper.video_wildcard_pattern()
      assert String.contains?(pattern, "mkv")
      assert String.contains?(pattern, "MKV")
      assert String.contains?(pattern, "mp4")
      assert String.contains?(pattern, "MP4")
    end
  end

  describe "replace_video_extension_with/2" do
    test "replaces mkv extensions correctly" do
      assert PathHelper.replace_video_extension_with("test.mkv", ".txt") == "test.txt"
      assert PathHelper.replace_video_extension_with("test.MKV", ".txt") == "test.txt"
      assert PathHelper.replace_video_extension_with("test.mp4", ".txt") == "test.txt"
      assert PathHelper.replace_video_extension_with("test.mov", ".txt") == "test.txt"
    end
  end
end
