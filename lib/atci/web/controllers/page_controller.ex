defmodule Atci.Web.PageController do
  use Atci.Web, :controller

  def index(conn, _params) do
    render(conn, :index)
  end
end
