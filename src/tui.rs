use crate::system_tab::render_system_tab;
use crate::{config, db, files};
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    },
    execute,
    terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, SetTitle, disable_raw_mode, enable_raw_mode,
    },
};
use ratatui::{
    Frame, Terminal,
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
};
use std::{
    env,
    error::Error,
    fs::{self, OpenOptions},
    io,
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

pub struct TableColors {
    pub buffer_bg: Color,
    pub header_bg: Color,
    pub header_fg: Color,
    pub row_fg: Color,
    pub footer_border_color: Color,
    pub selection: Color,
    pub success: Color,
    pub disabled: Color,
    pub info: Color,
    pub error: Color,
    pub text_highlight: Color,
}

impl TableColors {
    fn from_config(cfg: &config::AtciConfig) -> Self {
        Self {
            buffer_bg: parse_hex_color(&cfg.color_buffer_bg)
                .expect("Invalid hex color in config: color_buffer_bg"),
            header_bg: parse_hex_color(&cfg.color_header_bg)
                .expect("Invalid hex color in config: color_header_bg"),
            header_fg: parse_hex_color(&cfg.color_text_primary)
                .expect("Invalid hex color in config: color_text_primary"),
            row_fg: parse_hex_color(&cfg.color_text_primary)
                .expect("Invalid hex color in config: color_text_primary"),
            footer_border_color: parse_hex_color(&cfg.color_border_primary)
                .expect("Invalid hex color in config: color_border_primary"),
            selection: parse_hex_color(&cfg.color_selection)
                .expect("Invalid hex color in config: color_selection"),
            success: parse_hex_color(&cfg.color_success)
                .expect("Invalid hex color in config: color_success"),
            disabled: parse_hex_color(&cfg.color_disabled)
                .expect("Invalid hex color in config: color_disabled"),
            info: parse_hex_color(&cfg.color_info)
                .expect("Invalid hex color in config: color_info"),
            error: parse_hex_color(&cfg.color_error)
                .expect("Invalid hex color in config: color_error"),
            text_highlight: parse_hex_color(&cfg.color_text_highlight)
                .expect("Invalid hex color in config: color_text_highlight"),
        }
    }
}

/// Parse a hex color string (#RRGGBB or #RGB) into a ratatui Color
pub fn parse_hex_color(hex: &str) -> Option<Color> {
    let hex = hex.trim().strip_prefix('#')?;

    let (r, g, b) = if hex.len() == 6 {
        // #RRGGBB format
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        (r, g, b)
    } else if hex.len() == 3 {
        // #RGB format - expand to RRGGBB
        let r = u8::from_str_radix(&hex[0..1], 16).ok()?;
        let g = u8::from_str_radix(&hex[1..2], 16).ok()?;
        let b = u8::from_str_radix(&hex[2..3], 16).ok()?;
        (r * 17, g * 17, b * 17) // 0xF -> 0xFF
    } else {
        return None;
    };

    Some(Color::Rgb(r, g, b))
}

#[derive(Clone, Copy, PartialEq)]
pub enum TabState {
    System,
}

#[derive(Clone, Copy, PartialEq)]
pub enum SystemSection {
    Config,
    WatchDirectories,
}

#[derive(Clone, Copy, PartialEq)]
pub enum SetupWizardScreen {
    Welcome,
    FFmpeg,
    FFprobe,
    WhisperCli,
    Model,
    WatchDirectories,
    Password,
}

#[derive(Clone)]
pub struct ToolOption {
    pub display_text: String,
    pub action: ToolAction,
}

#[derive(Clone)]
pub enum ToolAction {
    UseDownloaded(String),
    UseSystem(String),
    Download,
    CustomPath,
}

pub struct App {
    pub colors: TableColors,
    pub current_tab: TabState,
    pub system_services: Vec<SystemService>,
    pub last_system_refresh: Instant,
    pub config_data: config::AtciConfig,
    pub config_selected_field: usize,
    pub config_editing_mode: bool,
    pub config_input_buffer: String,
    pub system_section: SystemSection,
    pub queue_items: Vec<String>,
    pub currently_processing: Option<String>,
    pub currently_processing_age: u64,
    pub watch_directories_selected_index: usize,
    pub show_directory_picker: bool,
    pub directory_picker: Option<ratatui_explorer::FileExplorer>,
    pub show_setup_wizard: bool,
    pub setup_wizard_screen: SetupWizardScreen,
    pub setup_wizard_options: Vec<ToolOption>,
    pub setup_wizard_selected_index: usize,
    pub setup_wizard_input_buffer: String,
    pub setup_wizard_input_mode: bool,
    pub setup_wizard_watch_dirs: Vec<String>,
}

#[derive(Clone)]
pub struct SystemService {
    pub name: String,
    pub status: ServiceStatus,
    pub pids: Vec<u32>,
}

#[derive(Clone, PartialEq)]
pub enum ServiceStatus {
    Active,
    Stopped,
}

impl Default for App {
    fn default() -> App {
        let config_data = config::load_config_or_default();
        App {
            colors: TableColors::from_config(&config_data),
            current_tab: TabState::System,
            system_services: Vec::new(),
            last_system_refresh: Instant::now(),
            config_data,
            config_selected_field: 0,
            config_editing_mode: false,
            config_input_buffer: String::new(),
            system_section: SystemSection::Config,
            queue_items: Vec::new(),
            currently_processing: None,
            currently_processing_age: 0,
            watch_directories_selected_index: 0,
            show_directory_picker: false,
            directory_picker: None,
            show_setup_wizard: false,
            setup_wizard_screen: SetupWizardScreen::Welcome,
            setup_wizard_options: Vec::new(),
            setup_wizard_selected_index: 0,
            setup_wizard_input_buffer: String::new(),
            setup_wizard_input_mode: false,
            setup_wizard_watch_dirs: Vec::new(),
        }
    }
}

impl App {
    // Create app for the main TUI (after setup is complete)
    fn new_after_setup() -> Result<App, Box<dyn Error>> {
        let config_data = config::load_config_or_default();
        let mut app = App {
            colors: TableColors::from_config(&config_data),
            current_tab: TabState::System,
            system_services: Vec::new(),
            last_system_refresh: Instant::now(),
            config_data,
            config_selected_field: 0,
            config_editing_mode: false,
            config_input_buffer: String::new(),
            system_section: SystemSection::Config,
            queue_items: Vec::new(),
            currently_processing: None,
            currently_processing_age: 0,
            watch_directories_selected_index: 0,
            show_directory_picker: false,
            directory_picker: None,
            show_setup_wizard: false, // Setup already complete
            setup_wizard_screen: SetupWizardScreen::Welcome,
            setup_wizard_options: Vec::new(),
            setup_wizard_selected_index: 0,
            setup_wizard_input_buffer: String::new(),
            setup_wizard_input_mode: false,
            setup_wizard_watch_dirs: Vec::new(),
        };

        // Initialize system services
        app.refresh_system_services();

        // Initialize queue
        app.refresh_queue();

        Ok(app)
    }

    // Create app for the setup wizard only
    fn new_for_wizard() -> Result<App, Box<dyn Error>> {
        let config_data = config::load_config_or_default();
        let mut app = App {
            colors: TableColors::from_config(&config_data),
            current_tab: TabState::System,
            system_services: Vec::new(),
            last_system_refresh: Instant::now(),
            config_data,
            config_selected_field: 0,
            config_editing_mode: false,
            config_input_buffer: String::new(),
            system_section: SystemSection::Config,
            queue_items: Vec::new(),
            currently_processing: None,
            currently_processing_age: 0,
            watch_directories_selected_index: 0,
            show_directory_picker: false,
            directory_picker: None,
            show_setup_wizard: true, // Always show wizard
            setup_wizard_screen: SetupWizardScreen::Welcome,
            setup_wizard_options: Vec::new(),
            setup_wizard_selected_index: 0,
            setup_wizard_input_buffer: String::new(),
            setup_wizard_input_mode: false,
            setup_wizard_watch_dirs: Vec::new(),
        };

        // Start wizard, which will skip Welcome if config exists
        app.start_wizard();

        Ok(app)
    }

    pub fn config_next_field(&mut self) {
        let total_fields = self.get_config_field_count();
        if self.config_selected_field < total_fields - 1 {
            self.config_selected_field += 1;
        }
    }

    pub fn config_previous_field(&mut self) {
        if self.config_selected_field > 0 {
            self.config_selected_field -= 1;
        }
    }

    pub fn get_config_field_count(&self) -> usize {
        21 // Total number of config fields (excluding watch_directories)
    }

    pub fn get_config_field_names(&self) -> Vec<&'static str> {
        vec![
            "ffmpeg_path",
            "ffprobe_path",
            "model_name",
            "whispercli_path",
            "password",
            "processing_success_command",
            "processing_failure_command",
            "allow_whisper",
            "allow_subtitles",
            "stream_chunk_size",
            "hostname",
            "color_buffer_bg",
            "color_header_bg",
            "color_text_primary",
            "color_border_primary",
            "color_selection",
            "color_success",
            "color_disabled",
            "color_info",
            "color_error",
            "color_text_highlight",
        ]
    }

    pub fn get_config_field_value(&self, field_index: usize) -> String {
        match field_index {
            0 => self.config_data.ffmpeg_path.clone(),
            1 => self.config_data.ffprobe_path.clone(),
            2 => self.config_data.model_name.clone(),
            3 => self.config_data.whispercli_path.clone(),
            4 => self.config_data.password.clone().unwrap_or_default(),
            5 => self.config_data.processing_success_command.clone(),
            6 => self.config_data.processing_failure_command.clone(),
            7 => self.config_data.allow_whisper.to_string(),
            8 => self.config_data.allow_subtitles.to_string(),
            9 => self.config_data.stream_chunk_size.to_string(),
            10 => self.config_data.hostname.clone(),
            11 => self.config_data.color_buffer_bg.clone(),
            12 => self.config_data.color_header_bg.clone(),
            13 => self.config_data.color_text_primary.clone(),
            14 => self.config_data.color_border_primary.clone(),
            15 => self.config_data.color_selection.clone(),
            16 => self.config_data.color_success.clone(),
            17 => self.config_data.color_disabled.clone(),
            18 => self.config_data.color_info.clone(),
            19 => self.config_data.color_error.clone(),
            20 => self.config_data.color_text_highlight.clone(),
            _ => String::new(),
        }
    }

    pub fn is_selected_field_boolean(&self) -> bool {
        let field_names = self.get_config_field_names();
        if self.config_selected_field < field_names.len() {
            let field_name = field_names[self.config_selected_field];
            matches!(field_name, "allow_whisper" | "allow_subtitles")
        } else {
            false
        }
    }

    pub fn toggle_boolean_field(&mut self) -> Result<(), String> {
        let field_names = self.get_config_field_names();
        if self.config_selected_field < field_names.len() {
            let field_name = field_names[self.config_selected_field];
            match field_name {
                "allow_whisper" => {
                    self.config_data.allow_whisper = !self.config_data.allow_whisper;
                    self.save_config()?;
                }
                "allow_subtitles" => {
                    self.config_data.allow_subtitles = !self.config_data.allow_subtitles;
                    self.save_config()?;
                }
                _ => return Err(format!("Field {} is not a boolean field", field_name)),
            }
        }
        Ok(())
    }

    pub fn start_config_editing(&mut self) {
        self.config_editing_mode = true;
        self.config_input_buffer = self.get_config_field_value(self.config_selected_field);
    }

    pub fn stop_config_editing(&mut self) {
        self.config_editing_mode = false;
        self.config_input_buffer.clear();
    }

    pub fn cancel_config_edit(&mut self) {
        // Simply stop editing without saving changes
        self.stop_config_editing();
    }

    pub fn apply_config_edit(&mut self) -> Result<(), String> {
        let field_names = self.get_config_field_names();
        if self.config_selected_field < field_names.len() {
            let field_name = field_names[self.config_selected_field];
            config::set_config_field(&mut self.config_data, field_name, &self.config_input_buffer)?;
            // Automatically save config after editing
            self.save_config()?;
            // Automatically reload config from disk to ensure consistency
            self.reload_config();
        }
        self.stop_config_editing();
        Ok(())
    }

    pub fn save_config(&mut self) -> Result<(), String> {
        config::store_config(&self.config_data).map_err(|e| format!("Failed to save config: {}", e))
    }

    pub fn reload_config(&mut self) {
        self.config_data = config::load_config_or_default();
        // Reload colors from the updated config
        self.colors = TableColors::from_config(&self.config_data);
    }

    pub fn add_char_to_config(&mut self, c: char) {
        if self.config_editing_mode {
            self.config_input_buffer.push(c);
        }
    }

    pub fn remove_char_from_config(&mut self) {
        if self.config_editing_mode {
            self.config_input_buffer.pop();
        }
    }

    pub fn refresh_queue(&mut self) {
        use crate::queue::{get_queue, get_queue_status};

        if let Ok(queue) = get_queue(None) {
            self.queue_items = queue;
        }

        if let Ok((path, age)) = get_queue_status(None) {
            self.currently_processing = path;
            self.currently_processing_age = age;
        }
    }

    pub fn open_directory_picker(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Start from home directory or current directory
        let start_path = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
        let mut explorer = ratatui_explorer::FileExplorer::with_theme(
            ratatui_explorer::Theme::default()
                .add_default_title()
                .with_style(Style::default().bg(self.colors.buffer_bg)),
        )?;
        explorer.set_cwd(&start_path)?;
        self.directory_picker = Some(explorer);
        self.show_directory_picker = true;
        Ok(())
    }

    pub fn close_directory_picker(&mut self) {
        self.show_directory_picker = false;
        self.directory_picker = None;
    }

    pub fn select_directory_from_picker(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(explorer) = &self.directory_picker {
            let current = explorer.current();
            let path = current.path();
            let path_str = path.to_string_lossy().to_string();

            // Add the directory if it's not already in the list
            if !self.config_data.watch_directories.contains(&path_str) {
                self.config_data.watch_directories.push(path_str);
                self.save_config()?;
            }
        }
        self.close_directory_picker();
        Ok(())
    }

    // Setup Wizard Methods

    pub fn check_if_setup_needed(&self) -> bool {
        self.config_data.ffmpeg_path.is_empty()
            || self.config_data.ffprobe_path.is_empty()
            || self.config_data.whispercli_path.is_empty()
            || self.config_data.model_name.is_empty()
            || self.config_data.watch_directories.is_empty()
    }

    pub fn start_wizard(&mut self) {
        self.show_setup_wizard = true;
        self.setup_wizard_screen = SetupWizardScreen::Welcome;
        self.setup_wizard_selected_index = 0;
        self.setup_wizard_input_buffer.clear();
        self.setup_wizard_input_mode = false;
        self.setup_wizard_watch_dirs.clear();
        self.setup_wizard_options.clear();

        // Skip welcome screen if any properties are already configured
        if self.should_skip_wizard_screen(&SetupWizardScreen::Welcome) {
            self.next_wizard_screen();
        }
    }

    pub fn load_tool_options(&mut self, tool: &str) {
        use crate::tools_manager;

        self.setup_wizard_options.clear();

        let tools = tools_manager::list_tools();
        if let Some(tool_info) = tools.iter().find(|t| t.name == tool) {
            // Add downloaded version if available
            if tool_info.downloaded {
                self.setup_wizard_options.push(ToolOption {
                    display_text: format!(
                        "Use downloaded {} ({})",
                        tool, tool_info.downloaded_path
                    ),
                    action: ToolAction::UseDownloaded(tool_info.downloaded_path.clone()),
                });
            }

            // Add system version if available
            if tool_info.system_available
                && let Some(system_path) = &tool_info.system_path
            {
                self.setup_wizard_options.push(ToolOption {
                    display_text: format!("Use system {} ({})", tool, system_path),
                    action: ToolAction::UseSystem(system_path.clone()),
                });
            }

            // Always offer download option
            self.setup_wizard_options.push(ToolOption {
                display_text: format!("Download and use {}", tool),
                action: ToolAction::Download,
            });

            // Always offer custom path option
            self.setup_wizard_options.push(ToolOption {
                display_text: "Enter custom path".to_string(),
                action: ToolAction::CustomPath,
            });
        }

        self.setup_wizard_selected_index = 0;
    }

    pub fn load_model_options(&mut self) {
        use crate::model_manager;

        self.setup_wizard_options.clear();

        let models = model_manager::list_models();

        // Add downloaded models first
        for model in models.iter().filter(|m| m.downloaded) {
            let status = if model.configured {
                " (currently configured)"
            } else {
                ""
            };
            self.setup_wizard_options.push(ToolOption {
                display_text: format!("Use downloaded {} ({}){}", model.name, model.path, status),
                action: ToolAction::UseDownloaded(model.name.clone()),
            });
        }

        // Add available models for download
        for model in models.iter().filter(|m| !m.downloaded) {
            self.setup_wizard_options.push(ToolOption {
                display_text: format!("Download and use {}", model.name),
                action: ToolAction::Download,
            });
        }

        // Add custom path option
        self.setup_wizard_options.push(ToolOption {
            display_text: "Enter custom model file path".to_string(),
            action: ToolAction::CustomPath,
        });

        self.setup_wizard_selected_index = 0;
    }

    pub fn apply_tool_selection(&mut self, field: &str, action: &ToolAction) -> Result<(), String> {
        match action {
            ToolAction::UseDownloaded(path) | ToolAction::UseSystem(path) => {
                config::set_config_field(&mut self.config_data, field, path)?;
                self.save_config()?;
                Ok(())
            }
            ToolAction::Download => {
                // Extract tool name from field (e.g., "ffmpeg_path" -> "ffmpeg", "whispercli_path" -> "whisper-cli")
                let tool_name = match field {
                    "whispercli_path" => "whisper-cli",
                    _ => field.strip_suffix("_path").unwrap_or(field),
                };

                // Exit TUI temporarily for download
                if let Err(e) = download_tool_with_tui_pause(tool_name) {
                    return Err(format!("Failed to download {}: {}", tool_name, e));
                }

                // Get the downloaded path and save to config
                let downloaded_path = crate::tools_manager::get_downloaded_path(tool_name);
                config::set_config_field(&mut self.config_data, field, &downloaded_path)?;
                self.save_config()?;
                Ok(())
            }
            ToolAction::CustomPath => {
                // Enter input mode for custom path
                self.setup_wizard_input_mode = true;
                self.setup_wizard_input_buffer.clear();
                Ok(())
            }
        }
    }

    pub fn apply_model_selection(&mut self, action: &ToolAction) -> Result<(), String> {
        match action {
            ToolAction::UseDownloaded(model_name) => {
                config::set_config_field(&mut self.config_data, "model_name", model_name)?;
                self.save_config()?;
                Ok(())
            }
            ToolAction::Download => {
                // Get the model name from the selected option
                if let Some(option) = self
                    .setup_wizard_options
                    .get(self.setup_wizard_selected_index)
                {
                    if let Some(model_name) = option.display_text.strip_prefix("Download and use ")
                    {
                        // Exit TUI temporarily for download
                        if let Err(e) = download_model_with_tui_pause(model_name) {
                            return Err(format!("Failed to download model {}: {}", model_name, e));
                        }

                        config::set_config_field(&mut self.config_data, "model_name", model_name)?;
                        self.save_config()?;
                        Ok(())
                    } else {
                        Err("Could not determine model name".to_string())
                    }
                } else {
                    Err("No option selected".to_string())
                }
            }
            ToolAction::CustomPath => {
                // Enter input mode for custom path
                self.setup_wizard_input_mode = true;
                self.setup_wizard_input_buffer.clear();
                Ok(())
            }
            _ => Err("Invalid action for model selection".to_string()),
        }
    }

    fn should_skip_wizard_screen(&self, screen: &SetupWizardScreen) -> bool {
        use SetupWizardScreen::*;
        match screen {
            Welcome => {
                // Skip welcome if ANY property is already configured
                !self.config_data.ffmpeg_path.is_empty()
                    || !self.config_data.ffprobe_path.is_empty()
                    || !self.config_data.whispercli_path.is_empty()
                    || !self.config_data.model_name.is_empty()
                    || !self.config_data.watch_directories.is_empty()
            }
            FFmpeg => !self.config_data.ffmpeg_path.is_empty(),
            FFprobe => !self.config_data.ffprobe_path.is_empty(),
            WhisperCli => !self.config_data.whispercli_path.is_empty(),
            Model => !self.config_data.model_name.is_empty(),
            WatchDirectories => !self.config_data.watch_directories.is_empty(),
            Password => false, // Always show password screen (it's optional anyway)
        }
    }

    fn get_wizard_step_info(&self) -> (usize, usize) {
        use SetupWizardScreen::*;

        let all_screens = [
            FFmpeg,
            FFprobe,
            WhisperCli,
            Model,
            WatchDirectories,
            Password,
        ];
        let visible_screens: Vec<_> = all_screens
            .iter()
            .filter(|s| !self.should_skip_wizard_screen(s))
            .collect();

        let total_steps = visible_screens.len();
        let current_step = visible_screens
            .iter()
            .position(|&&s| s == self.setup_wizard_screen)
            .map(|p| p + 1)
            .unwrap_or(0);

        (current_step, total_steps)
    }

    pub fn next_wizard_screen(&mut self) {
        use SetupWizardScreen::*;

        self.setup_wizard_screen = match self.setup_wizard_screen {
            Welcome => FFmpeg,
            FFmpeg => FFprobe,
            FFprobe => WhisperCli,
            WhisperCli => Model,
            Model => WatchDirectories,
            WatchDirectories => Password,
            Password => Password, // Stay at password (will be handled separately)
        };

        // Skip this screen if it's already configured
        if self.should_skip_wizard_screen(&self.setup_wizard_screen) {
            // Recursively move to next screen
            self.next_wizard_screen();
            return;
        }

        // Load options for the new screen
        match self.setup_wizard_screen {
            FFmpeg => self.load_tool_options("ffmpeg"),
            FFprobe => self.load_tool_options("ffprobe"),
            WhisperCli => self.load_tool_options("whisper-cli"),
            Model => self.load_model_options(),
            WatchDirectories => {
                // Initialize directory picker if not already open
                if self.directory_picker.is_none() {
                    let _ = self.open_directory_picker();
                }
            }
            _ => {}
        }
    }

    pub fn previous_wizard_screen(&mut self) {
        use SetupWizardScreen::*;

        self.setup_wizard_screen = match self.setup_wizard_screen {
            Welcome => Welcome, // Can't go back from welcome
            FFmpeg => Welcome,
            FFprobe => FFmpeg,
            WhisperCli => FFprobe,
            Model => WhisperCli,
            WatchDirectories => Model,
            Password => WatchDirectories,
        };

        // Skip this screen if it's already configured
        if self.should_skip_wizard_screen(&self.setup_wizard_screen) {
            // Recursively move to previous screen
            self.previous_wizard_screen();
            return;
        }

        // Load options for the new screen
        match self.setup_wizard_screen {
            FFmpeg => self.load_tool_options("ffmpeg"),
            FFprobe => self.load_tool_options("ffprobe"),
            WhisperCli => self.load_tool_options("whisper-cli"),
            Model => self.load_model_options(),
            WatchDirectories => {
                // Initialize directory picker if not already open
                if self.directory_picker.is_none() {
                    let _ = self.open_directory_picker();
                }
            }
            _ => {}
        }
    }

    pub fn complete_wizard(&mut self) -> Result<(), String> {
        // Save any pending watch directories
        if !self.setup_wizard_watch_dirs.is_empty() {
            for dir in &self.setup_wizard_watch_dirs {
                if !self.config_data.watch_directories.contains(dir) {
                    self.config_data.watch_directories.push(dir.clone());
                }
            }
            self.save_config().map_err(|e| e.to_string())?;
        }

        // Verify all required fields are set
        if self.check_if_setup_needed() {
            return Err("Configuration is incomplete".to_string());
        }

        // Hide wizard
        self.show_setup_wizard = false;
        self.setup_wizard_watch_dirs.clear();
        self.setup_wizard_options.clear();

        Ok(())
    }
}

