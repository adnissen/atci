defmodule Autotranscript.Application do
  use Application

  @impl true
  def start(_type, _args) do
    children = [
      {Autotranscript, []}
    ]

    opts = [strategy: :one_for_one, name: Autotranscript.Supervisor]
    Supervisor.start_link(children, opts)
  end
end
