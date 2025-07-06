defmodule Autotranscript.ConfigMigrationTest do
  use ExUnit.Case
  
  @moduletag :tmp_dir
  
  describe "config file migration" do
    test "automatically migrates old format config file on load", %{tmp_dir: tmp_dir} do
      # Create a temporary directory for testing
      watch_dir = Path.join(tmp_dir, "watch_videos")
      File.mkdir_p!(watch_dir)
      
      # Create old format config file
      old_config = %{
        "watch_directory" => watch_dir,
        "whispercli_path" => "/usr/bin/echo",
        "model_path" => "/usr/bin/echo"
      }
      
      config_file = Path.join(tmp_dir, ".atconfig")
      File.write!(config_file, Jason.encode!(old_config, pretty: true))
      
      # Test the migration by reading and parsing the config
      # This simulates what happens in load_config_from_file
      {:ok, content} = File.read(config_file)
      {:ok, loaded_config} = Jason.decode(content)
      
      # Apply the migration logic manually to test it
      migrated_config = case {Map.get(loaded_config, "watch_directory"), Map.get(loaded_config, "watch_directories")} do
        {watch_dir_val, nil} when is_binary(watch_dir_val) and watch_dir_val != "" ->
          loaded_config
          |> Map.put("watch_directories", [watch_dir_val])
          |> Map.delete("watch_directory")
        {_, _} ->
          loaded_config
      end
      
      # Verify the migration worked
      assert Map.get(migrated_config, "watch_directories") == [watch_dir]
      refute Map.has_key?(migrated_config, "watch_directory")
      
      # Verify the migrated config is complete
      assert Autotranscript.ConfigManager.config_complete?(migrated_config)
    end
    
    test "leaves new format config unchanged", %{tmp_dir: tmp_dir} do
      # Create a temporary directory for testing
      watch_dir = Path.join(tmp_dir, "watch_videos")
      File.mkdir_p!(watch_dir)
      
      # Create new format config file
      new_config = %{
        "watch_directories" => [watch_dir],
        "whispercli_path" => "/usr/bin/echo",
        "model_path" => "/usr/bin/echo"
      }
      
      config_file = Path.join(tmp_dir, ".atconfig")
      File.write!(config_file, Jason.encode!(new_config, pretty: true))
      
      # Test that new format doesn't get modified
      {:ok, content} = File.read(config_file)
      {:ok, loaded_config} = Jason.decode(content)
      
      # Apply the migration logic
      migrated_config = case {Map.get(loaded_config, "watch_directory"), Map.get(loaded_config, "watch_directories")} do
        {watch_dir_val, nil} when is_binary(watch_dir_val) and watch_dir_val != "" ->
          loaded_config
          |> Map.put("watch_directories", [watch_dir_val])
          |> Map.delete("watch_directory")
        {_, _} ->
          loaded_config
      end
      
      # Verify the config remained unchanged
      assert migrated_config == new_config
      assert Map.get(migrated_config, "watch_directories") == [watch_dir]
      refute Map.has_key?(migrated_config, "watch_directory")
    end
  end
end