/// Temporarily exits TUI mode, downloads a tool, and waits for user to press Enter
fn download_tool_with_tui_pause(tool_name: &str) -> Result<(), Box<dyn Error>> {
    // Exit TUI mode
    disable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, LeaveAlternateScreen, DisableMouseCapture)?;

    // Clear screen and show download
    println!("\n\n");
    println!("Downloading {}...", tool_name);
    println!();

    // Perform the actual download
    let result = crate::tools_manager::download_tool(tool_name);

    // Wait for user to acknowledge
    println!();
    if result.is_ok() {
        println!("Download completed successfully!");
    } else {
        println!("Download failed: {:?}", result.as_ref().err());
    }
    println!();
    println!("Press Enter to continue...");

    // Wait for Enter key
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    // Clear the screen before re-entering TUI
    print!("\x1B[2J\x1B[1;1H");
    use std::io::Write;
    stdout.flush()?;

    // Re-enter TUI mode
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    // Return the result
    result.map(|_| ())
}

/// Temporarily exits TUI mode, downloads a model, and waits for user to press Enter
fn download_model_with_tui_pause(model_name: &str) -> Result<(), Box<dyn Error>> {
    // Exit TUI mode
    disable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, LeaveAlternateScreen, DisableMouseCapture)?;

    // Clear screen and show download
    println!("\n\n");
    println!("Downloading model {}...", model_name);
    println!();

    // Perform the actual download
    let result = crate::model_manager::download_model(model_name);

    // Wait for user to acknowledge
    println!();
    if result.is_ok() {
        println!("Download completed successfully!");
    } else {
        println!("Download failed: {:?}", result.as_ref().err());
    }
    println!();
    println!("Press Enter to continue...");

    // Wait for Enter key
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    // Clear the screen before re-entering TUI
    print!("\x1B[2J\x1B[1;1H");
    use std::io::Write;
    stdout.flush()?;

    // Re-enter TUI mode
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    // Return the result
    result.map(|_| ())
}

