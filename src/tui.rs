use crate::editor_tab::{EditorData, render_editor_tab};
use crate::file_tab::{FileViewData, render_file_view_tab};
use crate::queue_tab::render_queue_tab;
use crate::search_tab::render_search_results_tab;
use crate::system_tab::render_system_tab;
use crate::transcripts_tab::render_transcripts_tab;
use crate::{config, db, files, search, short_url};
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
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, TableState},
};
use std::{
    error::Error,
    io,
    time::{Duration, Instant},
};
use throbber_widgets_tui::ThrobberState;

pub struct TableColors {
    pub buffer_bg: Color,
    pub header_bg: Color,
    pub header_fg: Color,
    pub row_fg: Color,
    pub selected_style_fg: Color,
    pub normal_row_color: Color,
    pub alt_row_color: Color,
    pub footer_border_color: Color,
}

impl TableColors {
    const fn new(color: &tailwind::Palette) -> Self {
        Self {
            buffer_bg: tailwind::SLATE.c950,
            header_bg: color.c900,
            header_fg: tailwind::SLATE.c200,
            row_fg: tailwind::SLATE.c200,
            selected_style_fg: color.c400,
            normal_row_color: tailwind::SLATE.c950,
            alt_row_color: tailwind::SLATE.c900,
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
pub enum SortOrder {
    Ascending,
    Descending,
}

#[derive(Clone, Copy, PartialEq)]
pub enum TabState {
    Transcripts,
    System,
    Queue,
    SearchResults,
    Editor,
    FileView,
}

#[derive(Clone, Copy, PartialEq)]
pub enum SystemSection {
    Services,
    Config,
}

pub struct App {
    pub state: TableState,
    pub colors: TableColors,
    pub video_data: Vec<files::VideoInfo>,
    pub sort_column: Option<usize>,
    pub sort_order: SortOrder,
    pub _last_refresh: Instant,
    pub terminal_height: u16,
    pub current_page: u32,
    pub total_pages: u32,
    pub total_records: u32,
    pub current_tab: TabState,
    pub filter_input: String,
    pub filter_input_mode: bool,
    pub search_input: String,
    pub search_input_mode: bool,
    pub system_selected_index: usize,
    pub system_services: Vec<SystemService>,
    pub last_system_refresh: Instant,
    pub search_results: Vec<search::SearchResult>,
    pub search_selected_index: usize,
    pub last_search_query: String,
    pub search_scroll_offset: usize,
    pub search_in_progress: bool,
    pub search_requested: bool,
    pub search_thread: Option<std::thread::JoinHandle<Result<Vec<search::SearchResult>, String>>>,
    pub throbber_state: ThrobberState,
    pub editor_data: Option<EditorData>,
    pub file_view_data: Option<FileViewData>,
    pub file_view_timestamp_mode: bool,
    pub config_data: config::AtciConfig,
    pub config_selected_field: usize,
    pub config_editing_mode: bool,
    pub config_input_buffer: String,
    pub system_section: SystemSection,
    pub queue_selected_index: usize,
    pub queue_items: Vec<String>,
    pub currently_processing: Option<String>,
    pub currently_processing_age: u64,
    pub show_regenerate_popup: bool,
    pub regenerate_popup_selected: usize,
    pub regenerate_popup_options: Vec<String>,
    pub regenerate_popup_option_types: Vec<String>,
}

#[derive(Clone)]
pub struct SystemService {
    pub name: String,
    pub status: ServiceStatus,
    pub pids: Vec<u32>,
}

#[derive(Clone)]
pub enum ServiceStatus {
    Active,
    Stopped,
}

impl Default for App {
    fn default() -> App {
        App {
            state: TableState::default(),
            colors: TableColors::new(&tailwind::BLUE),
            video_data: Vec::new(),
            sort_column: None,
            sort_order: SortOrder::Ascending,
            _last_refresh: Instant::now(),
            terminal_height: 24,
            current_page: 0,
            total_pages: 1,
            total_records: 0,
            current_tab: TabState::System,
            filter_input: String::new(),
            filter_input_mode: false,
            search_input: String::new(),
            search_input_mode: false,
            system_selected_index: 0,
            system_services: Vec::new(),
            last_system_refresh: Instant::now(),
            search_results: Vec::new(),
            search_selected_index: 0,
            last_search_query: String::new(),
            search_scroll_offset: 0,
            search_in_progress: false,
            search_requested: false,
            search_thread: None,
            throbber_state: ThrobberState::default(),
            editor_data: None,
            file_view_data: None,
            file_view_timestamp_mode: false,
            config_data: config::load_config_or_default(),
            config_selected_field: 0,
            config_editing_mode: false,
            config_input_buffer: String::new(),
            system_section: SystemSection::Services,
            queue_selected_index: 0,
            queue_items: Vec::new(),
            currently_processing: None,
            currently_processing_age: 0,
            show_regenerate_popup: false,
            regenerate_popup_selected: 0,
            regenerate_popup_options: Vec::new(),
            regenerate_popup_option_types: Vec::new(),
        }
    }
}

impl App {
    fn new() -> Result<App, Box<dyn Error>> {
        let terminal_height = crossterm::terminal::size()?.1;
        let page_size = Self::calculate_page_size(terminal_height);

        // Use database sorting instead of client-side sorting
        let cache_data = files::load_sorted_paginated_cache_data(
            None,             // filter (no filter on initial load)
            0,                // page (first page)
            page_size,        // limit
            "last_generated", // sort by Generated At
            0,                // sort_order (0 = DESC, 1 = ASC)
        )?;

        let mut app = App {
            state: TableState::default(),
            colors: TableColors::new(&tailwind::BLUE),
            video_data: cache_data.files,
            sort_column: Some(2), // Generated At column
            sort_order: SortOrder::Descending,
            _last_refresh: Instant::now(),
            terminal_height,
            current_page: 0,
            total_pages: cache_data.pages.unwrap_or(1),
            total_records: cache_data.total_records.unwrap_or(0),
            current_tab: TabState::System,
            filter_input: String::new(),
            filter_input_mode: false,
            search_input: String::new(),
            search_input_mode: false,
            system_selected_index: 0,
            system_services: Vec::new(),
            last_system_refresh: Instant::now(),
            search_results: Vec::new(),
            search_selected_index: 0,
            last_search_query: String::new(),
            search_scroll_offset: 0,
            search_in_progress: false,
            search_requested: false,
            search_thread: None,
            throbber_state: ThrobberState::default(),
            editor_data: None,
            file_view_data: None,
            file_view_timestamp_mode: false,
            config_data: config::load_config_or_default(),
            config_selected_field: 0,
            config_editing_mode: false,
            config_input_buffer: String::new(),
            system_section: SystemSection::Services,
            queue_selected_index: 0,
            queue_items: Vec::new(),
            currently_processing: None,
            currently_processing_age: 0,
            show_regenerate_popup: false,
            regenerate_popup_selected: 0,
            regenerate_popup_options: Vec::new(),
            regenerate_popup_option_types: Vec::new(),
        };

        // Select first item if available
        if !app.video_data.is_empty() {
            app.state.select(Some(0));
        }

        // Initialize system services
        app.refresh_system_services();

        // Initialize queue
        app.refresh_queue();

        Ok(app)
    }

    fn calculate_page_size(terminal_height: u16) -> u32 {
        // Account for: margins (2), header (1), controls (3), table header (1), borders (2)
        // Leave some buffer for safety
        let available_height = terminal_height.saturating_sub(9);
        std::cmp::max(available_height as u32, 5) + 1 // Minimum 5 rows
    }

    pub fn get_page_size(&self) -> u32 {
        Self::calculate_page_size(self.terminal_height)
    }

    // fn should_refresh(&self) -> bool {
    //     self._last_refresh.elapsed() >= Duration::from_secs(60)
    // }

    pub fn toggle_tab(&mut self) {
        self.current_tab = match self.current_tab {
            TabState::SearchResults => TabState::Transcripts,
            TabState::Transcripts => {
                if self.file_view_data.is_some() {
                    TabState::FileView
                } else if self.editor_data.is_some() {
                    TabState::Editor
                } else {
                    TabState::System
                }
            }
            TabState::FileView => {
                if self.editor_data.is_some() {
                    TabState::Editor
                } else {
                    TabState::System
                }
            }
            TabState::Editor => TabState::System,
            TabState::System => TabState::Queue,
            TabState::Queue => TabState::SearchResults,
        };
    }

    pub fn switch_to_transcripts(&mut self) {
        self.current_tab = TabState::Transcripts;
    }

    pub fn switch_to_system(&mut self) {
        self.current_tab = TabState::System;
    }

    pub fn switch_to_search_results(&mut self) {
        self.current_tab = TabState::SearchResults;
        // Update total records count with current filter
        self.update_total_records();
        // Populate search input with the last search query for easy editing
        if !self.last_search_query.is_empty() {
            self.search_input = self.last_search_query.clone();
        }
    }

    pub fn switch_to_file_view(&mut self) {
        self.current_tab = TabState::FileView;
    }

    pub fn open_file_view(&mut self, video_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let file_view_data = FileViewData::new(video_path.to_string())?;
        self.file_view_data = Some(file_view_data);
        self.current_tab = TabState::FileView;
        Ok(())
    }

    pub fn file_view_navigate_up(&mut self) {
        if let Some(data) = &mut self.file_view_data {
            data.navigate_up();
        }
    }

    pub fn file_view_navigate_down(&mut self) {
        if let Some(data) = &mut self.file_view_data {
            data.navigate_down();
        }
    }

    // pub fn file_view_jump_to_top(&mut self) {
    //     if let Some(data) = &mut self.file_view_data {
    //         data.jump_to_top();
    //     }
    // }

    // pub fn file_view_jump_to_bottom(&mut self) {
    //     if let Some(data) = &mut self.file_view_data {
    //         data.jump_to_bottom();
    //     }
    // }

    // pub fn file_view_page_up(&mut self) {
    //     if let Some(data) = &mut self.file_view_data {
    //         let page_size = (self.terminal_height as usize).saturating_sub(10);
    //         data.page_up(page_size);
    //     }
    // }

    // pub fn file_view_page_down(&mut self) {
    //     if let Some(data) = &mut self.file_view_data {
    //         let page_size = (self.terminal_height as usize).saturating_sub(10);
    //         data.page_down(page_size);
    //     }
    // }

    pub fn file_view_navigate_to_nearest_timestamp(&mut self) {
        if let Some(data) = &mut self.file_view_data {
            data.navigate_to_nearest_timestamp();
        }
    }

    pub fn file_view_navigate_to_next_timestamp(&mut self) {
        if let Some(data) = &mut self.file_view_data {
            data.navigate_to_next_timestamp();
        }
    }

    pub fn file_view_navigate_to_previous_timestamp(&mut self) {
        if let Some(data) = &mut self.file_view_data {
            data.navigate_to_previous_timestamp();
        }
    }

    pub fn file_view_navigate_to_next_timestamp_range(&mut self) {
        if let Some(data) = &mut self.file_view_data {
            data.navigate_to_next_timestamp_range();
        }
    }

    pub fn file_view_navigate_to_previous_timestamp_range(&mut self) {
        if let Some(data) = &mut self.file_view_data {
            data.navigate_to_previous_timestamp_range();
        }
    }

    // pub fn file_view_start_range_selection(&mut self) {
    //     if let Some(data) = &mut self.file_view_data {
    //         data.start_range_selection();
    //     }
    // }

    pub fn file_view_select_both_timestamps_on_current_line(&mut self) -> bool {
        if let Some(data) = &mut self.file_view_data {
            data.select_both_timestamps_on_current_line()
        } else {
            false
        }
    }

    pub fn file_view_jump_to_next_timestamp_and_select_both(&mut self) -> bool {
        if let Some(data) = &mut self.file_view_data {
            let result = data.jump_to_next_timestamp_and_select_both();
            if result {
                self.file_view_timestamp_mode = true;
            }
            result
        } else {
            false
        }
    }

    pub fn file_view_jump_to_previous_timestamp_and_select_both(&mut self) -> bool {
        if let Some(data) = &mut self.file_view_data {
            let result = data.jump_to_previous_timestamp_and_select_both();
            if result {
                self.file_view_timestamp_mode = true;
            }
            result
        } else {
            false
        }
    }

    pub fn parse_timestamp(&self, timestamp: &str) -> Option<String> {
        // Simple validation and cleanup of timestamp format
        // Expects format like "00:01:07.220"
        if timestamp.matches(':').count() >= 2 && timestamp.contains('.') {
            Some(timestamp.to_string())
        } else {
            None
        }
    }

    pub fn show_clip_url_popup_from_file_view(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(file_data) = &self.file_view_data {
            // Check if we have a range selected first
            if let Some((start_timestamp, end_timestamp)) =
                file_data.get_selected_range_timestamps()
            {
                // Parse the range timestamps
                if let (Some(start_time), Some(end_time)) = (
                    self.parse_timestamp(&start_timestamp),
                    self.parse_timestamp(&end_timestamp),
                ) {
                    let line_text = "".to_string(); // Could be more descriptive
                    let video_path = file_data.video_path.clone();

                    // Show clip URL popup with the range timestamps
                    self.show_clip_url_popup(video_path, start_time, end_time, line_text);
                    Ok(())
                } else {
                    Err("Could not parse timestamps from selected range".into())
                }
            } else if let Some(timestamp_line) = file_data.get_timestamp_for_current_line() {
                // Fallback to current line timestamp if no range selected
                if let Some((start_time, end_time)) = self.parse_timestamp_range(&timestamp_line) {
                    let line_text = file_data.get_text_for_current_line();
                    let video_path = file_data.video_path.clone();

                    // Show clip URL popup with the timestamp information
                    self.show_clip_url_popup(video_path, start_time, end_time, line_text);
                    Ok(())
                } else {
                    Err("Could not parse timestamp from current line".into())
                }
            } else {
                Err("No timestamp found for current line or line above".into())
            }
        } else {
            Err("No file view data available".into())
        }
    }

    pub fn toggle_filter_input(&mut self) {
        self.filter_input_mode = !self.filter_input_mode;
    }

    pub fn update_total_records(&mut self) {
        if let Ok(count) = files::count_cache_records(self.get_filter_option().as_ref()) {
            self.total_records = count;
        }
    }

    pub fn clear_filter(&mut self) {
        self.filter_input.clear();
        self.filter_input_mode = false;
        // Reload data without filter
        if self.current_tab == TabState::SearchResults {
            self.update_total_records();
            self.search_requested = true;
            self.search_in_progress = true;
        } else if let Err(e) = self.reload_with_current_sort() {
            eprintln!("Failed to reload data after clearing filter: {}", e);
        }
    }

    pub fn apply_filter(&mut self) {
        self.filter_input_mode = false;
        // If we're on SearchResults tab, re-run the search with the new filter
        if self.current_tab == TabState::SearchResults {
            self.update_total_records();
            self.search_requested = true;
            self.search_in_progress = true;
        } else {
            // For Transcripts tab, reload data with current filter
            if let Err(e) = self.reload_with_current_sort() {
                eprintln!("Failed to reload data with filter: {}", e);
            }
        }
    }

    pub fn add_char_to_filter(&mut self, c: char) {
        if self.filter_input_mode {
            self.filter_input.push(c);
            // Refresh data immediately as user types
            if self.current_tab == TabState::SearchResults {
                self.update_total_records();
                self.search_requested = true;
                self.search_in_progress = true;
            } else if let Err(e) = self.reload_with_current_sort() {
                eprintln!("Failed to reload data while typing filter: {}", e);
            }
        }
    }

    pub fn remove_char_from_filter(&mut self) {
        if self.filter_input_mode {
            self.filter_input.pop();
            // Refresh data immediately as user types
            if self.current_tab == TabState::SearchResults {
                self.update_total_records();
                self.search_requested = true;
                self.search_in_progress = true;
            } else if let Err(e) = self.reload_with_current_sort() {
                eprintln!("Failed to reload data while typing filter: {}", e);
            }
        }
    }

    pub fn get_filter_option(&self) -> Option<Vec<String>> {
        if self.filter_input.is_empty() {
            None
        } else {
            Some(
                self.filter_input
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect(),
            )
        }
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
        12 // Total number of config fields
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
            "watch_directories",
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
            7 => self.config_data.watch_directories.join(", "),
            8 => self.config_data.allow_whisper.to_string(),
            9 => self.config_data.allow_subtitles.to_string(),
            10 => self.config_data.stream_chunk_size.to_string(),
            11 => self.config_data.hostname.clone(),
            _ => String::new(),
        }
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

    pub fn switch_to_queue(&mut self) {
        self.current_tab = TabState::Queue;
        self.refresh_queue();
    }

    pub fn queue_next(&mut self) {
        let total_items = if self.currently_processing.is_some() {
            self.queue_items.len() + 1
        } else {
            self.queue_items.len()
        };

        if total_items > 0 && self.queue_selected_index < total_items - 1 {
            self.queue_selected_index += 1;
        }
    }

    pub fn queue_previous(&mut self) {
        if self.queue_selected_index > 0 {
            self.queue_selected_index -= 1;
        }
    }

    pub fn show_regenerate_popup(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        use crate::model_manager;
        use std::process::Command;

        // Get the selected video path
        let video_path = if let Some(selected_index) = self.state.selected() {
            if selected_index < self.video_data.len() {
                self.video_data[selected_index].full_path.clone()
            } else {
                return Err("No video selected".into());
            }
        } else {
            return Err("No video selected".into());
        };

        let video_path_obj = std::path::Path::new(&video_path);
        if !video_path_obj.exists() {
            return Err(format!("Video file does not exist: {}", video_path).into());
        }

        let cfg = config::load_config()?;
        let mut options = Vec::new();
        let mut option_types = Vec::new();

        // Check for subtitle streams using ffprobe directly (synchronous)
        let ffprobe_output = Command::new(&cfg.ffprobe_path)
            .args([
                "-v",
                "quiet",
                "-print_format",
                "json",
                "-show_streams",
                "-select_streams",
                "s",
                video_path_obj.to_str().unwrap(),
            ])
            .output();

        if let Ok(output) = ffprobe_output
            && output.status.success()
            && let Ok(json_str) = String::from_utf8(output.stdout)
            && let Ok(json) = serde_json::from_str::<serde_json::Value>(&json_str)
            && let Some(streams) = json["streams"].as_array()
        {
            for stream in streams {
                let index = stream["index"].as_i64().unwrap_or(0) as i32;
                let language = stream["tags"]["language"].as_str().unwrap_or("unknown");
                let title = stream["tags"]["title"].as_str();

                let lang_display = if let Some(t) = title {
                    format!("{} - {}", language, t)
                } else {
                    language.to_string()
                };

                options.push(format!("Subtitles: {} ({})", lang_display, index));
                option_types.push(format!("subtitle_{}", index));
            }
        }

        // Check for available Whisper models
        let models = model_manager::list_models();
        let downloaded_models: Vec<_> = models.iter().filter(|m| m.downloaded).collect();

        if !downloaded_models.is_empty() && cfg.allow_whisper {
            for model in &downloaded_models {
                options.push(format!("Whisper Model: {}", model.name));
                option_types.push(format!("whisper_{}", model.name));
            }
        }

        options.push("Cancel".to_string());
        option_types.push("cancel".to_string());

        if options.len() == 1 {
            return Err("No processing options available".into());
        }

        self.regenerate_popup_options = options;
        self.regenerate_popup_option_types = option_types;
        self.regenerate_popup_selected = 0;
        self.show_regenerate_popup = true;

        Ok(())
    }

    pub fn close_regenerate_popup(&mut self) {
        self.show_regenerate_popup = false;
        self.regenerate_popup_selected = 0;
        self.regenerate_popup_options.clear();
        self.regenerate_popup_option_types.clear();
    }

    pub fn build_clip_url(
        &self,
        filename: &str,
        start_time: &str,
        end_time: &str,
        text: Option<&str>,
    ) -> String {
        // URL encode the parameters
        let encoded_filename = urlencoding::encode(filename);
        let encoded_start = urlencoding::encode(start_time);
        let encoded_end = urlencoding::encode(end_time);

        let mut url = format!(
            "{}/clip/view?filename={}&start_time={}&end_time={}",
            self.config_data.hostname, encoded_filename, encoded_start, encoded_end
        );

        if let Some(t) = text
            && !t.is_empty()
        {
            let encoded_text = urlencoding::encode(t);
            url.push_str(&format!("&text={}", encoded_text));
        }

        url
    }

    pub fn show_clip_url_popup(
        &mut self,
        filename: String,
        start_time: String,
        end_time: String,
        text: String,
    ) {
        let text_opt = if text.is_empty() {
            None
        } else {
            Some(text.as_str())
        };
        let full_clip_url = self.build_clip_url(&filename, &start_time, &end_time, text_opt);

        // Create a short URL for the full clip URL
        let clip_url = match short_url::get_or_create(&full_clip_url) {
            Ok(short_id) => {
                // Build the short URL using the hostname and the short ID
                format!("{}/short/{}", self.config_data.hostname, short_id)
            }
            Err(e) => {
                eprintln!("Failed to create short URL: {}", e);
                // Fallback to the full URL if short URL creation fails
                full_clip_url
            }
        };

        // Copy URL to clipboard
        if let Err(e) = self.copy_text_to_clipboard(&clip_url) {
            eprintln!("Failed to copy URL to clipboard: {}", e);
        }

        // Open URL in default browser
        if let Err(e) = open::that(&clip_url) {
            eprintln!("Failed to open URL in browser: {}", e);
        }
    }

    fn copy_text_to_clipboard(&self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        use clipboard_rs::{Clipboard, ClipboardContext};

        let ctx = ClipboardContext::new()
            .map_err(|e| format!("Failed to create clipboard context: {}", e))?;

        ctx.set_text(text.to_string())
            .map_err(|e| format!("Failed to set text in clipboard: {}", e))?;

        Ok(())
    }

    pub fn regenerate_popup_next(&mut self) {
        if self.regenerate_popup_selected < self.regenerate_popup_options.len().saturating_sub(1) {
            self.regenerate_popup_selected += 1;
        }
    }

    pub fn regenerate_popup_previous(&mut self) {
        if self.regenerate_popup_selected > 0 {
            self.regenerate_popup_selected -= 1;
        }
    }

    pub fn execute_regenerate_selection(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.regenerate_popup_selected >= self.regenerate_popup_option_types.len() {
            return Err("Invalid selection".into());
        }

        let option_type = &self.regenerate_popup_option_types[self.regenerate_popup_selected];

        if option_type == "cancel" {
            self.close_regenerate_popup();
            return Ok(());
        }

        // Get the selected video path
        let video_path = if let Some(selected_index) = self.state.selected() {
            if selected_index < self.video_data.len() {
                self.video_data[selected_index].full_path.clone()
            } else {
                return Err("No video selected".into());
            }
        } else {
            return Err("No video selected".into());
        };

        // Determine model and subtitle stream based on selection
        let (model, subtitle_stream_index) = if option_type.starts_with("subtitle_") {
            let stream_index = option_type
                .strip_prefix("subtitle_")
                .unwrap()
                .parse::<i32>()?;
            (None, Some(stream_index))
        } else if option_type.starts_with("whisper_") {
            let model_name = option_type.strip_prefix("whisper_").unwrap();
            (Some(model_name.to_string()), None)
        } else {
            return Err("Unknown option type".into());
        };

        // Add to queue instead of processing immediately
        crate::queue::add_to_queue(&video_path, model, subtitle_stream_index)?;

        self.close_regenerate_popup();
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

    // Handle regenerate popup
    if app.show_regenerate_popup {
        match key.code {
            KeyCode::Esc => app.close_regenerate_popup(),
            KeyCode::Enter => {
                if let Err(e) = app.execute_regenerate_selection() {
                    eprintln!("Failed to execute regenerate: {}", e);
                    app.close_regenerate_popup();
                }
            }
            KeyCode::Down | KeyCode::Char('j') => app.regenerate_popup_next(),
            KeyCode::Up | KeyCode::Char('k') => app.regenerate_popup_previous(),
            _ => {}
        }
        return Ok(None);
    }

    // Handle filter input mode
    if app.filter_input_mode {
        match key.code {
            KeyCode::Esc => app.filter_input_mode = false,
            KeyCode::Enter => app.apply_filter(),
            KeyCode::Backspace => app.remove_char_from_filter(),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.clear_filter();
            }
            KeyCode::Char(c) => app.add_char_to_filter(c),
            _ => {}
        }
        return Ok(None);
    }

    // Handle search input mode
    if app.search_input_mode {
        match key.code {
            KeyCode::Esc => app.search_input_mode = false,
            KeyCode::Enter => app.apply_search(),
            KeyCode::Backspace => app.remove_char_from_search(),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.clear_search();
            }
            KeyCode::Char(c) => app.add_char_to_search(c),
            _ => {}
        }
        return Ok(None);
    }

    // Handle text editing mode
    if app.current_tab == TabState::Editor
        && app
            .editor_data
            .as_ref()
            .is_some_and(|data| data.text_editing_mode)
    {
        match key.code {
            KeyCode::Esc => app.exit_text_editing(),
            KeyCode::Enter => app.exit_text_editing(),
            KeyCode::Backspace => app.remove_char_from_text(),
            KeyCode::Char(c) => app.add_char_to_text(c),
            _ => {}
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
        KeyCode::Char('z') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            return Ok(Some(true));
        } // Signal to quit
        KeyCode::Tab => app.toggle_tab(),
        KeyCode::Char('t') => app.switch_to_transcripts(),
        KeyCode::Char('s') => app.switch_to_system(),
        KeyCode::Char('q') => app.switch_to_queue(),
        KeyCode::Char('r') => {
            if app.current_tab == TabState::Transcripts {
                // Show regenerate popup
                if let Err(e) = app.show_regenerate_popup() {
                    eprintln!("Failed to show regenerate popup: {}", e);
                }
            }
        }
        KeyCode::Char('e') => {
            // Only switch to editor if we have editor data
            if app.editor_data.is_some() {
                app.switch_to_editor();
            }
        }
        KeyCode::Char('v') => {
            if app.file_view_data.is_some() {
                // Switch to file view if we have file view data
                app.switch_to_file_view();
            }
        }
        KeyCode::Char('f') => {
            if app.current_tab == TabState::Transcripts
                || app.current_tab == TabState::SearchResults
            {
                app.toggle_filter_input();
            }
        }
        KeyCode::Char('/') => {
            if app.current_tab == TabState::SearchResults {
                // If already on search results, toggle search input
                app.toggle_search_input();
            } else {
                // Switch to search results tab
                app.switch_to_search_results();
            }
        }
        KeyCode::Char('o') => {
            if app.current_tab == TabState::Editor {
                // Open clip
                if let Err(e) = app.open_clip() {
                    eprintln!("Failed to open clip: {}", e);
                }
            }
        }
        KeyCode::Char('[') => {
            if app.current_tab == TabState::Editor {
                app.adjust_selected_frame_time(false); // Move backward by 0.1s
            }
        }
        KeyCode::Char(']') => {
            if app.current_tab == TabState::Editor {
                app.adjust_selected_frame_time(true); // Move forward by 0.1s
            }
        }
        KeyCode::Char('{') => {
            if app.current_tab == TabState::Editor {
                app.adjust_selected_frame_time_by_second(false); // Move backward by 1s
            }
        }
        KeyCode::Char('}') => {
            if app.current_tab == TabState::Editor {
                app.adjust_selected_frame_time_by_second(true); // Move forward by 1s
            }
        }
        KeyCode::Char('+') | KeyCode::Char('=') => {
            if app.current_tab == TabState::Editor {
                app.adjust_font_size(true); // Increase font size
            }
        }
        KeyCode::Char('-') => {
            if app.current_tab == TabState::Editor {
                app.adjust_font_size(false); // Decrease font size
            }
        }
        KeyCode::Char('c') => {
            if (app.current_tab == TabState::Transcripts
                || app.current_tab == TabState::SearchResults)
                && key.modifiers.contains(KeyModifiers::CONTROL)
            {
                app.clear_filter();
                app.clear_search();
            } else if app.current_tab == TabState::SearchResults {
                // Show clip URL popup from selected search result
                if let Err(e) = app.show_clip_url_popup_from_selected_match() {
                    eprintln!("Failed to show clip URL: {}", e);
                }
            } else if app.current_tab == TabState::Editor {
                // Copy clip
                if let Err(e) = app.copy_clip() {
                    eprintln!("Failed to copy clip: {}", e);
                }
            } else if app.current_tab == TabState::FileView {
                // Show clip URL popup from current line with timestamp
                if let Err(e) = app.show_clip_url_popup_from_file_view() {
                    eprintln!("Failed to show clip URL: {}", e);
                }
            }
        }
        KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
            if app.current_tab == TabState::Transcripts {
                if key.code == KeyCode::Char('J') || key.modifiers.contains(KeyModifiers::SHIFT) {
                    app.jump_to_bottom_of_page();
                } else {
                    app.next();
                }
            } else if app.current_tab == TabState::System {
                app.system_next();
            } else if app.current_tab == TabState::Queue {
                app.queue_next();
            } else if app.current_tab == TabState::SearchResults {
                app.search_next();
            } else if app.current_tab == TabState::Editor {
                app.navigate_editor_selection_up_or_down(true); // j = down
            } else if app.current_tab == TabState::FileView {
                if key.code == KeyCode::Char('J')
                    || (key.code == KeyCode::Char('j')
                        && key.modifiers.contains(KeyModifiers::SHIFT))
                    || (key.code == KeyCode::Down && key.modifiers.contains(KeyModifiers::SHIFT))
                {
                    if app.file_view_timestamp_mode {
                        // Capital J or Shift+j or Shift+Down - range selection forward
                        app.file_view_navigate_to_next_timestamp_range();
                    } else {
                        // When not in timestamp mode, try to select both timestamps on current line
                        // If no timestamps on current line, jump to next timestamp line and select both
                        if !app.file_view_select_both_timestamps_on_current_line() {
                            app.file_view_jump_to_next_timestamp_and_select_both();
                        }
                        app.file_view_timestamp_mode = true;
                    }
                } else {
                    app.file_view_navigate_down();
                    app.file_view_timestamp_mode = false;
                }
            }
        }
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
            if app.current_tab == TabState::Transcripts {
                if key.code == KeyCode::Char('K') || key.modifiers.contains(KeyModifiers::SHIFT) {
                    app.jump_to_top_of_page();
                } else {
                    app.previous();
                }
            } else if app.current_tab == TabState::System {
                app.system_previous();
            } else if app.current_tab == TabState::Queue {
                app.queue_previous();
            } else if app.current_tab == TabState::SearchResults {
                app.search_previous();
            } else if app.current_tab == TabState::Editor {
                app.navigate_editor_selection_up_or_down(false); // k = up
            } else if app.current_tab == TabState::FileView {
                if key.code == KeyCode::Char('K')
                    || (key.code == KeyCode::Char('k')
                        && key.modifiers.contains(KeyModifiers::SHIFT))
                    || (key.code == KeyCode::Up && key.modifiers.contains(KeyModifiers::SHIFT))
                {
                    if app.file_view_timestamp_mode {
                        // Capital K or Shift+k or Shift+Up - range selection backward
                        app.file_view_navigate_to_previous_timestamp_range();
                    } else {
                        // When not in timestamp mode, try to select both timestamps on current line
                        // If no timestamps on current line, jump to previous timestamp line and select both
                        if !app.file_view_select_both_timestamps_on_current_line() {
                            app.file_view_jump_to_previous_timestamp_and_select_both();
                        }
                        app.file_view_timestamp_mode = true;
                    }
                } else {
                    app.file_view_navigate_up();
                    app.file_view_timestamp_mode = false;
                }
            }
        }
        KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('H') => {
            if app.current_tab == TabState::Transcripts {
                if key.code == KeyCode::Char('H') || key.modifiers.contains(KeyModifiers::SHIFT) {
                    app.jump_to_first_page();
                } else {
                    app.prev_page();
                }
            } else if app.current_tab == TabState::Editor {
                app.navigate_editor_selection_left_or_right(false); // h/left = up/previous
            } else if app.current_tab == TabState::FileView {
                if key.code == KeyCode::Char('H')
                    || (key.code == KeyCode::Char('h')
                        && key.modifiers.contains(KeyModifiers::SHIFT))
                    || (key.code == KeyCode::Left && key.modifiers.contains(KeyModifiers::SHIFT))
                {
                    if app.file_view_timestamp_mode {
                        // Capital H or Shift+h or Shift+Left - range selection backward
                        app.file_view_navigate_to_previous_timestamp_range();
                    } else {
                        // Regular H behavior when not in timestamp mode
                        app.file_view_navigate_to_nearest_timestamp();
                        app.file_view_timestamp_mode = true;
                    }
                } else if !app.file_view_timestamp_mode {
                    app.file_view_navigate_to_nearest_timestamp();
                    app.file_view_timestamp_mode = true;
                } else {
                    app.file_view_navigate_to_previous_timestamp();
                }
            }
        }
        KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('L') => {
            if app.current_tab == TabState::Transcripts {
                if key.code == KeyCode::Char('L') || key.modifiers.contains(KeyModifiers::SHIFT) {
                    app.jump_to_last_page();
                } else {
                    app.next_page();
                }
            } else if app.current_tab == TabState::Editor {
                app.navigate_editor_selection_left_or_right(true); // l/right = down/next
            } else if app.current_tab == TabState::FileView {
                if key.code == KeyCode::Char('L')
                    || (key.code == KeyCode::Char('l')
                        && key.modifiers.contains(KeyModifiers::SHIFT))
                    || (key.code == KeyCode::Right && key.modifiers.contains(KeyModifiers::SHIFT))
                {
                    if app.file_view_timestamp_mode {
                        // Capital L or Shift+l or Shift+Right - range selection forward
                        app.file_view_navigate_to_next_timestamp_range();
                    } else {
                        // Regular L behavior when not in timestamp mode
                        app.file_view_navigate_to_nearest_timestamp();
                        app.file_view_timestamp_mode = true;
                    }
                } else if !app.file_view_timestamp_mode {
                    app.file_view_navigate_to_nearest_timestamp();
                    app.file_view_timestamp_mode = true;
                } else {
                    app.file_view_navigate_to_next_timestamp();
                }
            }
        }
        KeyCode::Enter => {
            if app.current_tab == TabState::System {
                match app.system_section {
                    SystemSection::Services => {
                        // Check if the selected service is active or stopped
                        if app.system_selected_index < app.system_services.len() {
                            let service = &app.system_services[app.system_selected_index];
                            match service.status {
                                ServiceStatus::Active => {
                                    if let Err(e) = app.kill_selected_service() {
                                        eprintln!("Failed to kill process: {}", e);
                                    }
                                }
                                ServiceStatus::Stopped => {
                                    if let Err(e) = app.start_selected_service() {
                                        eprintln!("Failed to start service: {}", e);
                                    }
                                }
                            }
                        }
                    }
                    SystemSection::Config => {
                        // Config editing mode
                        app.start_config_editing();
                    }
                }
            } else if app.current_tab == TabState::Transcripts {
                // Open file view from selected transcript
                if let Some(selected_index) = app.state.selected()
                    && selected_index < app.video_data.len()
                {
                    let video_path = app.video_data[selected_index].full_path.clone();
                    if let Err(e) = app.open_file_view(&video_path) {
                        eprintln!("Failed to open file view: {}", e);
                    }
                }
            } else if app.current_tab == TabState::SearchResults {
                // Open file view from selected search result
                if let Err(e) = app.open_file_view_from_selected_match() {
                    eprintln!("Failed to open file view: {}", e);
                }
            } else if app.current_tab == TabState::Editor {
                app.activate_selected_element();
            }
        }
        KeyCode::Char('1') => {
            if app.current_tab == TabState::Transcripts {
                app.sort_by_column(0);
            }
        }
        KeyCode::Char('2') => {
            if app.current_tab == TabState::Transcripts {
                app.sort_by_column(1);
            }
        }
        KeyCode::Char('3') => {
            if app.current_tab == TabState::Transcripts {
                app.sort_by_column(2);
            }
        }
        KeyCode::Char('4') => {
            if app.current_tab == TabState::Transcripts {
                app.sort_by_column(3);
            }
        }
        KeyCode::Char('5') => {
            if app.current_tab == TabState::Transcripts {
                app.sort_by_column(4);
            }
        }
        KeyCode::Char('6') => {
            if app.current_tab == TabState::Transcripts {
                app.sort_by_column(5);
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

        // Refresh system services and queue every 200ms
        if app.current_tab == TabState::System {
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
        } else if app.current_tab == TabState::Queue {
            // Refresh queue every second
            app.refresh_queue();
            if event::poll(Duration::from_secs(1))?
                && let Event::Key(key) = event::read()?
                && let Some(should_quit) = handle_key_event(app, key)?
                && should_quit
            {
                return Ok(());
            }
            terminal.draw(|f| ui(f, app, &conn))?;
            update_cursor_visibility(terminal, app)?;
        } else if app.current_tab == TabState::SearchResults {
            // Check if we need to start a search
            if app.search_requested {
                // Spawn the search thread (non-blocking)
                app.perform_search();
            }

            // Check if search thread has completed
            app.check_search_thread();

            // Poll every 200ms to keep throbber animating and check for input
            if event::poll(Duration::from_millis(200))?
                && let Event::Key(key) = event::read()?
                && let Some(should_quit) = handle_key_event(app, key)?
                && should_quit
            {
                return Ok(());
            }

            // Always redraw to update throbber animation
            terminal.draw(|f| ui(f, app, &conn))?;
            update_cursor_visibility(terminal, app)?;
        } else if let Event::Key(key) = event::read()? {
            if let Some(should_quit) = handle_key_event(app, key)?
                && should_quit
            {
                return Ok(());
            }
            terminal.draw(|f| ui(f, app, &conn))?;
            update_cursor_visibility(terminal, app)?;
        }
    }
}

fn update_cursor_visibility<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &App,
) -> Result<(), Box<dyn Error>> {
    if app.filter_input_mode
        || app.search_input_mode
        || (app.current_tab == TabState::Editor
            && app
                .editor_data
                .as_ref()
                .is_some_and(|data| data.text_editing_mode))
    {
        terminal.show_cursor()?;
    } else {
        terminal.hide_cursor()?;
    }
    Ok(())
}

fn ui(f: &mut Frame, app: &mut App, conn: &rusqlite::Connection) {
    let chunks =
        if app.current_tab == TabState::Transcripts || app.current_tab == TabState::SearchResults {
            Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    [
                        Constraint::Min(3),    // Content area
                        Constraint::Length(3), // Filter/Search area
                        Constraint::Length(3), // Bottom panes area
                    ]
                    .as_ref(),
                )
                .split(f.area())
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    [
                        Constraint::Min(3),    // Content area
                        Constraint::Length(3), // Bottom panes area
                    ]
                    .as_ref(),
                )
                .split(f.area())
        };

    // Split the bottom area into Controls and Page panes (only for Transcripts tab)
    let bottom_chunks = if app.current_tab == TabState::Transcripts {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Percentage(90), // Controls area (9/10)
                    Constraint::Percentage(10), // Page area (1/10)
                ]
                .as_ref(),
            )
            .split(chunks[2]) // Use index 2 since we have filter area
    } else if app.current_tab == TabState::SearchResults {
        // For SearchResults tab, use full width for controls (no page info)
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Percentage(100), // Controls area takes full width
                ]
                .as_ref(),
            )
            .split(chunks[2]) // Use index 2 since we have filter/search area
    } else {
        // For other tabs, use full width for controls
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Percentage(100), // Controls area takes full width
                ]
                .as_ref(),
            )
            .split(chunks[1])
    };

    let header_style = Style::default()
        .fg(app.colors.header_fg)
        .bg(app.colors.header_bg);
    let selected_row_style = Style::default()
        .add_modifier(Modifier::REVERSED)
        .fg(app.colors.selected_style_fg);

    // Render content based on current tab
    match app.current_tab {
        TabState::Transcripts => {
            render_transcripts_tab(f, chunks[0], app, header_style, selected_row_style)
        }
        TabState::System => render_system_tab(f, chunks[0], app, conn),
        TabState::Queue => render_queue_tab(f, chunks[0], app),
        TabState::SearchResults => render_search_results_tab(f, chunks[0], app),
        TabState::Editor => render_editor_tab(f, chunks[0], app),
        TabState::FileView => render_file_view_tab(f, chunks[0], app),
    }

    // Render filter section (on transcripts tab) or filter+search sections (on search results tab)
    if app.current_tab == TabState::Transcripts {
        render_filter_section(f, chunks[1], app);
    } else if app.current_tab == TabState::SearchResults {
        render_filter_and_search_sections(f, chunks[1], app);
    }

    // Controls section
    let controls_text = if app.show_regenerate_popup {
        "↑↓/jk: Navigate  Enter: Select  Esc: Cancel".to_string()
    } else {
        match app.current_tab {
            TabState::Transcripts => {
                if app.filter_input_mode {
                    "Enter: Apply  Esc: Cancel  Ctrl+C: Clear  Type to filter...".to_string()
                } else {
                    let base_controls = "↑↓/jk: Navigate  ←→/hl: Page  1-6: Sort  r: Regenerate  f: Filter  Enter: View File  Ctrl+C: Clear  t/s/q/";
                    let mut tab_controls = String::new();
                    if app.editor_data.is_some() {
                        tab_controls.push('e');
                    }
                    if app.file_view_data.is_some() {
                        if !tab_controls.is_empty() {
                            tab_controls.push('-');
                        }
                        tab_controls.push('v');
                    }
                    if !tab_controls.is_empty() {
                        format!(
                            "{}{}-Tab: Switch  Ctrl+Z: Quit",
                            base_controls, tab_controls
                        )
                    } else {
                        format!("{}-Tab: Switch  Ctrl+Z: Quit", base_controls)
                    }
                }
            }
            TabState::System => {
                if app.config_editing_mode {
                    "Enter: Save & Exit  Esc: Cancel  Type to edit...".to_string()
                } else {
                    let section_info = match app.system_section {
                        SystemSection::Services => "Services: Enter: Start/Kill",
                        SystemSection::Config => "Config: Enter: Edit",
                    };
                    let base_controls =
                        format!("↑↓/jk: Navigate  {}  Shift+R: Reload  t/s/q/", section_info);
                    let mut tab_controls = String::new();
                    if app.editor_data.is_some() {
                        tab_controls.push('e');
                    }
                    if app.file_view_data.is_some() {
                        if !tab_controls.is_empty() {
                            tab_controls.push('-');
                        }
                        tab_controls.push('v');
                    }
                    if !tab_controls.is_empty() {
                        format!(
                            "{}{}-Tab: Switch  Ctrl+Z: Quit",
                            base_controls, tab_controls
                        )
                    } else {
                        format!("{}-Tab: Switch  Ctrl+Z: Quit", base_controls)
                    }
                }
            }
            TabState::Queue => {
                let base_controls = "↑↓/jk: Navigate  t/s/q/";
                let mut tab_controls = String::new();
                if app.editor_data.is_some() {
                    tab_controls.push('e');
                }
                if app.file_view_data.is_some() {
                    if !tab_controls.is_empty() {
                        tab_controls.push('-');
                    }
                    tab_controls.push('v');
                }
                if !tab_controls.is_empty() {
                    format!(
                        "{}{}-Tab: Switch  Ctrl+Z: Quit",
                        base_controls, tab_controls
                    )
                } else {
                    format!("{}-Tab: Switch  Ctrl+Z: Quit", base_controls)
                }
            }
            TabState::SearchResults => {
                if app.filter_input_mode {
                    "Enter: Apply  Esc: Cancel  Ctrl+C: Clear  Type to filter...".to_string()
                } else if app.search_input_mode {
                    "Enter: Search  Esc: Cancel  Ctrl+C: Clear  Type to search...".to_string()
                } else {
                    let base_controls = "↑↓/jk: Navigate  Enter: View File  c: Open Editor  f: Filter  /: Search  Ctrl+C: Clear  t/s";
                    let mut tab_controls = String::new();
                    if app.editor_data.is_some() {
                        tab_controls.push('e');
                    }
                    if !tab_controls.is_empty() {
                        format!(
                            "{} {}-Tab: Switch  Ctrl+Z: Quit",
                            base_controls, tab_controls
                        )
                    } else {
                        format!("{}-Tab: Switch  Ctrl+Z: Quit", base_controls)
                    }
                }
            }
            TabState::Editor => {
                let _overlay_status = if app
                    .editor_data
                    .as_ref()
                    .is_some_and(|data| data.show_overlay_text)
                {
                    "ON"
                } else {
                    "OFF"
                };
                let pending_regen = ""; // No longer using async regeneration
                let base_controls = format!(
                    "[/]: Adjust Frame Time{}  j/k/h/l: Navigate  Enter: Activate  c: Copy  o: Open  t/s/q/",
                    pending_regen
                );
                let mut tab_controls = String::new();
                if app.file_view_data.is_some() {
                    tab_controls.push('v');
                }
                if !tab_controls.is_empty() {
                    format!(
                        "{}{}-Tab: Switch  Ctrl+Z: Quit",
                        base_controls, tab_controls
                    )
                } else {
                    format!("{}-Tab: Switch  Ctrl+Z: Quit", base_controls)
                }
            }
            TabState::FileView => {
                let base_controls = "↑↓/jk: Navigate  ←→/hl: Page Up/Down  c: Open Editor  t/s/q/";
                let mut tab_controls = String::new();
                if app.editor_data.is_some() {
                    tab_controls.push('e');
                }
                if !tab_controls.is_empty() {
                    format!(
                        "{}{}-Tab: Switch  Ctrl+Z: Quit",
                        base_controls, tab_controls
                    )
                } else {
                    format!("{}-Tab: Switch  Ctrl+Z: Quit", base_controls)
                }
            }
        }
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

    // Page info section (only show on transcripts tab)
    if app.current_tab == TabState::Transcripts {
        let page_text = format!("{} / {}", app.current_page + 1, app.total_pages);
        let page_block = Block::default()
            .title("Page")
            .borders(Borders::ALL)
            .border_style(Style::new().fg(app.colors.footer_border_color));

        let page_paragraph = Paragraph::new(page_text)
            .block(page_block)
            .style(Style::new().fg(app.colors.row_fg))
            .alignment(Alignment::Center);

        f.render_widget(page_paragraph, bottom_chunks[1]);
    }
}

