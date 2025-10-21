use crate::tui::TableColors;
use crate::{config, model_manager, tools_manager};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, SetTitle, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    widgets::{Block, Borders, Paragraph},
};
use std::{
    env, error::Error, fs, io,
    path::PathBuf
};

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

pub struct SetupWizard {
    pub colors: TableColors,
    pub config_data: config::AtciConfig,
    pub show_setup_wizard: bool,
    pub setup_wizard_screen: SetupWizardScreen,
    pub setup_wizard_options: Vec<ToolOption>,
    pub setup_wizard_selected_index: usize,
    pub setup_wizard_input_buffer: String,
    pub setup_wizard_input_mode: bool,
    pub setup_wizard_watch_dirs: Vec<String>,
    pub directory_picker: Option<ratatui_explorer::FileExplorer>,
}

impl SetupWizard {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let config_data = config::load_config_or_default();
        let mut wizard = Self {
            colors: TableColors::from_config(&config_data),
            config_data,
            show_setup_wizard: true,
            setup_wizard_screen: SetupWizardScreen::Welcome,
            setup_wizard_options: Vec::new(),
            setup_wizard_selected_index: 0,
            setup_wizard_input_buffer: String::new(),
            setup_wizard_input_mode: false,
            setup_wizard_watch_dirs: Vec::new(),
            directory_picker: None,
        };

        // Start wizard, which will skip Welcome if config exists
        wizard.start_wizard();

        Ok(wizard)
    }

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

    pub fn get_wizard_step_info(&self) -> (usize, usize) {
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

    pub fn save_config(&mut self) -> Result<(), String> {
        config::store_config(&self.config_data).map_err(|e| format!("Failed to save config: {}", e))
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

pub fn run_setup_wizard() -> Result<bool, Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        SetTitle("atci - Setup Wizard"),
        EnterAlternateScreen,
        EnableMouseCapture
    )?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut wizard = SetupWizard::new()?;
    let should_continue = run_wizard_loop(&mut terminal, &mut wizard)?;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(should_continue)
}

fn run_wizard_loop<B: Backend>(
    terminal: &mut Terminal<B>,
    wizard: &mut SetupWizard,
) -> Result<bool, Box<dyn Error>> {
    loop {
        // ideally we would never call terminal.clear(), as it means the screen flashes on input since the entire terminal has to redraw.
        // but for the setup wizard, we jump in and out of raw_mode (i.e. the ratatui app), 
        // and when we come _back_ into the tui, it gets half rendered and is generally broken visually, because ratatui assumes the space is clear.
        // until I move the download stuff into the ratatui application, we will have to deal with a little screen flicker
        let _ = terminal.clear();
        terminal.draw(|f| {
            // Render the setup wizard (full screen)
            render_setup_wizard_modal(f, wizard);
        })?;

        if let Event::Key(key) = event::read()?
        {
            if let Some(should_quit) = handle_setup_wizard_input(wizard, key)?
                && should_quit
            {
                // User pressed Ctrl+C or Esc to quit - don't continue to main TUI
                return Ok(false);
            }

            // Check if wizard is complete (completed successfully)
            if !wizard.show_setup_wizard {
                // Wizard completed successfully - continue to main TUI
                return Ok(true);
            }
        }
    }
}