pub fn start_web_server_as_child() -> Result<std::process::Child, Box<dyn Error>> {
    let current_exe = env::current_exe()?;
    let home_dir = dirs::home_dir().ok_or("Could not find home directory")?;
    let log_path = home_dir.join(".atci").join("web.log");

    // Ensure .atci directory exists
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Open log file for appending
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;

    // Clone file descriptors for stdout and stderr
    let stdout_file = log_file.try_clone()?;
    let stderr_file = log_file;

    // Spawn a new atci web process as a child process
    let child = Command::new(&current_exe)
        .arg("web")
        .arg("all")
        .stdin(Stdio::null())
        .stdout(stdout_file)
        .stderr(stderr_file)
        .spawn()?;

    println!(
        "Started web server as child process (PID: {}, logs: ~/.atci/web.log)",
        child.id()
    );

    Ok(child)
}

pub fn run() -> Result<(), Box<dyn Error>> {
    // Check if setup is needed BEFORE creating the runtime
    let config_data = config::load_config_or_default();
    let needs_setup = config_data.ffmpeg_path.is_empty()
        || config_data.ffprobe_path.is_empty()
        || config_data.whispercli_path.is_empty()
        || config_data.model_name.is_empty()
        || config_data.watch_directories.is_empty();

    // If setup is needed, run the setup wizard first (OUTSIDE of any async context)
    if needs_setup {
        match run_setup_wizard() {
            Ok(should_continue) => {
                if !should_continue {
                    // User quit the wizard, exit program
                    return Ok(());
                }
                // Wizard completed successfully, start web server (drop Child to run independently)
                match start_web_server_as_child() {
                    Ok(_child) => {
                        // Child dropped here, runs independently in background
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to start web server: {}", e);
                    }
                }
                // Update video info cache now that watch directories are configured
                if let Err(e) = files::get_and_save_video_info_from_disk() {
                    eprintln!("Warning: Failed to update video info cache: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Setup wizard error: {:?}", e);
                return Err(e);
            }
        }
    }

    // Now create the runtime for the main TUI
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        // Note: We don't start the watcher directly here anymore.
        // The web server (started by main.rs) will handle starting the watcher if needed.

        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(
            stdout,
            SetTitle("atci"),
            EnterAlternateScreen,
            EnableMouseCapture
        )?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let mut app = App::new_after_setup()?;
        let res = run_app(&mut terminal, &mut app);

        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        if let Err(err) = res {
            println!("{:?}", err)
        }

        Ok(())
    })
}

