import Config

# Configuration values are now managed via .atciconfig files
# See Atci.ConfigManager for configuration management

# Phoenix configuration
config :atci, Atci.Web.Endpoint,
  url: [host: "localhost"],
  http: [ip: {127, 0, 0, 1}, port: 6200],
  render_errors: [
    formats: [html: Atci.Web.ErrorView, json: Atci.Web.ErrorView],
    layout: false
  ],
  pubsub_server: Atci.PubSub,
  live_view: [signing_salt: "wxcPlQD7lUeWmGva/OdEyAqeEvscEgA7"],
  secret_key_base: "tvVSDLmJ5oN3FZksmgcegX07BXe2P80Ht2J57gsWQ4wnRqtmH/OxcNLfLXHtEBuT"

# Logger configuration
config :logger,
  level: :info,
  format: "$time $metadata[$level] $message\n"

# Use Jason for JSON parsing in Phoenix
config :phoenix, :json_library, Jason
