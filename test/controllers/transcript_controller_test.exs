defmodule Autotranscript.Web.TranscriptControllerTest do
  use Autotranscript.Web.ConnCase
  
  describe "set_watch_directory/2" do
    test "sets watch directory to array format", %{conn: conn} do
      # Create a temporary directory for testing
      temp_dir = Path.join(System.tmp_dir!(), "test_watch_dir_#{:rand.uniform(10000)}")
      File.mkdir_p!(temp_dir)
      
      # Make POST request to set watch directory
      conn = post(conn, "/watch_directory", %{"watch_directory" => temp_dir})
      
      # Verify response
      assert conn.status == 200
      assert Jason.decode!(conn.resp_body) == %{
        "success" => true,
        "message" => "Watch directory updated successfully",
        "watch_directories" => [temp_dir]
      }
      
      # Verify the config was updated correctly
      config = Autotranscript.ConfigManager.get_config()
      assert Map.get(config, "watch_directories") == [temp_dir]
      refute Map.has_key?(config, "watch_directory")
      
      # Clean up
      File.rm_rf(temp_dir)
    end
    
    test "returns error for non-existent directory", %{conn: conn} do
      non_existent_dir = "/this/directory/does/not/exist"
      
      conn = post(conn, "/watch_directory", %{"watch_directory" => non_existent_dir})
      
      assert conn.status == 400
      response = Jason.decode!(conn.resp_body)
      assert response["success"] == false
      assert String.contains?(response["message"], "does not exist")
    end
    
    test "returns error for missing parameter", %{conn: conn} do
      conn = post(conn, "/watch_directory", %{})
      
      assert conn.status == 400
      response = Jason.decode!(conn.resp_body)
      assert response["success"] == false
      assert String.contains?(response["message"], "Missing or invalid")
    end
    
    test "returns error for empty directory parameter", %{conn: conn} do
      conn = post(conn, "/watch_directory", %{"watch_directory" => ""})
      
      assert conn.status == 400
      response = Jason.decode!(conn.resp_body)
      assert response["success"] == false
      assert String.contains?(response["message"], "Missing or invalid")
    end
  end
end