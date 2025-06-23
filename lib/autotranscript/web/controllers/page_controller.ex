defmodule Autotranscript.Web.PageController do
  use Autotranscript.Web, :controller

  def index(conn, _params) do
    render(conn, :index)
  end
end
