use crate::system_tab::render_system_tab;
use crate::{config, db};
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
    error::Error,
    io,
    time::{Duration, Instant},
};

pub struct TableColors {
    pub buffer_bg: Color,
    pub header_bg: Color,
    pub header_fg: Color,
    pub row_fg: Color,
    pub footer_border_color: Color,
}

impl TableColors {
    const fn new(color: &tailwind::Palette) -> Self {
        Self {
            buffer_bg: tailwind::SLATE.c950,
            header_bg: color.c900,
            header_fg: tailwind::SLATE.c200,
            row_fg: tailwind::SLATE.c200,
            footer_border_color: color.c400,
        }
    }
}

mod tailwind {
    use ratatui::style::Color;

    pub struct Palette {
        pub c200: Color,
        pub c400: Color,
        pub c900: Color,
        pub c950: Color,
    }

    pub const SLATE: Palette = Palette {
        c200: Color::Rgb(226, 232, 240),
        c400: Color::Rgb(148, 163, 184),
        c900: Color::Rgb(15, 23, 42),
        c950: Color::Rgb(2, 6, 23),
    };

    pub const BLUE: Palette = Palette {
        c200: Color::Rgb(191, 219, 254),
        c400: Color::Rgb(96, 165, 250),
        c900: Color::Rgb(30, 58, 138),
        c950: Color::Rgb(23, 37, 84),
    };
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
        App {
            colors: TableColors::new(&tailwind::BLUE),
            current_tab: TabState::System,
            system_services: Vec::new(),
            last_system_refresh: Instant::now(),
            config_data: config::load_config_or_default(),
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
        }
    }
}

impl App {
    fn new() -> Result<App, Box<dyn Error>> {
        let mut app = App {
            colors: TableColors::new(&tailwind::BLUE),
            current_tab: TabState::System,
            system_services: Vec::new(),
            last_system_refresh: Instant::now(),
            config_data: config::load_config_or_default(),
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
        };

        // Initialize system services
        app.refresh_system_services();

        // Initialize queue
        app.refresh_queue();

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
        11 // Total number of config fields (excluding watch_directories)
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
        }
        self.stop_config_editing();
        Ok(())
    }

    pub fn save_config(&mut self) -> Result<(), String> {
        config::store_config(&self.config_data).map_err(|e| format!("Failed to save config: {}", e))
    }

    pub fn reload_config(&mut self) {
        self.config_data = config::load_config_or_default();
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
}

pub fn run() -> Result<(), Box<dyn Error>> {
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

        let mut app = App::new()?;
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
        "↑↓/jk: Navigate  Enter: Edit  o: Open Browser App  n: Add Directory  d: Remove Directory  Ctrl+C: Quit".to_string()
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

    // Render directory picker modal on top if shown
    if app.show_directory_picker
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
            .border_style(Style::default().fg(Color::Yellow))
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
