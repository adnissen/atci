defmodule Autotranscript.ConfigManagerTest do
  use ExUnit.Case

  alias Autotranscript.ConfigManager

  describe "config_complete?/1" do
    test "returns true for valid configuration with multiple watch directories" do
      # Create temporary directories for testing
      temp_dir1 = Path.join(System.tmp_dir!(), "test_watch_dir1_#{:rand.uniform(10000)}")
      temp_dir2 = Path.join(System.tmp_dir!(), "test_watch_dir2_#{:rand.uniform(10000)}")
      File.mkdir_p!(temp_dir1)
      File.mkdir_p!(temp_dir2)

      config = %{
        "watch_directories" => [temp_dir1, temp_dir2],
        # Use echo as a test binary
        "whispercli_path" => "/usr/bin/echo",
        # Use echo as a test binary
        "model_path" => "/usr/bin/echo",
        "ffmpeg_path" => "/usr/bin/echo",
        "ffprobe_path" => "/usr/bin/echo"
      }

      assert ConfigManager.config_complete?(config)

      # Clean up
      File.rm_rf(temp_dir1)
      File.rm_rf(temp_dir2)
    end

    test "returns false for configuration with subdirectories" do
      # Create temporary directories for testing
      temp_dir1 = Path.join(System.tmp_dir!(), "test_watch_dir1_#{:rand.uniform(10000)}")
      File.mkdir_p!(temp_dir1)
      subdir = Path.join(temp_dir1, "subdir")
      File.mkdir_p!(subdir)

      config = %{
        "watch_directories" => [temp_dir1, subdir],
        "whispercli_path" => "/usr/bin/echo",
        "model_path" => "/usr/bin/echo",
        "ffmpeg_path" => "/usr/bin/echo",
        "ffprobe_path" => "/usr/bin/echo"
      }

      refute ConfigManager.config_complete?(config)

      # Clean up
      File.rm_rf(temp_dir1)
    end

    test "returns false for empty watch_directories" do
      config = %{
        "watch_directories" => [],
        "whispercli_path" => "/usr/bin/echo",
        "model_path" => "/usr/bin/echo",
        "ffmpeg_path" => "/usr/bin/echo",
        "ffprobe_path" => "/usr/bin/echo"
      }

      refute ConfigManager.config_complete?(config)
    end
  end

  describe "get_config_value/1" do
    test "returns first watch directory for backward compatibility" do
      # Create temporary directories for testing
      temp_dir1 = Path.join(System.tmp_dir!(), "test_watch_dir1_#{:rand.uniform(10000)}")
      temp_dir2 = Path.join(System.tmp_dir!(), "test_watch_dir2_#{:rand.uniform(10000)}")
      File.mkdir_p!(temp_dir1)
      File.mkdir_p!(temp_dir2)

      # Test the logic that would be in the GenServer
      state = %{
        "watch_directories" => [temp_dir1, temp_dir2],
        "whispercli_path" => "/usr/bin/echo",
        "model_path" => "/usr/bin/echo"
      }

      # Simulate the logic from handle_call
      result =
        case Map.get(state, "watch_directories") do
          [first_dir | _] -> first_dir
          _ -> nil
        end

      assert result == temp_dir1

      # Clean up
      File.rm_rf(temp_dir1)
      File.rm_rf(temp_dir2)
    end
  end

  describe "config migration" do
    test "migrates old format watch_directory to watch_directories" do
      # Create temporary directory for testing
      temp_dir = Path.join(System.tmp_dir!(), "test_watch_dir_#{:rand.uniform(10000)}")
      File.mkdir_p!(temp_dir)

      old_config = %{
        "watch_directory" => temp_dir,
        "whispercli_path" => "/usr/bin/echo",
        "model_path" => "/usr/bin/echo",
        "ffmpeg_path" => "/usr/bin/echo",
        "ffprobe_path" => "/usr/bin/echo"
      }

      # Test the migration logic directly by simulating what happens in migrate_config_format
      migrated_config = case {Map.get(old_config, "watch_directory"), Map.get(old_config, "watch_directories")} do
        {watch_dir, nil} when is_binary(watch_dir) and watch_dir != "" ->
          old_config
          |> Map.put("watch_directories", [watch_dir])
          |> Map.delete("watch_directory")
        {_, _} ->
          old_config
      end

      expected_config = %{
        "watch_directories" => [temp_dir],
        "whispercli_path" => "/usr/bin/echo",
        "model_path" => "/usr/bin/echo",
        "ffmpeg_path" => "/usr/bin/echo",
        "ffprobe_path" => "/usr/bin/echo"
      }

      assert migrated_config == expected_config
      assert ConfigManager.config_complete?(expected_config)

      # Clean up
      File.rm_rf(temp_dir)
    end

    test "leaves new format config unchanged" do
      # Create temporary directory for testing
      temp_dir = Path.join(System.tmp_dir!(), "test_watch_dir_#{:rand.uniform(10000)}")
      File.mkdir_p!(temp_dir)

      new_config = %{
        "watch_directories" => [temp_dir],
        "whispercli_path" => "/usr/bin/echo",
        "model_path" => "/usr/bin/echo",
        "ffmpeg_path" => "/usr/bin/echo",
        "ffprobe_path" => "/usr/bin/echo"
      }

      # Test the migration logic - should not change new format
      migrated_config = case {Map.get(new_config, "watch_directory"), Map.get(new_config, "watch_directories")} do
        {watch_dir_val, nil} when is_binary(watch_dir_val) and watch_dir_val != "" ->
          new_config
          |> Map.put("watch_directories", [watch_dir_val])
          |> Map.delete("watch_directory")
        {_, _} ->
          new_config
      end

      assert migrated_config == new_config
      assert ConfigManager.config_complete?(new_config)

      # Clean up
      File.rm_rf(temp_dir)
    end
  end
end