fn run_setup_wizard() -> Result<bool, Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        SetTitle("atci - Setup Wizard"),
        EnterAlternateScreen,
        EnableMouseCapture
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new_for_wizard()?;
    let should_continue = run_wizard_app(&mut terminal, &mut app)?;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(should_continue)
}

fn run_wizard_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<bool, Box<dyn Error>> {
    loop {
        terminal.draw(|f| {
            // Render the setup wizard (full screen)
            render_setup_wizard_modal(f, app);
        })?;

        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
        {
            if let Some(should_quit) = handle_setup_wizard_input(app, key)?
                && should_quit
            {
                // User pressed Ctrl+C or Esc to quit - don't continue to main TUI
                return Ok(false);
            }

            // Check if wizard is complete (completed successfully)
            if !app.show_setup_wizard {
                // Wizard completed successfully - continue to main TUI
                return Ok(true);
            }
        }
    }
}

fn handle_key_event(
    app: &mut App,
    key: crossterm::event::KeyEvent,
) -> Result<Option<bool>, Box<dyn Error>> {
    // Filter out key release events to prevent duplicate input on Windows
    if key.kind == KeyEventKind::Release {
        return Ok(None);
    }

    // Handle directory picker modal
    if app.show_directory_picker {
        match key.code {
            KeyCode::Esc => app.close_directory_picker(),
            KeyCode::Char('n') => {
                // 'n' key selects the directory and adds it to config
                if let Err(e) = app.select_directory_from_picker() {
                    eprintln!("Failed to select directory: {}", e);
                    app.close_directory_picker();
                }
            }
            KeyCode::Enter => {
                // Enter navigates into the selected directory
                if let Some(explorer) = &mut app.directory_picker {
                    let event = Event::Key(key);
                    let _ = explorer.handle(&event);
                }
            }
            _ => {
                // Pass other keys to the file explorer
                // Convert KeyEvent to Event for the file explorer
                if let Some(explorer) = &mut app.directory_picker {
                    let event = Event::Key(key);
                    let _ = explorer.handle(&event);
                }
            }
        }
        return Ok(None);
    }

    // Handle config editing mode
    if app.current_tab == TabState::System && app.config_editing_mode {
        match key.code {
            KeyCode::Esc => app.cancel_config_edit(),
            KeyCode::Enter => {
                if let Err(e) = app.apply_config_edit() {
                    eprintln!("Failed to apply config edit: {}", e);
                }
            }
            KeyCode::Backspace => app.remove_char_from_config(),
            KeyCode::Char(c) => app.add_char_to_config(c),
            _ => {}
        }
        return Ok(None);
    }

    // Handle normal mode key events
    match key.code {
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            return Ok(Some(true));
        } // Signal to quit
        KeyCode::Char('n') => {
            if app.current_tab == TabState::System
                && app.system_section == SystemSection::WatchDirectories
                && !app.config_editing_mode
            {
                // Open directory picker
                if let Err(e) = app.open_directory_picker() {
                    eprintln!("Failed to open directory picker: {}", e);
                }
            }
        }
        KeyCode::Char('o') => {
            if app.current_tab == TabState::System && !app.config_editing_mode {
                // Open web server in browser (unless editing config)
                if let Err(e) = app.open_web_server_in_browser() {
                    eprintln!("Failed to open web server: {}", e);
                }
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.current_tab == TabState::System {
                app.system_next();
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.current_tab == TabState::System {
                app.system_previous();
            }
        }
        KeyCode::Char('d') => {
            if app.current_tab == TabState::System
                && app.system_section == SystemSection::WatchDirectories
                && !app.config_editing_mode
            {
                // Remove selected watch directory
                if !app.config_data.watch_directories.is_empty()
                    && app.watch_directories_selected_index
                        < app.config_data.watch_directories.len()
                {
                    app.config_data
                        .watch_directories
                        .remove(app.watch_directories_selected_index);
                    // Adjust selected index if needed
                    if app.watch_directories_selected_index
                        >= app.config_data.watch_directories.len()
                        && app.watch_directories_selected_index > 0
                    {
                        app.watch_directories_selected_index -= 1;
                    }
                    // Save config after removing
                    if let Err(e) = app.save_config() {
                        eprintln!(
                            "Failed to save config after removing watch directory: {}",
                            e
                        );
                    }
                }
            }
        }
        KeyCode::Enter => {
            if app.current_tab == TabState::System {
                // Only handle config editing, services are not selectable
                if app.system_section == SystemSection::Config {
                    // For boolean fields, toggle the value instead of entering edit mode
                    if app.is_selected_field_boolean() {
                        if let Err(e) = app.toggle_boolean_field() {
                            eprintln!("Failed to toggle boolean field: {}", e);
                        }
                    } else {
                        app.start_config_editing();
                    }
                }
            }
        }
        KeyCode::Char('S') => {
            if app.current_tab == TabState::System && key.modifiers.contains(KeyModifiers::SHIFT) {
                // Save config
                if let Err(e) = app.save_config() {
                    eprintln!("Failed to save config: {}", e);
                }
            }
        }
        KeyCode::Char('R') => {
            if app.current_tab == TabState::System && key.modifiers.contains(KeyModifiers::SHIFT) {
                // Reload config
                app.reload_config();
            }
        }
        _ => {}
    }

    Ok(None)
}

fn handle_setup_wizard_input(
    app: &mut App,
    key: crossterm::event::KeyEvent,
) -> Result<Option<bool>, Box<dyn Error>> {
    use SetupWizardScreen::*;

    // Allow Ctrl+C to exit at any time during setup wizard
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return Ok(Some(true));
    }

    match app.setup_wizard_screen {
        Welcome => {
            // On welcome screen, Enter advances, Esc quits
            match key.code {
                KeyCode::Enter => {
                    app.next_wizard_screen();
                }
                KeyCode::Esc => {
                    // Quit wizard
                    return Ok(Some(true));
                }
                _ => {}
            }
        }
        FFmpeg | FFprobe | WhisperCli => {
            // Tool selection screens
            if app.setup_wizard_input_mode {
                // In input mode for custom path
                match key.code {
                    KeyCode::Esc => {
                        app.setup_wizard_input_mode = false;
                        app.setup_wizard_input_buffer.clear();
                    }
                    KeyCode::Enter => {
                        // Apply custom path
                        let field = match app.setup_wizard_screen {
                            FFmpeg => "ffmpeg_path",
                            FFprobe => "ffprobe_path",
                            WhisperCli => "whispercli_path",
                            _ => "",
                        };
                        match config::set_config_field(
                            &mut app.config_data,
                            field,
                            &app.setup_wizard_input_buffer,
                        ) {
                            Ok(()) => {
                                if let Err(e) = app.save_config() {
                                    eprintln!("Failed to save config: {}", e);
                                } else {
                                    app.setup_wizard_input_mode = false;
                                    app.setup_wizard_input_buffer.clear();
                                    app.next_wizard_screen();
                                }
                            }
                            Err(e) => {
                                eprintln!("Invalid path: {}", e);
                            }
                        }
                    }
                    KeyCode::Backspace => {
                        app.setup_wizard_input_buffer.pop();
                    }
                    KeyCode::Char(c) => {
                        app.setup_wizard_input_buffer.push(c);
                    }
                    _ => {}
                }
            } else {
                // In selection mode
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        if app.setup_wizard_selected_index > 0 {
                            app.setup_wizard_selected_index -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if app.setup_wizard_selected_index
                            < app.setup_wizard_options.len().saturating_sub(1)
                        {
                            app.setup_wizard_selected_index += 1;
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(option) = app
                            .setup_wizard_options
                            .get(app.setup_wizard_selected_index)
                        {
                            let field = match app.setup_wizard_screen {
                                FFmpeg => "ffmpeg_path",
                                FFprobe => "ffprobe_path",
                                WhisperCli => "whispercli_path",
                                _ => "",
                            };
                            let action = option.action.clone();
                            match app.apply_tool_selection(field, &action) {
                                Ok(()) => {
                                    if !app.setup_wizard_input_mode {
                                        // Only advance if not entering input mode
                                        app.next_wizard_screen();
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Error: {}", e);
                                }
                            }
                        }
                    }
                    KeyCode::Esc => {
                        if app.setup_wizard_screen != Welcome {
                            app.previous_wizard_screen();
                        }
                    }
                    _ => {}
                }
            }
        }
        Model => {
            // Model selection screen
            if app.setup_wizard_input_mode {
                // In input mode for custom model path
                match key.code {
                    KeyCode::Esc => {
                        app.setup_wizard_input_mode = false;
                        app.setup_wizard_input_buffer.clear();
                    }
                    KeyCode::Enter => {
                        // Apply custom model path
                        match config::set_config_field(
                            &mut app.config_data,
                            "model_name",
                            &app.setup_wizard_input_buffer,
                        ) {
                            Ok(()) => {
                                if let Err(e) = app.save_config() {
                                    eprintln!("Failed to save config: {}", e);
                                } else {
                                    app.setup_wizard_input_mode = false;
                                    app.setup_wizard_input_buffer.clear();
                                    app.next_wizard_screen();
                                }
                            }
                            Err(e) => {
                                eprintln!("Invalid model path: {}", e);
                            }
                        }
                    }
                    KeyCode::Backspace => {
                        app.setup_wizard_input_buffer.pop();
                    }
                    KeyCode::Char(c) => {
                        app.setup_wizard_input_buffer.push(c);
                    }
                    _ => {}
                }
            } else {
                // In selection mode
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        if app.setup_wizard_selected_index > 0 {
                            app.setup_wizard_selected_index -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if app.setup_wizard_selected_index
                            < app.setup_wizard_options.len().saturating_sub(1)
                        {
                            app.setup_wizard_selected_index += 1;
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(option) = app
                            .setup_wizard_options
                            .get(app.setup_wizard_selected_index)
                        {
                            let action = option.action.clone();
                            match app.apply_model_selection(&action) {
                                Ok(()) => {
                                    if !app.setup_wizard_input_mode {
                                        // Only advance if not entering input mode
                                        app.next_wizard_screen();
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Error: {}", e);
                                }
                            }
                        }
                    }
                    KeyCode::Esc => {
                        app.previous_wizard_screen();
                    }
                    _ => {}
                }
            }
        }
        WatchDirectories => {
            // Watch directories screen with directory explorer
            match key.code {
                KeyCode::Char('n') => {
                    // Add current directory to list
                    if let Some(explorer) = &app.directory_picker {
                        let current = explorer.current();
                        let path = current.path();
                        let path_str = path.to_string_lossy().to_string();

                        if !app.setup_wizard_watch_dirs.contains(&path_str) {
                            app.setup_wizard_watch_dirs.push(path_str);
                        }
                    }
                }
                KeyCode::Char('c') => {
                    // Continue to next screen (save directories first)
                    if !app.setup_wizard_watch_dirs.is_empty() {
                        for dir in &app.setup_wizard_watch_dirs {
                            if !app.config_data.watch_directories.contains(dir) {
                                app.config_data.watch_directories.push(dir.clone());
                            }
                        }
                        if let Err(e) = app.save_config() {
                            eprintln!("Failed to save config: {}", e);
                        } else {
                            app.setup_wizard_watch_dirs.clear();
                            app.next_wizard_screen();
                        }
                    } else {
                        // No directories added, create atci_videos in home directory
                        if let Some(home_dir) = env::var_os("HOME") {
                            let mut atci_videos_path = PathBuf::from(home_dir);
                            atci_videos_path.push("atci_videos");

                            // Create the directory if it doesn't exist
                            match fs::create_dir_all(&atci_videos_path) {
                                Ok(()) => {
                                    let path_str = atci_videos_path.to_string_lossy().to_string();
                                    if !app.config_data.watch_directories.contains(&path_str) {
                                        app.config_data.watch_directories.push(path_str);
                                    }
                                    if let Err(e) = app.save_config() {
                                        eprintln!("Failed to save config: {}", e);
                                    } else {
                                        app.next_wizard_screen();
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Failed to create atci_videos directory: {}", e);
                                }
                            }
                        } else {
                            eprintln!("Could not determine home directory");
                        }
                    }
                }
                KeyCode::Esc => {
                    app.previous_wizard_screen();
                }
                _ => {
                    // Pass other keys to directory explorer
                    if let Some(explorer) = &mut app.directory_picker {
                        let event = Event::Key(key);
                        let _ = explorer.handle(&event);
                    }
                }
            }
        }
        Password => {
            // Password input screen
            match key.code {
                KeyCode::Esc => {
                    // Skip password (optional) and complete wizard
                    app.setup_wizard_input_buffer.clear();
                    match app.complete_wizard() {
                        Ok(()) => {}
                        Err(e) => {
                            eprintln!("Error completing wizard: {}", e);
                        }
                    }
                }
                KeyCode::Enter => {
                    // Save password and complete wizard
                    if !app.setup_wizard_input_buffer.is_empty() {
                        match config::set_config_field(
                            &mut app.config_data,
                            "password",
                            &app.setup_wizard_input_buffer,
                        ) {
                            Ok(()) => {
                                if let Err(e) = app.save_config() {
                                    eprintln!("Failed to save config: {}", e);
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to set password: {}", e);
                            }
                        }
                    }
                    app.setup_wizard_input_buffer.clear();
                    match app.complete_wizard() {
                        Ok(()) => {}
                        Err(e) => {
                            eprintln!("Error completing wizard: {}", e);
                        }
                    }
                }
                KeyCode::Backspace => {
                    app.setup_wizard_input_buffer.pop();
                }
                KeyCode::Char(c) => {
                    app.setup_wizard_input_buffer.push(c);
                }
                _ => {}
            }
        }
    }

    Ok(None)
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<(), Box<dyn Error>> {
    let conn = db::get_connection().expect("couldn't get db connection");
    terminal.draw(|f| ui(f, app, &conn))?;
    update_cursor_visibility(terminal, app)?;
    loop {
        // Check if we should refresh data (for transcripts tab)
        // if app.should_refresh() {
        //     if let Err(e) = app.refresh_data() {
        //         eprintln!("Failed to refresh data: {}", e);
        //     }
        // }

        if app.should_refresh_system_services() {
            app.refresh_system_services();
        }
        app.refresh_queue();
        if event::poll(Duration::from_millis(200))?
            && let Event::Key(key) = event::read()?
            && let Some(should_quit) = handle_key_event(app, key)?
            && should_quit
        {
            return Ok(());
        }
        terminal.draw(|f| ui(f, app, &conn))?;
        update_cursor_visibility(terminal, app)?;
    }
}

fn update_cursor_visibility<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &App,
) -> Result<(), Box<dyn Error>> {
    if app.config_editing_mode {
        terminal.show_cursor()?;
    } else {
        terminal.hide_cursor()?;
    }
    Ok(())
}

fn ui(f: &mut Frame, app: &mut App, conn: &rusqlite::Connection) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Min(3),    // Content area
                Constraint::Length(3), // Bottom panes area
            ]
            .as_ref(),
        )
        .split(f.area());

    let bottom_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage(100), // Controls area takes full width
            ]
            .as_ref(),
        )
        .split(chunks[1]);

    // Render content
    render_system_tab(f, chunks[0], app, conn);

    // Controls section
    let controls_text = if app.show_directory_picker {
        "↑↓/jk: Navigate  Enter: Open Directory  n: Select Directory  h/l: Parent/Child  Esc: Cancel".to_string()
    } else if app.config_editing_mode {
        "Enter: Save & Exit  Esc: Cancel  Type to edit...".to_string()
    } else {
        "↑↓/jk: Navigate  Enter: Edit  o: Open Browser App  Ctrl+C: Quit".to_string()
    };
    let controls_block = Block::default()
        .title("Controls")
        .borders(Borders::ALL)
        .border_style(Style::new().fg(app.colors.footer_border_color));

    let controls_paragraph = Paragraph::new(controls_text.as_str())
        .block(controls_block)
        .style(Style::new().fg(app.colors.row_fg))
        .alignment(Alignment::Center);

    f.render_widget(controls_paragraph, bottom_chunks[0]);

    // Render directory picker modal on top if shown (but not during setup wizard)
    if app.show_directory_picker
        && !app.show_setup_wizard
        && let Some(explorer) = &app.directory_picker
    {
        // Create a centered modal area
        let area = f.area();
        let popup_width = area.width.saturating_sub(10).min(100);
        let popup_height = area.height.saturating_sub(6).min(30);
        let popup_x = (area.width.saturating_sub(popup_width)) / 2;
        let popup_y = (area.height.saturating_sub(popup_height)) / 2;

        let popup_area = ratatui::layout::Rect {
            x: popup_x,
            y: popup_y,
            width: popup_width,
            height: popup_height,
        };

        // Render a clear background over the popup area to visually dim underlying UI
        let clear_block = Clear;
        f.render_widget(clear_block, popup_area);

        // Render a solid background block
        let block = Block::default()
            .title("Select Directory (Enter: Navigate, n: Select, Esc: Cancel)")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.colors.selection))
            .style(Style::default().bg(app.colors.buffer_bg));
        f.render_widget(block, popup_area);

        // Render the file explorer inside
        let inner_area = ratatui::layout::Rect {
            x: popup_area.x + 1,
            y: popup_area.y + 1,
            width: popup_area.width.saturating_sub(2),
            height: popup_area.height.saturating_sub(2),
        };
        f.render_widget_ref(explorer.widget(), inner_area);
    }
}

