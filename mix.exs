defmodule Atci.MixProject do
  use Mix.Project

  def project do
    [
      app: :atci,
      version: "0.1.0",
      elixir: "~> 1.18",
      start_permanent: Mix.env() == :prod,
      deps: deps()
    ]
  end

  # Run "mix help compile.app" to learn about applications.
  def application do
    [
      extra_applications: [:logger, :httpoison],
      mod: {Atci.Application, []}
    ]
  end

  # Run "mix help deps" to learn about dependencies.
  defp deps do
    [
      {:phoenix, "~> 1.7"},
      {:phoenix_html, "~> 4.0"},
      {:phoenix_live_view, "~> 0.20"},
      {:plug_cowboy, "~> 2.7"},
      {:gettext, ">= 0.24.0"},
      {:jason, "~> 1.4"},
      {:uuid, "~> 1.1"},
      {:httpoison, "~> 2.0"}
    ]
  end
end