pub fn create_tab_title_with_editor(
    current_tab: TabState,
    colors: &TableColors,
    _has_search_results: bool,
    has_editor_data: bool,
    has_file_view_data: bool,
) -> ratatui::text::Line<'_> {
    use ratatui::style::Color;
    use ratatui::text::{Line, Span};

    let mut spans = vec![
        match current_tab {
            TabState::SearchResults => {
                Span::styled("Search (/)", Style::default().fg(Color::White))
            }
            _ => Span::styled(
                "Search (/)",
                Style::default().fg(colors.footer_border_color),
            ),
        },
        Span::styled(" | ", Style::default().fg(colors.row_fg)),
        match current_tab {
            TabState::Transcripts => {
                Span::styled("Transcripts (t)", Style::default().fg(Color::White))
            }
            _ => Span::styled(
                "Transcripts (t)",
                Style::default().fg(colors.footer_border_color),
            ),
        },
    ];

    // Only show file view tab if we have file view data
    if has_file_view_data {
        spans.push(Span::styled(" | ", Style::default().fg(colors.row_fg)));
        spans.push(match current_tab {
            TabState::FileView => Span::styled("File View (v)", Style::default().fg(Color::White)),
            _ => Span::styled(
                "File View (v)",
                Style::default().fg(colors.footer_border_color),
            ),
        });
    }

    // Only show editor tab if we have editor data
    if has_editor_data {
        spans.push(Span::styled(" | ", Style::default().fg(colors.row_fg)));
        spans.push(match current_tab {
            TabState::Editor => Span::styled("Editor (e)", Style::default().fg(Color::White)),
            _ => Span::styled(
                "Editor (e)",
                Style::default().fg(colors.footer_border_color),
            ),
        });
    }

    // System and Queue tabs
    spans.push(Span::styled(" | ", Style::default().fg(colors.row_fg)));
    spans.push(match current_tab {
        TabState::System => Span::styled("System (s)", Style::default().fg(Color::White)),
        _ => Span::styled(
            "System (s)",
            Style::default().fg(colors.footer_border_color),
        ),
    });
    spans.push(Span::styled(" | ", Style::default().fg(colors.row_fg)));
    spans.push(match current_tab {
        TabState::Queue => Span::styled("Queue (q)", Style::default().fg(Color::White)),
        _ => Span::styled("Queue (q)", Style::default().fg(colors.footer_border_color)),
    });

    Line::from(spans)
}

