defmodule Autotranscript.PathHelper do
  @moduledoc """
  Helper functions for working with video file paths and extensions.
  """

  # Define the list of video extensions we support
  @video_extensions ["mp4", "mov"]

  @doc """
  Returns the list of supported video extensions.
  """
  def video_extensions, do: @video_extensions

  @doc """
  Returns a wildcard pattern for Path.wildcard that includes both uppercase and lowercase versions of all video extensions.

  ## Examples
      iex> Autotranscript.PathHelper.video_wildcard_pattern()
      "{mp4,MP4}"
  """
  def video_wildcard_pattern do
    extensions = @video_extensions ++ Enum.map(@video_extensions, &String.upcase/1)
    "{#{Enum.join(extensions, ",")}}"
  end

  @doc """
  Returns a list of video extensions in both uppercase and lowercase formats with dots.

  ## Examples
      iex> Autotranscript.PathHelper.video_extensions_with_dots()
      [".mp4", ".MP4"]
  """
  def video_extensions_with_dots do
    @video_extensions
    |> Enum.flat_map(fn ext -> [".#{ext}", ".#{String.upcase(ext)}"] end)
  end

  @doc """
  Takes a directory and filename (without extension) and checks for the existence of a file
  with any of the supported video extensions (both uppercase and lowercase).
  Returns the full path if found, nil otherwise.

  ## Parameters
    - directory: String path to the directory
    - filename: String filename without extension

  ## Returns
    - String path to the found file, or nil if no file exists

  ## Examples
      iex> Autotranscript.PathHelper.find_video_file("/path/to/videos", "myvideo")
      "/path/to/videos/myvideo.mp4"

      iex> Autotranscript.PathHelper.find_video_file("/path/to/videos", "nonexistent")
      nil
  """
  def find_video_file(directory, filename) do
    @video_extensions
    |> Enum.flat_map(fn ext -> ["#{filename}.#{ext}", "#{filename}.#{String.upcase(ext)}"] end)
    |> Enum.map(fn file -> Path.join(directory, file) end)
    |> Enum.find(&File.exists?/1)
  end

  @doc """
  Checks if a video file exists for the given directory and filename.

  ## Parameters
    - directory: String path to the directory
    - filename: String filename without extension

  ## Returns
    - Boolean indicating whether a video file exists

  ## Examples
      iex> Autotranscript.PathHelper.video_file_exists?("/path/to/videos", "myvideo")
      true

      iex> Autotranscript.PathHelper.video_file_exists?("/path/to/videos", "nonexistent")
      false
  """
  def video_file_exists?(directory, filename) do
    find_video_file(directory, filename) != nil
  end

  @doc """
  Takes a video file path and returns the corresponding .txt file path if it exists, nil otherwise.
  Works with any supported video extension (uppercase or lowercase).

  ## Parameters
    - video_path: String path to the video file

  ## Returns
    - String path to the .txt file if it exists, or nil if no file exists

  ## Examples
      iex> Autotranscript.PathHelper.find_txt_file_from_video("/path/to/videos/myvideo.mp4")
      "/path/to/videos/myvideo.txt"

      iex> Autotranscript.PathHelper.find_txt_file_from_video("/path/to/videos/nonexistent.mp4")
      nil
  """
  def find_txt_file_from_video(video_path) do
    txt_path = replace_video_extension_with(video_path, ".txt")
    if File.exists?(txt_path), do: txt_path, else: nil
  end

  @doc """
  Takes a video file path and returns the corresponding .meta file path if it exists, nil otherwise.
  Works with any supported video extension (uppercase or lowercase).

  ## Parameters
    - video_path: String path to the video file

  ## Returns
    - String path to the .meta file if it exists, or nil if no file exists

  ## Examples
      iex> Autotranscript.PathHelper.find_meta_file_from_video("/path/to/videos/myvideo.mp4")
      "/path/to/videos/myvideo.meta"

      iex> Autotranscript.PathHelper.find_meta_file_from_video("/path/to/videos/nonexistent.mp4")
      nil
  """
  def find_meta_file_from_video(video_path) do
    meta_path = replace_video_extension_with(video_path, ".meta")
    if File.exists?(meta_path), do: meta_path, else: nil
  end

  @doc """
  Takes a video file path and returns the corresponding .mp3 file path if it exists, nil otherwise.
  Works with any supported video extension (uppercase or lowercase).

  ## Parameters
    - video_path: String path to the video file

  ## Returns
    - String path to the .mp3 file if it exists, or nil if no file exists

  ## Examples
      iex> Autotranscript.PathHelper.find_mp3_file_from_video("/path/to/videos/myvideo.mp4")
      "/path/to/videos/myvideo.mp3"

      iex> Autotranscript.PathHelper.find_mp3_file_from_video("/path/to/videos/nonexistent.mp4")
      nil
  """
  def find_mp3_file_from_video(video_path) do
    # Try both lowercase and uppercase mp3 extensions
    mp3_path_lower = replace_video_extension_with(video_path, ".mp3")
    mp3_path_upper = replace_video_extension_with(video_path, ".MP3")

    cond do
      File.exists?(mp3_path_lower) -> mp3_path_lower
      File.exists?(mp3_path_upper) -> mp3_path_upper
      true -> nil
    end
  end

  # Helper function to replace video extension with a new extension
  def replace_video_extension_with(video_path, new_extension) do
    video_extensions_with_dots()
    |> Enum.reduce(video_path, fn ext, path ->
      String.replace_trailing(path, ext, new_extension)
    end)
  end
end