fn render_setup_wizard_modal(f: &mut Frame, app: &App) {
    // Use full screen area
    let area = f.area();

    // Get screen title
    let screen_title = match app.setup_wizard_screen {
        SetupWizardScreen::Welcome => "Welcome",
        SetupWizardScreen::FFmpeg => "FFmpeg",
        SetupWizardScreen::FFprobe => "FFprobe",
        SetupWizardScreen::WhisperCli => "Whisper CLI",
        SetupWizardScreen::Model => "Model",
        SetupWizardScreen::WatchDirectories => "Watch Directories",
        SetupWizardScreen::Password => "Password",
    };

    let (current_step, total_steps) = app.get_wizard_step_info();
    let title = if current_step > 0 {
        format!(
            "Setup Wizard - {} ({}/{})",
            screen_title, current_step, total_steps
        )
    } else {
        "Setup Wizard - Welcome".to_string()
    };

    // Main block with theme border color
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.colors.footer_border_color))
        .style(Style::default().bg(app.colors.buffer_bg));
    f.render_widget(block, area);

    // Inner area for content (with margin for borders)
    let inner_area = ratatui::layout::Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    // Render content based on current screen
    match app.setup_wizard_screen {
        SetupWizardScreen::Welcome => render_wizard_welcome(f, app, inner_area),
        SetupWizardScreen::FFmpeg | SetupWizardScreen::FFprobe | SetupWizardScreen::WhisperCli => {
            render_wizard_tool_selection(f, app, inner_area)
        }
        SetupWizardScreen::Model => render_wizard_model_selection(f, app, inner_area),
        SetupWizardScreen::WatchDirectories => render_wizard_watch_directories(f, app, inner_area),
        SetupWizardScreen::Password => render_wizard_password(f, app, inner_area),
    }

    // Render progress indicator at bottom
    if current_step > 0 && current_step <= total_steps {
        let progress_y = area.y + area.height - 2;
        let progress_text = (1..=total_steps)
            .map(|i| if i <= current_step { "●" } else { "○" })
            .collect::<Vec<_>>()
            .join(" ");

        let progress_paragraph = Paragraph::new(progress_text)
            .style(Style::default().fg(app.colors.footer_border_color))
            .alignment(Alignment::Center);

        let progress_area = ratatui::layout::Rect {
            x: area.x,
            y: progress_y,
            width: area.width,
            height: 1,
        };

        f.render_widget(progress_paragraph, progress_area);
    }
}

