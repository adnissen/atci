defmodule Autotranscript.Application do
  use Application

  @impl true
  def start(_type, _args) do
    config = Autotranscript.ConfigManager.get_config()
    
    # Store the config in the application environment for the endpoint
    Application.put_env(:autotranscript, :atconfig, config)
    
    children = [
      {Autotranscript.VideoProcessor, [atconfig: config]},
      {Autotranscript.Transcriber, [atconfig: config]},
      {Autotranscript.Web.Endpoint, []}
    ]
    opts = [strategy: :one_for_one, name: Autotranscript.Supervisor]
    Supervisor.start_link(children, opts)
  end
end
