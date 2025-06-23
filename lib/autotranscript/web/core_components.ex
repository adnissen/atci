defmodule Autotranscript.Web.CoreComponents do
  @moduledoc """
  Provides core UI components.
  """
  use Phoenix.Component

  @doc """
  Renders a page title.

  ## Examples

      <.page_title>Welcome</.page_title>
  """
  slot :inner_block, required: true

  def page_title(assigns) do
    ~H"""
    <title><%= render_slot(@inner_block) %></title>
    """
  end

  @doc """
  Renders a live title that updates automatically.

  ## Examples

      <.app_live_title suffix=" · Phoenix Framework">
        <%= assigns[:page_title] || "Welcome" %>
      </.app_live_title>
  """
  attr :suffix, :string, default: nil
  slot :inner_block, required: true

  def app_live_title(assigns) do
    ~H"""
    <title><%= render_slot(@inner_block) %><%= @suffix %></title>
    """
  end
end