fn render_filter_section(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    // Render filter section only (for Transcripts tab)
    let filter_text = if app.filter_input.is_empty() {
        "Enter comma-separated filters (e.g., mp4,youtube,2024)".to_string()
    } else {
        app.filter_input.clone()
    };

    let filter_style = if app.filter_input_mode {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(app.colors.row_fg)
    };

    let filter_block = Block::default()
        .title("Filters (f)")
        .borders(Borders::ALL)
        .border_style(Style::new().fg(if app.filter_input_mode {
            Color::Yellow
        } else {
            app.colors.footer_border_color
        }));

    let filter_paragraph = Paragraph::new(filter_text.clone())
        .block(filter_block)
        .style(filter_style)
        .alignment(Alignment::Left);

    f.render_widget(filter_paragraph, area);

    // Show cursor if in filter input mode
    if app.filter_input_mode {
        let cursor_x = area.x + 1 + app.filter_input.len() as u16;
        let cursor_y = area.y + 1;
        f.set_cursor_position((cursor_x, cursor_y));
    }
}

fn render_filter_and_search_sections(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    // Split the area horizontally for filter and search
    let filter_search_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage(50), // Filter section
                Constraint::Percentage(50), // Search section
            ]
            .as_ref(),
        )
        .split(area);

    // Render filter section
    let filter_text = if app.filter_input.is_empty() {
        "Enter comma-separated filters (e.g., mp4,youtube,2024)".to_string()
    } else {
        app.filter_input.clone()
    };

    let filter_style = if app.filter_input_mode {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(app.colors.row_fg)
    };

    let filter_block = Block::default()
        .title("Filters (f)")
        .borders(Borders::ALL)
        .border_style(Style::new().fg(if app.filter_input_mode {
            Color::Yellow
        } else {
            app.colors.footer_border_color
        }));

    let filter_paragraph = Paragraph::new(filter_text.clone())
        .block(filter_block)
        .style(filter_style)
        .alignment(Alignment::Left);

    f.render_widget(filter_paragraph, filter_search_chunks[0]);

    // Show cursor if in filter input mode
    if app.filter_input_mode {
        let cursor_x = filter_search_chunks[0].x + 1 + app.filter_input.len() as u16;
        let cursor_y = filter_search_chunks[0].y + 1;
        f.set_cursor_position((cursor_x, cursor_y));
    }

    // Render search section
    let search_text = if app.search_input.is_empty() {
        "Enter search terms to search within transcripts".to_string()
    } else {
        app.search_input.clone()
    };

    let search_style = if app.search_input_mode {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(app.colors.row_fg)
    };

    let search_block = Block::default()
        .title("Search (/)")
        .borders(Borders::ALL)
        .border_style(Style::new().fg(if app.search_input_mode {
            Color::Green
        } else {
            app.colors.footer_border_color
        }));

    let search_paragraph = Paragraph::new(search_text.clone())
        .block(search_block)
        .style(search_style)
        .alignment(Alignment::Left);

    f.render_widget(search_paragraph, filter_search_chunks[1]);

    // Show cursor if in search input mode
    if app.search_input_mode {
        let cursor_x = filter_search_chunks[1].x + 1 + app.search_input.len() as u16;
        let cursor_y = filter_search_chunks[1].y + 1;
        f.set_cursor_position((cursor_x, cursor_y));
    }
}
