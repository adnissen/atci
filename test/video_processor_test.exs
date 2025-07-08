defmodule Autotranscript.VideoProcessorTest do
  use ExUnit.Case
  alias Autotranscript.VideoProcessor

  describe "parse_srt_content/1" do
    test "parses simple SRT content correctly" do
      srt_content = """
      1
      00:00:01,000 --> 00:00:03,000
      Hello world

      2
      00:00:04,500 --> 00:00:07,000
      This is a test subtitle
      on multiple lines

      3
      00:00:10,000 --> 00:00:12,500
      Final subtitle
      """

      # Since parse_srt_content is private, we'll test through convert_srt_to_transcript
      temp_srt = Path.join(System.tmp_dir(), "test_subtitle.srt")
      temp_txt = Path.join(System.tmp_dir(), "test_transcript.txt")

      File.write!(temp_srt, srt_content)
      
      assert :ok = VideoProcessor.convert_srt_to_transcript(temp_srt, temp_txt)
      
      {:ok, result} = File.read(temp_txt)
      lines = String.split(result, "\n", trim: true)

      assert List.first(lines) == "model: subtitle file"
      assert Enum.at(lines, 1) == "00:00:01.000 Hello world"
      assert Enum.at(lines, 2) == "00:00:04.500 This is a test subtitle on multiple lines"
      assert Enum.at(lines, 3) == "00:00:10.000 Final subtitle"

      # Cleanup
      File.rm(temp_srt)
      File.rm(temp_txt)
    end

    test "handles empty SRT content" do
      temp_srt = Path.join(System.tmp_dir(), "test_empty.srt")
      temp_txt = Path.join(System.tmp_dir(), "test_empty_transcript.txt")

      File.write!(temp_srt, "")
      
      assert :ok = VideoProcessor.convert_srt_to_transcript(temp_srt, temp_txt)
      
      {:ok, result} = File.read(temp_txt)
      
      assert result == "model: subtitle file"

      # Cleanup
      File.rm(temp_srt)
      File.rm(temp_txt)
    end

    test "handles malformed SRT content gracefully" do
      srt_content = """
      Not a valid subtitle format
      Random text here
      """

      temp_srt = Path.join(System.tmp_dir(), "test_malformed.srt")
      temp_txt = Path.join(System.tmp_dir(), "test_malformed_transcript.txt")

      File.write!(temp_srt, srt_content)
      
      assert :ok = VideoProcessor.convert_srt_to_transcript(temp_srt, temp_txt)
      
      {:ok, result} = File.read(temp_txt)
      
      # Should still have the model line even if no valid subtitles were parsed
      assert result == "model: subtitle file"

      # Cleanup
      File.rm(temp_srt)
      File.rm(temp_txt)
    end
  end
end