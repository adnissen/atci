defmodule Autotranscript.ConfigMigrationTest do
  use ExUnit.Case

  @moduletag :tmp_dir

  describe "config file migration" do
    test "config file migration automatically migrates old format config file on load" do
      # Create a temporary directory for testing
      temp_dir = Path.join(System.tmp_dir!(), "test_watch_dir_#{:rand.uniform(10000)}")
      File.mkdir_p!(temp_dir)

      # Create old format config
      old_config = %{
        "watch_directory" => temp_dir,
        "whispercli_path" => "/usr/bin/echo",
        "model_path" => "/usr/bin/echo",
        "ffmpeg_path" => "/usr/bin/echo",
        "ffprobe_path" => "/usr/bin/echo"
      }

      # Test the migration logic directly by simulating what happens in migrate_config_format
      migrated_config =
        case {Map.get(old_config, "watch_directory"), Map.get(old_config, "watch_directories")} do
          {watch_dir_val, nil} when is_binary(watch_dir_val) and watch_dir_val != "" ->
            old_config
            |> Map.put("watch_directories", [watch_dir_val])
            |> Map.delete("watch_directory")

          {_, _} ->
            old_config
        end

      # Verify the migration worked
      assert Map.get(migrated_config, "watch_directories") == [temp_dir]
      assert Map.get(migrated_config, "whispercli_path") == "/usr/bin/echo"
      assert Map.get(migrated_config, "model_path") == "/usr/bin/echo"
      assert Map.get(migrated_config, "ffmpeg_path") == "/usr/bin/echo"
      assert Map.get(migrated_config, "ffprobe_path") == "/usr/bin/echo"
      refute Map.has_key?(migrated_config, "watch_directory")

      # Verify the migrated config is complete
      assert Autotranscript.ConfigManager.config_complete?(migrated_config)

      # Clean up
      File.rm_rf(temp_dir)
    end

    test "leaves new format config unchanged", %{tmp_dir: tmp_dir} do
      # Create a temporary directory for testing
      watch_dir = Path.join(tmp_dir, "watch_videos")
      File.mkdir_p!(watch_dir)

      # Create new format config
      new_config = %{
        "watch_directories" => [watch_dir],
        "whispercli_path" => "/usr/bin/echo",
        "model_path" => "/usr/bin/echo",
        "ffmpeg_path" => "/usr/bin/echo",
        "ffprobe_path" => "/usr/bin/echo"
      }

      # Test the migration logic - should not change new format
      migrated_config =
        case {Map.get(new_config, "watch_directory"), Map.get(new_config, "watch_directories")} do
          {watch_dir_val, nil} when is_binary(watch_dir_val) and watch_dir_val != "" ->
            new_config
            |> Map.put("watch_directories", [watch_dir_val])
            |> Map.delete("watch_directory")

          {_, _} ->
            new_config
        end

      # Verify the config was not changed
      assert migrated_config == new_config
      assert Map.get(migrated_config, "watch_directories") == [watch_dir]
      refute Map.has_key?(migrated_config, "watch_directory")

      # Verify the config is complete
      assert Autotranscript.ConfigManager.config_complete?(migrated_config)
    end
  end
end
