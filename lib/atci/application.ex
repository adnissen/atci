defmodule Atci.Application do
  use Application

  @impl true
  def start(_type, _args) do
    children = [
      {Atci.ConfigManager, []},
      {Atci.VideoInfoCache, []},
      {Atci.VideoProcessor, []},
      {Atci.Transcriber, []},
      {Atci.Web.Endpoint, []}
    ]

    opts = [strategy: :rest_for_one, name: Atci.Supervisor]
    Supervisor.start_link(children, opts)
  end
end
