use crate::AtciConfig;

pub fn load_config() -> Result<AtciConfig, confy::ConfyError> {
    if let Ok(config_path) = std::env::var("ATCI_CONFIG_PATH") {
        confy::load_path(&config_path)
    } else {
        confy::load("atci", "config")
    }
}

pub fn load_config_or_default() -> AtciConfig {
    load_config().unwrap_or_default()
}

pub fn store_config(config: &AtciConfig) -> Result<(), confy::ConfyError> {
    if let Ok(config_path) = std::env::var("ATCI_CONFIG_PATH") {
        confy::store_path(&config_path, config)
    } else {
        confy::store("atci", "config", config)
    }
}