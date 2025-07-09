defmodule Autotranscript.MetaFileHandlerTest do
  use ExUnit.Case
  alias Autotranscript.MetaFileHandler

  setup do
    # Create a temporary directory for tests
    temp_dir = Path.join(System.tmp_dir(), "meta_file_handler_test_#{:rand.uniform(10000)}")
    File.mkdir_p!(temp_dir)

    on_exit(fn ->
      File.rm_rf!(temp_dir)
    end)

    {:ok, temp_dir: temp_dir}
  end

  describe "write_meta_file/2" do
    test "writes a new meta file with multiple fields", %{temp_dir: temp_dir} do
      meta_path = Path.join(temp_dir, "test.meta")
      metadata = %{
        "source" => "ggml-base.en",
        "length" => "01:23:45",
        "processed_at" => "2024-01-01"
      }

      assert :ok = MetaFileHandler.write_meta_file(meta_path, metadata)
      assert File.exists?(meta_path)

      {:ok, content} = File.read(meta_path)
      assert String.contains?(content, "source: ggml-base.en")
      assert String.contains?(content, "length: 01:23:45")
      assert String.contains?(content, "processed_at: 2024-01-01")
    end

    test "overwrites an existing meta file", %{temp_dir: temp_dir} do
      meta_path = Path.join(temp_dir, "test.meta")

      # Write initial content
      File.write!(meta_path, "old: content\n")

      metadata = %{"source" => "subtitle file"}
      assert :ok = MetaFileHandler.write_meta_file(meta_path, metadata)

      {:ok, content} = File.read(meta_path)
      assert String.contains?(content, "source: subtitle file")
      refute String.contains?(content, "old: content")
    end
  end

  describe "read_meta_file/1" do
    test "reads a meta file with multiple fields", %{temp_dir: temp_dir} do
      meta_path = Path.join(temp_dir, "test.meta")
      content = """
      source: ggml-base.en
      length: 01:23:45
      processed_at: 2024-01-01
      """
      File.write!(meta_path, content)

      {:ok, metadata} = MetaFileHandler.read_meta_file(meta_path)
      assert metadata["source"] == "ggml-base.en"
      assert metadata["length"] == "01:23:45"
      assert metadata["processed_at"] == "2024-01-01"
    end



    test "handles empty meta file", %{temp_dir: temp_dir} do
      meta_path = Path.join(temp_dir, "test.meta")
      File.write!(meta_path, "")

      {:ok, metadata} = MetaFileHandler.read_meta_file(meta_path)
      assert metadata == %{}
    end

    test "returns error for non-existent file", %{temp_dir: temp_dir} do
      meta_path = Path.join(temp_dir, "non_existent.meta")

      assert {:error, :enoent} = MetaFileHandler.read_meta_file(meta_path)
    end
  end

  describe "update_meta_field/3" do
    test "updates an existing field", %{temp_dir: temp_dir} do
      meta_path = Path.join(temp_dir, "test.meta")
      File.write!(meta_path, "source: old_value\nlength: 01:23:45\n")

      assert :ok = MetaFileHandler.update_meta_field(meta_path, "source", "new_value")

      {:ok, metadata} = MetaFileHandler.read_meta_file(meta_path)
      assert metadata["source"] == "new_value"
      assert metadata["length"] == "01:23:45"
    end

    test "adds a new field to existing file", %{temp_dir: temp_dir} do
      meta_path = Path.join(temp_dir, "test.meta")
      File.write!(meta_path, "length: 01:23:45\n")

      assert :ok = MetaFileHandler.update_meta_field(meta_path, "source", "subtitle file")

      {:ok, metadata} = MetaFileHandler.read_meta_file(meta_path)
      assert metadata["source"] == "subtitle file"
      assert metadata["length"] == "01:23:45"
    end

    test "creates new file if it doesn't exist", %{temp_dir: temp_dir} do
      meta_path = Path.join(temp_dir, "new.meta")

      assert :ok = MetaFileHandler.update_meta_field(meta_path, "source", "ggml-base.en")
      assert File.exists?(meta_path)

      {:ok, metadata} = MetaFileHandler.read_meta_file(meta_path)
      assert metadata["source"] == "ggml-base.en"
    end
  end

  describe "get_meta_field/2" do
    test "gets an existing field", %{temp_dir: temp_dir} do
      meta_path = Path.join(temp_dir, "test.meta")
      File.write!(meta_path, "source: ggml-base.en\nlength: 01:23:45\n")

      assert {:ok, "ggml-base.en"} = MetaFileHandler.get_meta_field(meta_path, "source")
      assert {:ok, "01:23:45"} = MetaFileHandler.get_meta_field(meta_path, "length")
    end

    test "returns not_found for non-existent field", %{temp_dir: temp_dir} do
      meta_path = Path.join(temp_dir, "test.meta")
      File.write!(meta_path, "length: 01:23:45\n")

      assert {:error, :not_found} = MetaFileHandler.get_meta_field(meta_path, "source")
    end

    test "returns error for non-existent file", %{temp_dir: temp_dir} do
      meta_path = Path.join(temp_dir, "non_existent.meta")

      assert {:error, :enoent} = MetaFileHandler.get_meta_field(meta_path, "source")
    end
  end
end
