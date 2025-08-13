defmodule Atci.Web.ErrorView do
  use Atci.Web, :html

  # If you want to customize the layout for error pages, you can specify it here.
  # use Phoenix.Controller, layout: {Atci.Web.Layouts, :root}

  # Renders a template for a given status code (e.g., 404, 500)
  def render("500.html", assigns) do
    ~H"""
    <div class="container">
      <h1>Internal Server Error</h1>
      <p>Sorry, something went wrong on our end. Please try again later.</p>
    </div>
    <style>
      .container {
        max-width: 600px;
        margin: auto;
        background: #fff;
        padding: 2em;
        border-radius: 8px;
        box-shadow: 0 2px 8px rgba(0,0,0,0.05);
        text-align: center;
      }
      h1 {
        color: #d32f2f;
        margin-bottom: 1rem;
      }
      body {
        font-family: sans-serif;
        background: #f9f9f9;
        color: #222;
      }
    </style>
    """
  end

  def render("404.html", assigns) do
    ~H"""
    <div class="container">
      <h1>Page Not Found</h1>
      <p>The page you are looking for does not exist. Please check the URL and try again.</p>
    </div>
    <style>
      .container {
        max-width: 600px;
        margin: auto;
        background: #fff;
        padding: 2em;
        border-radius: 8px;
        box-shadow: 0 2px 8px rgba(0,0,0,0.05);
        text-align: center;
      }
      h1 {
        color: #d32f2f;
        margin-bottom: 1rem;
      }
      body {
        font-family: sans-serif;
        background: #f9f9f9;
        color: #222;
      }
    </style>
    """
  end

  def render("401.html", assigns) do
    ~H"""
    <div class="container">
      <h1>Password?</h1>
      <form id="auth-form" onsubmit="return submitAuth(event)">
        <input type="password" id="password" name="password" placeholder="Password" required style="padding:0.5em; width:200px;" />
        <button type="submit" style="padding:0.5em 1em; margin-left:0.5em;">Login</button>
      </form>
      <p id="error-message" style="color:#d32f2f;"></p>
    </div>
    <script>
      function submitAuth(event) {
        event.preventDefault();
        var password = document.getElementById('password').value;
        var xhr = new XMLHttpRequest();
        xhr.open('GET', '/app', true);
        xhr.setRequestHeader('Authorization', 'Basic ' + btoa('user:' + password));
        xhr.onreadystatechange = function() {
          if (xhr.readyState === 4) {
            if (xhr.status === 200) {
              window.location.href = '/app';
            } else {
              document.getElementById('error-message').textContent = 'Incorrect password.';
            }
          }
        };
        xhr.send();
        return false;
      }
    </script>
    <style>
      .container {
        max-width: 400px;
        margin: 4em auto;
        background: #fff;
        padding: 2em;
        border-radius: 8px;
        box-shadow: 0 2px 8px rgba(0,0,0,0.05);
        text-align: center;
      }
      h1 {
        color: #1976d2;
        margin-bottom: 1rem;
      }
      body {
        font-family: sans-serif;
        background: #f9f9f9;
        color: #222;
      }
      input[type=password] {
        font-size: 1em;
      }
      button {
        font-size: 1em;
        background: #1976d2;
        color: #fff;
        border: none;
        border-radius: 4px;
        cursor: pointer;
      }
      button:hover {
        background: #1565c0;
      }
    </style>
    """
  end

  # Handle JSON error responses
  def render("404.json", _assigns) do
    %{error: "Not found"}
  end

  def render("500.json", _assigns) do
    %{error: "Internal server error"}
  end

  # Fallback for other templates
  def render(template, _assigns) do
    Phoenix.Controller.status_message_from_template(template)
  end
end
