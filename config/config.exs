import Config

# Configuration values are now managed via .atconfig files
# See Autotranscript.ConfigManager for configuration management

# Phoenix configuration
config :autotranscript, Autotranscript.Web.Endpoint,
  url: [host: "localhost"],
  http: [ip: {127, 0, 0, 1}, port: 6200],
  render_errors: [formats: [html: Autotranscript.Web.ErrorView, json: Autotranscript.Web.ErrorView], layout: false],
  pubsub_server: Autotranscript.PubSub,
  live_view: [signing_salt: "autotranscript-salt"],
  secret_key_base: "autotranscript-secret-key-base-for-development-only-change-in-production"

# Logger configuration
config :logger,
  level: :info,
  format: "$time $metadata[$level] $message\n"

# Use Jason for JSON parsing in Phoenix
config :phoenix, :json_library, Jason