fn render_wizard_welcome(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    use ratatui::style::Modifier;
    use ratatui::text::{Line, Span};

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Welcome to atci!",
            Style::default()
                .fg(app.colors.footer_border_color)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("This wizard will help you configure the required settings:"),
        Line::from(""),
        Line::from("  1. FFmpeg - for video processing"),
        Line::from("  2. FFprobe - for video metadata"),
        Line::from("  3. Whisper CLI - for transcription"),
        Line::from("  4. Model - whisper model for transcription"),
        Line::from("  5. Watch Directories - folders to monitor"),
        Line::from("  6. Password - (optional) for web interface"),
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(
            "Press Enter to begin",
            Style::default().fg(app.colors.success),
        )),
    ];

    let paragraph = Paragraph::new(lines)
        .style(Style::default().fg(app.colors.row_fg))
        .alignment(Alignment::Left);

    f.render_widget(paragraph, area);
}

fn render_wizard_tool_selection(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    use ratatui::style::Modifier;
    use ratatui::text::{Line, Span};

    let tool_name = match app.setup_wizard_screen {
        SetupWizardScreen::FFmpeg => "FFmpeg",
        SetupWizardScreen::FFprobe => "FFprobe",
        SetupWizardScreen::WhisperCli => "Whisper CLI",
        _ => "Tool",
    };

    let mut lines = vec![
        Line::from(""),
        Line::from(format!("Select which {} to use:", tool_name)),
        Line::from(""),
    ];

    // Show input mode if active
    if app.setup_wizard_input_mode {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Enter custom path:",
            Style::default().fg(app.colors.footer_border_color),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(
                &app.setup_wizard_input_buffer,
                Style::default().fg(app.colors.success),
            ),
            Span::styled("█", Style::default().fg(app.colors.success)),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Press Enter to confirm, Esc to cancel",
            Style::default().fg(app.colors.disabled),
        )));
    } else {
        // Show options
        for (i, option) in app.setup_wizard_options.iter().enumerate() {
            let is_selected = i == app.setup_wizard_selected_index;
            let line = if is_selected {
                Line::from(vec![
                    Span::styled("► ", Style::default().fg(app.colors.selection)),
                    Span::styled(
                        &option.display_text,
                        Style::default()
                            .fg(app.colors.selection)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])
            } else {
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(&option.display_text, Style::default().fg(app.colors.row_fg)),
                ])
            };
            lines.push(line);
        }
    }

    let paragraph = Paragraph::new(lines)
        .style(Style::default().fg(app.colors.row_fg))
        .alignment(Alignment::Left);

    f.render_widget(paragraph, area);
}

