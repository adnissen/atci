defmodule AtciTest do
  use ExUnit.Case
  doctest Atci

  test "greets the world" do
    assert Atci.hello() == :world
  end
end
