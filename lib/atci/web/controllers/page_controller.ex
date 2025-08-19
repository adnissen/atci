defmodule Atci.Web.PageController do
  use Atci.Web, :controller

  def index(conn, _params) do
    conn |> redirect(to: "/app")
  end
end