fn render_wizard_model_selection(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    use ratatui::style::Modifier;
    use ratatui::text::{Line, Span};

    let mut lines = vec![
        Line::from(""),
        Line::from("Select which Whisper model to use:"),
        Line::from(""),
    ];

    // Show input mode if active
    if app.setup_wizard_input_mode {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Enter custom model path:",
            Style::default().fg(app.colors.footer_border_color),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(
                &app.setup_wizard_input_buffer,
                Style::default().fg(app.colors.success),
            ),
            Span::styled("█", Style::default().fg(app.colors.success)),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Press Enter to confirm, Esc to cancel",
            Style::default().fg(app.colors.disabled),
        )));
    } else {
        // Show options
        for (i, option) in app.setup_wizard_options.iter().enumerate() {
            let is_selected = i == app.setup_wizard_selected_index;
            let line = if is_selected {
                Line::from(vec![
                    Span::styled("► ", Style::default().fg(app.colors.selection)),
                    Span::styled(
                        &option.display_text,
                        Style::default()
                            .fg(app.colors.selection)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])
            } else {
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(&option.display_text, Style::default().fg(app.colors.row_fg)),
                ])
            };
            lines.push(line);
        }
    }

    let paragraph = Paragraph::new(lines)
        .style(Style::default().fg(app.colors.row_fg))
        .alignment(Alignment::Left);

    f.render_widget(paragraph, area);
}

