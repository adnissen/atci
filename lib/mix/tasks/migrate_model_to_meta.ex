defmodule Mix.Tasks.MigrateModelToMeta do
  use Mix.Task

  @shortdoc "Migrates model information from transcript files to meta files"
  @moduledoc """
  This task migrates the "model:" line from the beginning of transcript files
  to the corresponding .meta files as "source:" information.

  Usage:
    mix migrate_model_to_meta [watch_directory]

  If no directory is provided, it will use the configured watch directory.
  """

  alias Autotranscript.{ConfigManager, MetaFileHandler}

  def run(args) do
    Mix.Task.run("app.start")

    watch_directory = case args do
      [dir] -> dir
      [] -> ConfigManager.get_config_value("watch_directory")
    end

    if watch_directory == nil or watch_directory == "" do
      Mix.shell().error("No watch directory specified. Please provide a directory path or configure watch_directory.")
      System.halt(1)
    end

    Mix.shell().info("Migrating model information to meta files in: #{watch_directory}")

    # Find all transcript files
    txt_files = Path.wildcard(Path.join(watch_directory, "**/*.txt"))
    
    {migrated_count, skipped_count, error_count} = 
      Enum.reduce(txt_files, {0, 0, 0}, fn txt_path, {migrated, skipped, errors} ->
        case migrate_file(txt_path) do
          :migrated ->
            Mix.shell().info("✓ Migrated: #{Path.relative_to(txt_path, watch_directory)}")
            {migrated + 1, skipped, errors}
          :skipped ->
            {migrated, skipped + 1, errors}
          {:error, reason} ->
            Mix.shell().error("✗ Error migrating #{Path.relative_to(txt_path, watch_directory)}: #{inspect(reason)}")
            {migrated, skipped, errors + 1}
        end
      end)

    Mix.shell().info("\nMigration complete!")
    Mix.shell().info("  Migrated: #{migrated_count} files")
    Mix.shell().info("  Skipped: #{skipped_count} files (no model line found)")
    if error_count > 0 do
      Mix.shell().info("  Errors: #{error_count} files")
    end
  end

  defp migrate_file(txt_path) do
    case File.read(txt_path) do
      {:ok, content} ->
        lines = String.split(content, "\n")
        
        case lines do
          [first_line | rest] when is_binary(first_line) ->
            if String.starts_with?(first_line, "model: ") do
              # Extract model name
              model_name = String.replace_prefix(first_line, "model: ", "") |> String.trim()
              
              # Remove model line from transcript
              new_content = Enum.join(rest, "\n")
              
              case File.write(txt_path, new_content, [:utf8]) do
                :ok ->
                  # Save to meta file
                  meta_path = String.replace_trailing(txt_path, ".txt", ".meta")
                  
                  case MetaFileHandler.update_meta_field(meta_path, "source", model_name) do
                    :ok -> :migrated
                    {:error, reason} -> {:error, {:meta_write_failed, reason}}
                  end
                {:error, reason} ->
                  {:error, {:txt_write_failed, reason}}
              end
            else
              :skipped
            end
          _ ->
            :skipped
        end
      {:error, reason} ->
        {:error, {:read_failed, reason}}
    end
  end
end