fn handle_setup_wizard_input(
    wizard: &mut SetupWizard,
    key: crossterm::event::KeyEvent,
) -> Result<Option<bool>, Box<dyn Error>> {
    use SetupWizardScreen::*;

    // Allow Ctrl+C to exit at any time during setup wizard
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return Ok(Some(true));
    }

    match wizard.setup_wizard_screen {
        Welcome => {
            // On welcome screen, Enter advances, Esc quits
            match key.code {
                KeyCode::Enter => {
                    wizard.next_wizard_screen();
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
            if wizard.setup_wizard_input_mode {
                // In input mode for custom path
                match key.code {
                    KeyCode::Esc => {
                        wizard.setup_wizard_input_mode = false;
                        wizard.setup_wizard_input_buffer.clear();
                    }
                    KeyCode::Enter => {
                        // Apply custom path
                        let field = match wizard.setup_wizard_screen {
                            FFmpeg => "ffmpeg_path",
                            FFprobe => "ffprobe_path",
                            WhisperCli => "whispercli_path",
                            _ => "",
                        };
                        match config::set_config_field(
                            &mut wizard.config_data,
                            field,
                            &wizard.setup_wizard_input_buffer,
                        ) {
                            Ok(()) => {
                                if let Err(e) = wizard.save_config() {
                                    eprintln!("Failed to save config: {}", e);
                                } else {
                                    wizard.setup_wizard_input_mode = false;
                                    wizard.setup_wizard_input_buffer.clear();
                                    wizard.next_wizard_screen();
                                }
                            }
                            Err(e) => {
                                eprintln!("Invalid path: {}", e);
                            }
                        }
                    }
                    KeyCode::Backspace => {
                        wizard.setup_wizard_input_buffer.pop();
                    }
                    KeyCode::Char(c) => {
                        wizard.setup_wizard_input_buffer.push(c);
                    }
                    _ => {}
                }
            } else {
                // In selection mode
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        if wizard.setup_wizard_selected_index > 0 {
                            wizard.setup_wizard_selected_index -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if wizard.setup_wizard_selected_index
                            < wizard.setup_wizard_options.len().saturating_sub(1)
                        {
                            wizard.setup_wizard_selected_index += 1;
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(option) = wizard
                            .setup_wizard_options
                            .get(wizard.setup_wizard_selected_index)
                        {
                            let field = match wizard.setup_wizard_screen {
                                FFmpeg => "ffmpeg_path",
                                FFprobe => "ffprobe_path",
                                WhisperCli => "whispercli_path",
                                _ => "",
                            };
                            let action = option.action.clone();
                            match wizard.apply_tool_selection(field, &action) {
                                Ok(()) => {
                                    if !wizard.setup_wizard_input_mode {
                                        // Only advance if not entering input mode
                                        wizard.next_wizard_screen();
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Error: {}", e);
                                }
                            }
                        }
                    }
                    KeyCode::Esc => {
                        if wizard.setup_wizard_screen != Welcome {
                            wizard.previous_wizard_screen();
                        }
                    }
                    _ => {}
                }
            }
        }
        Model => {
            // Model selection screen
            if wizard.setup_wizard_input_mode {
                // In input mode for custom model path
                match key.code {
                    KeyCode::Esc => {
                        wizard.setup_wizard_input_mode = false;
                        wizard.setup_wizard_input_buffer.clear();
                    }
                    KeyCode::Enter => {
                        // Apply custom model path
                        match config::set_config_field(
                            &mut wizard.config_data,
                            "model_name",
                            &wizard.setup_wizard_input_buffer,
                        ) {
                            Ok(()) => {
                                if let Err(e) = wizard.save_config() {
                                    eprintln!("Failed to save config: {}", e);
                                } else {
                                    wizard.setup_wizard_input_mode = false;
                                    wizard.setup_wizard_input_buffer.clear();
                                    wizard.next_wizard_screen();
                                }
                            }
                            Err(e) => {
                                eprintln!("Invalid model path: {}", e);
                            }
                        }
                    }
                    KeyCode::Backspace => {
                        wizard.setup_wizard_input_buffer.pop();
                    }
                    KeyCode::Char(c) => {
                        wizard.setup_wizard_input_buffer.push(c);
                    }
                    _ => {}
                }
            } else {
                // In selection mode
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        if wizard.setup_wizard_selected_index > 0 {
                            wizard.setup_wizard_selected_index -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if wizard.setup_wizard_selected_index
                            < wizard.setup_wizard_options.len().saturating_sub(1)
                        {
                            wizard.setup_wizard_selected_index += 1;
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(option) = wizard
                            .setup_wizard_options
                            .get(wizard.setup_wizard_selected_index)
                        {
                            let action = option.action.clone();
                            match wizard.apply_model_selection(&action) {
                                Ok(()) => {
                                    if !wizard.setup_wizard_input_mode {
                                        // Only advance if not entering input mode
                                        wizard.next_wizard_screen();
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Error: {}", e);
                                }
                            }
                        }
                    }
                    KeyCode::Esc => {
                        wizard.previous_wizard_screen();
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
                    if let Some(explorer) = &wizard.directory_picker {
                        let current = explorer.current();
                        let path = current.path();
                        let path_str = path.to_string_lossy().to_string();

                        if !wizard.setup_wizard_watch_dirs.contains(&path_str) {
                            wizard.setup_wizard_watch_dirs.push(path_str);
                        }
                    }
                }
                KeyCode::Char('c') => {
                    // Continue to next screen (save directories first)
                    if !wizard.setup_wizard_watch_dirs.is_empty() {
                        for dir in &wizard.setup_wizard_watch_dirs {
                            if !wizard.config_data.watch_directories.contains(dir) {
                                wizard.config_data.watch_directories.push(dir.clone());
                            }
                        }
                        if let Err(e) = wizard.save_config() {
                            eprintln!("Failed to save config: {}", e);
                        } else {
                            wizard.setup_wizard_watch_dirs.clear();
                            wizard.next_wizard_screen();
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
                                    if !wizard.config_data.watch_directories.contains(&path_str) {
                                        wizard.config_data.watch_directories.push(path_str);
                                    }
                                    if let Err(e) = wizard.save_config() {
                                        eprintln!("Failed to save config: {}", e);
                                    } else {
                                        wizard.next_wizard_screen();
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
                    wizard.previous_wizard_screen();
                }
                _ => {
                    // Pass other keys to directory explorer
                    if let Some(explorer) = &mut wizard.directory_picker {
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
                    wizard.setup_wizard_input_buffer.clear();
                    match wizard.complete_wizard() {
                        Ok(()) => {}
                        Err(e) => {
                            eprintln!("Error completing wizard: {}", e);
                        }
                    }
                }
                KeyCode::Enter => {
                    // Save password and complete wizard
                    if !wizard.setup_wizard_input_buffer.is_empty() {
                        match config::set_config_field(
                            &mut wizard.config_data,
                            "password",
                            &wizard.setup_wizard_input_buffer,
                        ) {
                            Ok(()) => {
                                if let Err(e) = wizard.save_config() {
                                    eprintln!("Failed to save config: {}", e);
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to set password: {}", e);
                            }
                        }
                    }
                    wizard.setup_wizard_input_buffer.clear();
                    match wizard.complete_wizard() {
                        Ok(()) => {}
                        Err(e) => {
                            eprintln!("Error completing wizard: {}", e);
                        }
                    }
                }
                KeyCode::Backspace => {
                    wizard.setup_wizard_input_buffer.pop();
                }
                KeyCode::Char(c) => {
                    wizard.setup_wizard_input_buffer.push(c);
                }
                _ => {}
            }
        }
    }

    Ok(None)
}

fn render_setup_wizard_modal(f: &mut Frame, wizard: &SetupWizard) {
    // Use full screen area
    let area = f.area();

    // Get screen title
    let screen_title = match wizard.setup_wizard_screen {
        SetupWizardScreen::Welcome => "Welcome",
        SetupWizardScreen::FFmpeg => "FFmpeg",
        SetupWizardScreen::FFprobe => "FFprobe",
        SetupWizardScreen::WhisperCli => "Whisper CLI",
        SetupWizardScreen::Model => "Model",
        SetupWizardScreen::WatchDirectories => "Watch Directories",
        SetupWizardScreen::Password => "Password",
    };

    let (current_step, total_steps) = wizard.get_wizard_step_info();
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
        .border_style(Style::default().fg(wizard.colors.footer_border_color))
        .style(Style::default().bg(wizard.colors.buffer_bg));
    f.render_widget(block, area);

    // Inner area for content (with margin for borders)
    let inner_area = ratatui::layout::Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    // Render content based on current screen
    match wizard.setup_wizard_screen {
        SetupWizardScreen::Welcome => render_wizard_welcome(f, wizard, inner_area),
        SetupWizardScreen::FFmpeg | SetupWizardScreen::FFprobe | SetupWizardScreen::WhisperCli => {
            render_wizard_tool_selection(f, wizard, inner_area)
        }
        SetupWizardScreen::Model => render_wizard_model_selection(f, wizard, inner_area),
        SetupWizardScreen::WatchDirectories => render_wizard_watch_directories(f, wizard, inner_area),
        SetupWizardScreen::Password => render_wizard_password(f, wizard, inner_area),
    }

    // Render progress indicator at bottom
    if current_step > 0 && current_step <= total_steps {
        let progress_y = area.y + area.height - 2;
        let progress_text = (1..=total_steps)
            .map(|i| if i <= current_step { "●" } else { "○" })
            .collect::<Vec<_>>()
            .join(" ");

        let progress_paragraph = Paragraph::new(progress_text)
            .style(Style::default().fg(wizard.colors.footer_border_color))
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

fn render_wizard_welcome(f: &mut Frame, wizard: &SetupWizard, area: ratatui::layout::Rect) {
    use ratatui::text::{Line, Span};

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Welcome to atci!",
            Style::default()
                .fg(wizard.colors.footer_border_color)
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
            Style::default().fg(wizard.colors.success),
        )),
    ];

    let paragraph = Paragraph::new(lines)
        .style(Style::default().fg(wizard.colors.row_fg))
        .alignment(Alignment::Left);

    f.render_widget(paragraph, area);
}

fn render_wizard_tool_selection(f: &mut Frame, wizard: &SetupWizard, area: ratatui::layout::Rect) {
    use ratatui::text::{Line, Span};

    let tool_name = match wizard.setup_wizard_screen {
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
    if wizard.setup_wizard_input_mode {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Enter custom path:",
            Style::default().fg(wizard.colors.footer_border_color),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(
                &wizard.setup_wizard_input_buffer,
                Style::default().fg(wizard.colors.success),
            ),
            Span::styled("█", Style::default().fg(wizard.colors.success)),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Press Enter to confirm, Esc to cancel",
            Style::default().fg(wizard.colors.disabled),
        )));
    } else {
        // Show options
        for (i, option) in wizard.setup_wizard_options.iter().enumerate() {
            let is_selected = i == wizard.setup_wizard_selected_index;
            let line = if is_selected {
                Line::from(vec![
                    Span::styled("► ", Style::default().fg(wizard.colors.selection)),
                    Span::styled(
                        &option.display_text,
                        Style::default()
                            .fg(wizard.colors.selection)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])
            } else {
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(&option.display_text, Style::default().fg(wizard.colors.row_fg)),
                ])
            };
            lines.push(line);
        }
    }

    let paragraph = Paragraph::new(lines)
        .style(Style::default().fg(wizard.colors.row_fg))
        .alignment(Alignment::Left);

    f.render_widget(paragraph, area);
}

fn render_wizard_model_selection(f: &mut Frame, wizard: &SetupWizard, area: ratatui::layout::Rect) {
    use ratatui::text::{Line, Span};

    let mut lines = vec![
        Line::from(""),
        Line::from("Select which Whisper model to use:"),
        Line::from(""),
    ];

    // Show input mode if active
    if wizard.setup_wizard_input_mode {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Enter custom model path:",
            Style::default().fg(wizard.colors.footer_border_color),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(
                &wizard.setup_wizard_input_buffer,
                Style::default().fg(wizard.colors.success),
            ),
            Span::styled("█", Style::default().fg(wizard.colors.success)),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Press Enter to confirm, Esc to cancel",
            Style::default().fg(wizard.colors.disabled),
        )));
    } else {
        // Show options
        for (i, option) in wizard.setup_wizard_options.iter().enumerate() {
            let is_selected = i == wizard.setup_wizard_selected_index;
            let line = if is_selected {
                Line::from(vec![
                    Span::styled("► ", Style::default().fg(wizard.colors.selection)),
                    Span::styled(
                        &option.display_text,
                        Style::default()
                            .fg(wizard.colors.selection)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])
            } else {
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(&option.display_text, Style::default().fg(wizard.colors.row_fg)),
                ])
            };
            lines.push(line);
        }
    }

    let paragraph = Paragraph::new(lines)
        .style(Style::default().fg(wizard.colors.row_fg))
        .alignment(Alignment::Left);

    f.render_widget(paragraph, area);
}

fn render_wizard_watch_directories(f: &mut Frame, wizard: &SetupWizard, area: ratatui::layout::Rect) {
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
                .fg(wizard.colors.footer_border_color)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    if wizard.setup_wizard_watch_dirs.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No directories added yet",
            Style::default().fg(wizard.colors.disabled),
        )));
    } else {
        for dir in &wizard.setup_wizard_watch_dirs {
            lines.push(Line::from(format!("  • {}", dir)));
        }
    }

    lines.push(Line::from(""));
    if wizard.setup_wizard_watch_dirs.is_empty() {
        lines.push(Line::from(Span::styled(
            "Press 'n' to add current directory",
            Style::default().fg(wizard.colors.success),
        )));
        lines.push(Line::from(Span::styled(
            "Press 'c' to create ~/atci_videos and continue",
            Style::default().fg(wizard.colors.success),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "Press 'n' to add current directory, 'c' to continue",
            Style::default().fg(wizard.colors.success),
        )));
    }

    let dirs_paragraph = Paragraph::new(lines)
        .style(Style::default().fg(wizard.colors.row_fg))
        .alignment(Alignment::Left);

    f.render_widget(dirs_paragraph, chunks[0]);

    // Render directory explorer if available
    if let Some(explorer) = &wizard.directory_picker {
        let explorer_block = Block::default()
            .title("Browse Directories")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(wizard.colors.footer_border_color));

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

fn render_wizard_password(f: &mut Frame, wizard: &SetupWizard, area: ratatui::layout::Rect) {
    use ratatui::text::{Line, Span};

    let lines = vec![
        Line::from(""),
        Line::from("Set an optional password for the web interface:"),
        Line::from(""),
        Line::from(Span::styled(
            "(This is optional - press Esc to skip)",
            Style::default().fg(wizard.colors.disabled),
        )),
        Line::from(""),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "Password: ",
                Style::default().fg(wizard.colors.footer_border_color),
            ),
            Span::styled(
                "*".repeat(wizard.setup_wizard_input_buffer.len()),
                Style::default().fg(wizard.colors.success),
            ),
            Span::styled("█", Style::default().fg(wizard.colors.success)),
        ]),
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(
            "Press Enter to save, Esc to skip",
            Style::default().fg(wizard.colors.success),
        )),
    ];

    let paragraph = Paragraph::new(lines)
        .style(Style::default().fg(wizard.colors.row_fg))
        .alignment(Alignment::Left);

    f.render_widget(paragraph, area);
}
