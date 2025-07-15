defmodule Autotranscript.Web.Plugs.NonlocalPassword do
  @moduledoc """
  A plug that requires a hardcoded password via Basic Auth for non-local requests.
  Local requests (from 127.0.0.1/::1) are allowed without authentication.
  """
  import Plug.Conn
  alias Autotranscript.ConfigManager

  def init(opts), do: opts

  def call(conn, _opts) do
    password = ConfigManager.get_config_value("nonlocal_password")

    if is_nil(password) or password == "" do
      conn
    else
      cond do
        cookie_password_valid?(conn, password) ->
          conn
        true ->
          case get_basic_auth(conn) do
            {:ok, input_password} when input_password == password ->
              conn
              |> put_resp_cookie("nonlocal_password", password, http_only: true, same_site: "Strict")
            _ ->
              if html_request?(conn) do
                conn
                |> put_status(401)
                |> Phoenix.Controller.put_view(Autotranscript.Web.ErrorView)
                |> Phoenix.Controller.render("401.html")
                |> halt()
              else
                conn
                |> put_resp_header("www-authenticate", "Basic realm=\"Restricted\"")
                |> send_resp(401, "Unauthorized")
                |> halt()
              end
          end
      end
    end
  end

  defp local_request?(%Plug.Conn{remote_ip: {127, 0, 0, 1}}), do: true
  defp local_request?(%Plug.Conn{remote_ip: {0, 0, 0, 0, 0, 0, 0, 1}}), do: true
  defp local_request?(_), do: false

  defp get_basic_auth(conn) do
    with ["Basic " <> encoded] <- get_req_header(conn, "authorization"),
         {:ok, decoded} <- Base.decode64(encoded),
         [_, password] <- String.split(decoded, ":", parts: 2) do
      {:ok, password}
    else
      _ -> :error
    end
  end

  defp cookie_password_valid?(conn, password) do
    case fetch_cookies(conn) do
      %{cookies: %{"nonlocal_password" => cookie_pw}} when cookie_pw == password ->
        true
      _ ->
        false
    end
  end

  defp html_request?(conn) do
    get_req_header(conn, "accept")
    |> Enum.any?(fn h -> String.contains?(h, "html") end)
  end
end