fn render_wizard_watch_directories(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    use ratatui::style::Modifier;
    use ratatui::text::{Line, Span};

    // Split area: top for added directories, bottom for explorer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Min(10)])
        .split(area);

    // Render added directories list
    let mut lines = vec![
        Line::from(Span::styled(
            "Added Watch Directories:",
            Style::default()
                .fg(app.colors.footer_border_color)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    if app.setup_wizard_watch_dirs.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No directories added yet",
            Style::default().fg(app.colors.disabled),
        )));
    } else {
        for dir in &app.setup_wizard_watch_dirs {
            lines.push(Line::from(format!("  • {}", dir)));
        }
    }

    lines.push(Line::from(""));
    if app.setup_wizard_watch_dirs.is_empty() {
        lines.push(Line::from(Span::styled(
            "Press 'n' to add current directory",
            Style::default().fg(app.colors.success),
        )));
        lines.push(Line::from(Span::styled(
            "Press 'c' to create ~/atci_videos and continue",
            Style::default().fg(app.colors.success),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "Press 'n' to add current directory, 'c' to continue",
            Style::default().fg(app.colors.success),
        )));
    }

    let dirs_paragraph = Paragraph::new(lines)
        .style(Style::default().fg(app.colors.row_fg))
        .alignment(Alignment::Left);

    f.render_widget(dirs_paragraph, chunks[0]);

    // Render directory explorer if available
    if let Some(explorer) = &app.directory_picker {
        let explorer_block = Block::default()
            .title("Browse Directories")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.colors.footer_border_color));

        let explorer_area = chunks[1];
        f.render_widget(explorer_block.clone(), explorer_area);

        let inner_explorer_area = ratatui::layout::Rect {
            x: explorer_area.x + 1,
            y: explorer_area.y + 1,
            width: explorer_area.width.saturating_sub(2),
            height: explorer_area.height.saturating_sub(2),
        };

        f.render_widget_ref(explorer.widget(), inner_explorer_area);
    }
}

fn render_wizard_password(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    use ratatui::text::{Line, Span};

    let lines = vec![
        Line::from(""),
        Line::from("Set an optional password for the web interface:"),
        Line::from(""),
        Line::from(Span::styled(
            "(This is optional - press Esc to skip)",
            Style::default().fg(app.colors.disabled),
        )),
        Line::from(""),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "Password: ",
                Style::default().fg(app.colors.footer_border_color),
            ),
            Span::styled(
                "*".repeat(app.setup_wizard_input_buffer.len()),
                Style::default().fg(app.colors.success),
            ),
            Span::styled("█", Style::default().fg(app.colors.success)),
        ]),
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(
            "Press Enter to save, Esc to skip",
            Style::default().fg(app.colors.success),
        )),
    ];

    let paragraph = Paragraph::new(lines)
        .style(Style::default().fg(app.colors.row_fg))
        .alignment(Alignment::Left);

    f.render_widget(paragraph, area);
}
