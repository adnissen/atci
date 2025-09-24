use crate::{files, config, search, clipper};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen, SetTitle},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style, Stylize},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame, Terminal,
};
use std::{error::Error, fs, io, time::{Duration, Instant}};

struct TableColors {
    buffer_bg: Color,
    header_bg: Color,
    header_fg: Color,
    row_fg: Color,
    selected_style_fg: Color,
    normal_row_color: Color,
    alt_row_color: Color,
    footer_border_color: Color,
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
enum SortOrder {
    Ascending,
    Descending,
}

#[derive(Clone, Copy, PartialEq)]
enum TabState {
    Transcripts,
    System,
    SearchResults,
}

struct App {
    state: TableState,
    colors: TableColors,
    video_data: Vec<files::VideoInfo>,
    sort_column: Option<usize>,
    sort_order: SortOrder,
    last_refresh: Instant,
    terminal_height: u16,
    current_page: u32,
    total_pages: u32,
    current_tab: TabState,
    filter_input: String,
    filter_input_mode: bool,
    search_input: String,
    search_input_mode: bool,
    system_selected_index: usize,
    system_services: Vec<SystemService>,
    last_system_refresh: Instant,
    search_results: Vec<search::SearchResult>,
    search_selected_index: usize,
    last_search_query: String,
    search_scroll_offset: usize,
}

#[derive(Clone)]
struct SystemService {
    name: String,
    status: ServiceStatus,
    pids: Vec<u32>,
}

#[derive(Clone)]
enum ServiceStatus {
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
            last_refresh: Instant::now(),
            terminal_height: 24,
            current_page: 0,
            total_pages: 1,
            current_tab: TabState::Transcripts,
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
        }
    }
}

impl App {
    fn new() -> Result<App, Box<dyn Error>> {
        let terminal_height = crossterm::terminal::size()?.1;
        let page_size = Self::calculate_page_size(terminal_height);

        // Use database sorting instead of client-side sorting
        let cache_data = files::load_sorted_paginated_cache_data(
            None,        // filter (no filter on initial load)
            0,           // page (first page)
            page_size,   // limit
            "last_generated", // sort by Generated At
            0,           // sort_order (0 = DESC, 1 = ASC)
        )?;

        let mut app = App {
            state: TableState::default(),
            colors: TableColors::new(&tailwind::BLUE),
            video_data: cache_data.files,
            sort_column: Some(2), // Generated At column
            sort_order: SortOrder::Descending,
            last_refresh: Instant::now(),
            terminal_height,
            current_page: 0,
            total_pages: cache_data.pages.unwrap_or(1),
            current_tab: TabState::Transcripts,
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
        };

        // Select first item if available
        if !app.video_data.is_empty() {
            app.state.select(Some(0));
        }

        // Initialize system services
        app.refresh_system_services();

        Ok(app)
    }

    fn calculate_page_size(terminal_height: u16) -> u32 {
        // Account for: margins (2), header (1), controls (3), table header (1), borders (2)
        // Leave some buffer for safety
        let available_height = terminal_height.saturating_sub(9);
        std::cmp::max(available_height as u32, 5) + 1 // Minimum 5 rows
    }

    fn get_page_size(&self) -> u32 {
        Self::calculate_page_size(self.terminal_height)
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.video_data.len().saturating_sub(1) {
                    // Reached bottom, try to load next page
                    if self.current_page < self.total_pages.saturating_sub(1) {
                        if let Err(e) = self.load_next_page() {
                            eprintln!("Failed to load next page: {}", e);
                        }
                        return; // Selection will be set in load_next_page
                    }
                    // If no more pages, stop
                    return;
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    // Reached top, try to load previous page
                    if self.current_page > 0 {
                        if let Err(e) = self.load_previous_page() {
                            eprintln!("Failed to load previous page: {}", e);
                        }
                        return; // Selection will be set in load_previous_page
                    }
                    // If no previous page (page 1), stay at top - don't wrap
                    0
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn sort_by_column(&mut self, column_index: usize) {
        if column_index >= 6 {
            return; // Invalid column
        }

        // Cycle sort order: None -> Asc -> Desc -> None
        if let Some(current_column) = self.sort_column {
            if current_column == column_index {
                // Same column, cycle sort order
                match self.sort_order {
                    SortOrder::Ascending => self.sort_order = SortOrder::Descending,
                    SortOrder::Descending => {
                        // Reset to default sort (Generated At descending)
                        self.sort_column = Some(2); // Generated At column
                        self.sort_order = SortOrder::Descending;
                    }
                }
            } else {
                // Different column, start with ascending
                self.sort_column = Some(column_index);
                self.sort_order = SortOrder::Ascending;
            }
        } else {
            // No current sort, start with ascending
            self.sort_column = Some(column_index);
            self.sort_order = SortOrder::Ascending;
        }

        // Reload data with new sorting from database
        if let Err(e) = self.reload_with_current_sort() {
            eprintln!("Failed to reload data with new sort: {}", e);
        }
    }

    fn reload_with_current_sort(&mut self) -> Result<(), Box<dyn Error>> {
        let (sort_by, sort_order) = self.get_sort_params();
        let page_size = self.get_page_size();

        let cache_data = files::load_sorted_paginated_cache_data(
            self.get_filter_option().as_ref(), // filter
            0,           // page (reset to first page when sorting)
            page_size,   // limit
            sort_by,     // sort column
            sort_order,  // sort order
        )?;

        self.video_data = cache_data.files;
        self.current_page = 0;
        self.total_pages = cache_data.pages.unwrap_or(1);

        // Reset selection to first item when sorting changes
        if !self.video_data.is_empty() {
            self.state.select(Some(0));
        }

        Ok(())
    }

    fn load_next_page(&mut self) -> Result<(), Box<dyn Error>> {
        if self.current_page >= self.total_pages.saturating_sub(1) {
            return Ok(()); // Already at last page
        }

        let next_page = self.current_page + 1;
        self.load_page(next_page)?;

        // Select first item of new page (for automatic page loading when scrolling)
        if !self.video_data.is_empty() {
            self.state.select(Some(0));
        }

        Ok(())
    }

    fn load_previous_page(&mut self) -> Result<(), Box<dyn Error>> {
        if self.current_page == 0 {
            return Ok(()); // Already at first page
        }

        let prev_page = self.current_page - 1;
        self.load_page(prev_page)?;

        // Select last item of new page (for automatic page loading when scrolling)
        if !self.video_data.is_empty() {
            self.state.select(Some(self.video_data.len() - 1));
        }

        Ok(())
    }

    fn load_next_page_preserve_cursor(&mut self) -> Result<(), Box<dyn Error>> {
        if self.current_page >= self.total_pages.saturating_sub(1) {
            return Ok(()); // Already at last page
        }

        // Remember current row position
        let current_row = self.state.selected().unwrap_or(0);

        let next_page = self.current_page + 1;
        self.load_page(next_page)?;

        // Try to keep same row position, or select last available row
        if !self.video_data.is_empty() {
            let target_row = if current_row < self.video_data.len() {
                current_row
            } else {
                self.video_data.len() - 1
            };
            self.state.select(Some(target_row));
        }

        Ok(())
    }

    fn load_previous_page_preserve_cursor(&mut self) -> Result<(), Box<dyn Error>> {
        if self.current_page == 0 {
            return Ok(()); // Already at first page
        }

        // Remember current row position
        let current_row = self.state.selected().unwrap_or(0);

        let prev_page = self.current_page - 1;
        self.load_page(prev_page)?;

        // Try to keep same row position, or select last available row
        if !self.video_data.is_empty() {
            let target_row = if current_row < self.video_data.len() {
                current_row
            } else {
                self.video_data.len() - 1
            };
            self.state.select(Some(target_row));
        }

        Ok(())
    }

    fn load_page(&mut self, page: u32) -> Result<(), Box<dyn Error>> {
        let (sort_by, sort_order) = self.get_sort_params();
        let page_size = self.get_page_size();

        let cache_data = files::load_sorted_paginated_cache_data(
            self.get_filter_option().as_ref(), // filter
            page,        // specific page
            page_size,   // limit
            sort_by,     // sort column
            sort_order,  // sort order
        )?;

        self.video_data = cache_data.files;
        self.current_page = page;
        self.total_pages = cache_data.pages.unwrap_or(1);

        Ok(())
    }

    fn refresh_data(&mut self) -> Result<(), Box<dyn Error>> {
        // Get currently selected item for preservation
        let selected_path = self.state.selected()
            .and_then(|i| self.video_data.get(i))
            .map(|v| v.full_path.clone());

        // Update disk cache and reload data with current sorting
        files::get_and_save_video_info_from_disk()?;

        let (sort_by, sort_order) = self.get_sort_params();
        let page_size = self.get_page_size();

        let cache_data = files::load_sorted_paginated_cache_data(
            self.get_filter_option().as_ref(), // filter
            self.current_page, // current page
            page_size,   // limit
            sort_by,     // sort column
            sort_order,  // sort order
        )?;

        self.video_data = cache_data.files;
        self.total_pages = cache_data.pages.unwrap_or(1);

        // Restore selection if possible
        if let Some(path) = selected_path {
            if let Some(new_index) = self.video_data.iter().position(|v| v.full_path == path) {
                self.state.select(Some(new_index));
            } else {
                // If selected item no longer exists, select first item
                if !self.video_data.is_empty() {
                    self.state.select(Some(0));
                }
            }
        }

        self.last_refresh = Instant::now();
        Ok(())
    }

    fn get_sort_params(&self) -> (&str, u8) {
        let sort_by = if let Some(column) = self.sort_column {
            match column {
                0 => "base_name",      // Filename
                1 => "created_at",     // Created At
                2 => "last_generated", // Generated At
                3 => "line_count",     // Lines
                4 => "length",         // Length
                5 => "source",         // Source
                _ => "last_generated", // Default fallback
            }
        } else {
            "last_generated" // Default
        };

        let sort_order = match self.sort_order {
            SortOrder::Ascending => 1,  // ASC
            SortOrder::Descending => 0, // DESC
        };

        (sort_by, sort_order)
    }

    fn should_refresh(&self) -> bool {
        self.last_refresh.elapsed() >= Duration::from_secs(60)
    }

    pub fn next_page(&mut self) {
        if let Err(e) = self.load_next_page_preserve_cursor() {
            eprintln!("Failed to load next page: {}", e);
        }
    }

    pub fn prev_page(&mut self) {
        if let Err(e) = self.load_previous_page_preserve_cursor() {
            eprintln!("Failed to load previous page: {}", e);
        }
    }

    pub fn toggle_tab(&mut self) {
        self.current_tab = match self.current_tab {
            TabState::Transcripts => TabState::System,
            TabState::System => {
                if !self.search_results.is_empty() {
                    TabState::SearchResults
                } else {
                    TabState::Transcripts
                }
            },
            TabState::SearchResults => TabState::Transcripts,
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
    }

    pub fn jump_to_top_of_page(&mut self) {
        if !self.video_data.is_empty() {
            self.state.select(Some(0));
        }
    }

    pub fn jump_to_bottom_of_page(&mut self) {
        if !self.video_data.is_empty() {
            self.state.select(Some(self.video_data.len() - 1));
        }
    }

    pub fn jump_to_first_page(&mut self) {
        if let Err(e) = self.load_page(0) {
            eprintln!("Failed to load first page: {}", e);
            return;
        }
        if !self.video_data.is_empty() {
            self.state.select(Some(0));
        }
    }

    pub fn jump_to_last_page(&mut self) {
        let last_page = self.total_pages.saturating_sub(1);
        if let Err(e) = self.load_page(last_page) {
            eprintln!("Failed to load last page: {}", e);
            return;
        }
        if !self.video_data.is_empty() {
            self.state.select(Some(self.video_data.len() - 1));
        }
    }

    pub fn toggle_filter_input(&mut self) {
        self.filter_input_mode = !self.filter_input_mode;
    }

    pub fn clear_filter(&mut self) {
        self.filter_input.clear();
        self.filter_input_mode = false;
        // Reload data without filter
        if let Err(e) = self.reload_with_current_sort() {
            eprintln!("Failed to reload data after clearing filter: {}", e);
        }
    }

    pub fn apply_filter(&mut self) {
        self.filter_input_mode = false;
        // Reload data with current filter
        if let Err(e) = self.reload_with_current_sort() {
            eprintln!("Failed to reload data with filter: {}", e);
        }
    }

    pub fn add_char_to_filter(&mut self, c: char) {
        if self.filter_input_mode {
            self.filter_input.push(c);
            // Refresh table immediately as user types
            if let Err(e) = self.reload_with_current_sort() {
                eprintln!("Failed to reload data while typing filter: {}", e);
            }
        }
    }

    pub fn remove_char_from_filter(&mut self) {
        if self.filter_input_mode {
            self.filter_input.pop();
            // Refresh table immediately as user types
            if let Err(e) = self.reload_with_current_sort() {
                eprintln!("Failed to reload data while typing filter: {}", e);
            }   
        }
    }

    fn get_filter_option(&self) -> Option<Vec<String>> {
        if self.filter_input.is_empty() {
            None
        } else {
            Some(
                self.filter_input
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            )
        }
    }

    pub fn toggle_search_input(&mut self) {
        self.search_input_mode = !self.search_input_mode;
    }

    pub fn clear_search(&mut self) {
        self.search_input.clear();
        self.search_input_mode = false;
        // Placeholder - will implement search functionality later
    }

    pub fn apply_search(&mut self) {
        self.search_input_mode = false;
        // Placeholder - will implement search functionality later
        self.perform_search();
    }

    pub fn add_char_to_search(&mut self, c: char) {
        if self.search_input_mode {
            self.search_input.push(c);
        }
    }

    pub fn remove_char_from_search(&mut self) {
        if self.search_input_mode {
            self.search_input.pop();
        }
    }

    fn perform_search(&mut self) {
        // Perform actual search functionality
        if !self.search_input.is_empty() {
            match search::search(&self.search_input, self.get_filter_option().as_ref(), false, false) {
                Ok(results) => {
                    self.search_results = results;
                    self.search_selected_index = 0;
                    self.search_scroll_offset = 0;
                    self.last_search_query = self.search_input.clone();
                    
                    // Switch to search results tab
                    self.current_tab = TabState::SearchResults;
                }
                Err(e) => {
                    eprintln!("Search failed: {}", e);
                    // Keep search results empty on error
                    self.search_results.clear();
                }
            }
        }
    }

    pub fn system_next(&mut self) {
        if !self.system_services.is_empty() && self.system_selected_index < self.system_services.len() - 1 {
            self.system_selected_index += 1;
        }
    }

    pub fn system_previous(&mut self) {
        if self.system_selected_index > 0 {
            self.system_selected_index -= 1;
        }
    }

    pub fn search_next(&mut self) {
        let total_matches: usize = self.search_results.iter().map(|r| r.matches.len()).sum();
        if total_matches > 0 && self.search_selected_index < total_matches - 1 {
            self.search_selected_index += 1;
            self.update_search_scroll();
        }
    }

    pub fn search_previous(&mut self) {
        if self.search_selected_index > 0 {
            self.search_selected_index -= 1;
            self.update_search_scroll();
        }
    }

    fn update_search_scroll(&mut self) {
        // Calculate the line position of the currently selected match
        let selected_line = self.get_selected_match_line_position();
        
        // Assume we have about 10 visible lines in the search results area
        // (this will be adjusted based on actual terminal height)
        let visible_lines = self.get_search_visible_lines();
        
        // Scroll up if selection is above visible area
        if selected_line < self.search_scroll_offset {
            self.search_scroll_offset = selected_line;
        }
        // Scroll down if selection is below visible area
        else if selected_line >= self.search_scroll_offset + visible_lines {
            self.search_scroll_offset = selected_line.saturating_sub(visible_lines - 1);
        }
    }

    fn get_selected_match_line_position(&self) -> usize {
        let mut current_line = 0;
        let mut current_match_index = 0;

        for result in &self.search_results {
            // File header line
            current_line += 1;

            // Match lines
            for _ in &result.matches {
                if current_match_index == self.search_selected_index {
                    return current_line;
                }
                current_line += 1;
                current_match_index += 1;
            }

            // Empty line between files
            current_line += 1;
        }

        current_line
    }

    fn get_search_visible_lines(&self) -> usize {
        // Calculate available height for search results
        // Terminal height minus: margins (2), header (1), controls (3), search query section (3), borders (2)
        let available_height = self.terminal_height.saturating_sub(11);
        std::cmp::max(available_height as usize, 5)
    }

    fn get_selected_search_match(&self) -> Option<&search::SearchMatch> {
        let mut current_match_index = 0;

        for result in &self.search_results {
            for search_match in &result.matches {
                if current_match_index == self.search_selected_index {
                    return Some(search_match);
                }
                current_match_index += 1;
            }
        }

        None
    }

    pub fn create_clip_from_selected_match(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(search_match) = self.get_selected_search_match() {
            if let Some(timestamp) = &search_match.timestamp {
                // Parse timestamp from line like "126: 00:05:25.920 --> 00:05:46.060"
                if let Some((start_time, end_time)) = self.parse_timestamp_range(timestamp) {
                    let video_path = std::path::Path::new(&search_match.video_info.full_path);
                    
                    // Create clip with the match text as overlay
                    let clip_path = clipper::clip(
                        video_path,
                        &start_time,
                        &end_time,
                        Some(&search_match.line_text), // Use the matched text for overlay
                        false,                          // Display text
                        "mp4",                         // Format
                        None,                          // Use default font size
                    )?;

                    // Open the clip
                    self.open_file(&clip_path)?;
                } else {
                    return Err("Could not parse timestamp from search result".into());
                }
            } else {
                return Err("Selected search result has no timestamp".into());
            }
        } else {
            return Err("No search result selected".into());
        }

        Ok(())
    }

    fn parse_timestamp_range(&self, timestamp_line: &str) -> Option<(String, String)> {
        // Parse lines like "51: 00:01:07.220 --> 00:01:10.680" or "00:01:07.220 --> 00:01:10.680"
        
        // First check if line contains the arrow separator
        if let Some(_arrow_pos) = timestamp_line.find(" --> ") {
            // Check if it has a number prefix (subtitle format): "51: 00:01:07.220 --> 00:01:10.680"
            if let Some(colon_pos) = timestamp_line.find(": ") {
                let timestamp_part = &timestamp_line[colon_pos + 2..];
                let start_end: Vec<&str> = timestamp_part.split(" --> ").collect();
                if start_end.len() == 2 {
                    return Some((start_end[0].to_string(), start_end[1].to_string()));
                }
            } else {
                // Direct format: "00:01:07.220 --> 00:01:10.680"
                let start_end: Vec<&str> = timestamp_line.split(" --> ").collect();
                if start_end.len() == 2 {
                    return Some((start_end[0].to_string(), start_end[1].to_string()));
                }
            }
        }
        None
    }

    fn open_file(&self, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open")
                .arg(path)
                .spawn()?;
        }

        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("cmd")
                .args(["/C", "start", ""])
                .arg(path)
                .spawn()?;
        }

        #[cfg(target_os = "linux")]
        {
            std::process::Command::new("xdg-open")
                .arg(path)
                .spawn()?;
        }

        Ok(())
    }

    pub fn refresh_system_services(&mut self) {
        self.system_services = get_system_services();
        // Ensure selection is within bounds
        if self.system_selected_index >= self.system_services.len() && !self.system_services.is_empty() {
            self.system_selected_index = self.system_services.len() - 1;
        }
        self.last_system_refresh = Instant::now();
    }

    fn should_refresh_system_services(&self) -> bool {
        self.last_system_refresh.elapsed() >= Duration::from_secs(1)
    }

    pub fn kill_selected_service(&mut self) -> Result<(), Box<dyn Error>> {
        if self.system_selected_index < self.system_services.len() {
            let service = &self.system_services[self.system_selected_index];
            if !service.pids.is_empty() {
                let pid = service.pids[0]; // Kill first PID for now
                kill_process(pid)?;
                // Delete the associated PID file
                if let Err(e) = delete_pid_file(pid) {
                    eprintln!("Warning: Failed to delete PID file for {}: {}", pid, e);
                }
                // Refresh services after killing
                self.refresh_system_services();
            }
        }
        Ok(())
    }

    pub fn start_selected_service(&mut self) -> Result<(), Box<dyn Error>> {
        if self.system_selected_index < self.system_services.len() {
            let service = &self.system_services[self.system_selected_index];
            match service.status {
                ServiceStatus::Stopped => {
                    // Start the watcher service using the same logic as in ensure_watcher_running
                    start_watcher_process()?;
                    // Refresh services after starting
                    self.refresh_system_services();
                }
                ServiceStatus::Active => {
                    // Service is already running, nothing to do
                }
            }
        }
        Ok(())
    }
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        // Ensure file watcher is running before starting TUI
        if let Err(e) = ensure_watcher_running().await {
            eprintln!("Warning: Failed to ensure watcher is running: {}", e);
        }

        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, SetTitle("atci"), EnterAlternateScreen, EnableMouseCapture)?;
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

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<(), Box<dyn Error>>
where
    <B as Backend>::Error: 'static,
{
    loop {
        terminal.draw(|f| ui(f, app))?;

        // Check if we should refresh data (for transcripts tab)
        if app.should_refresh() {
            if let Err(e) = app.refresh_data() {
                eprintln!("Failed to refresh data: {}", e);
            }
        }
        
        // Refresh system services every second
        if app.should_refresh_system_services() {
            app.refresh_system_services();
        }

        // Use poll to avoid blocking and allow periodic refreshes
        if event::poll(Duration::from_millis(1000))? {
            if let Event::Key(key) = event::read()? {
                // Handle filter input mode
                if app.filter_input_mode {
                    match key.code {
                        KeyCode::Esc => app.filter_input_mode = false,
                        KeyCode::Enter => app.apply_filter(),
                        KeyCode::Backspace => app.remove_char_from_filter(),
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.clear_filter();
                        },
                        KeyCode::Char(c) => app.add_char_to_filter(c),
                        _ => {}
                    }
                    continue;
                }

                // Handle search input mode
                if app.search_input_mode {
                    match key.code {
                        KeyCode::Esc => app.search_input_mode = false,
                        KeyCode::Enter => app.apply_search(),
                        KeyCode::Backspace => app.remove_char_from_search(),
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.clear_search();
                        },
                        KeyCode::Char(c) => app.add_char_to_search(c),
                        _ => {}
                    }
                    continue;
                }

                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Tab => app.toggle_tab(),
                    KeyCode::Char('t') => app.switch_to_transcripts(),
                    KeyCode::Char('s') => app.switch_to_system(),
                    KeyCode::Char('r') => {
                        // Only switch to search results if we have results
                        if !app.search_results.is_empty() {
                            app.switch_to_search_results();
                        }
                    },
                    KeyCode::Char('f') => {
                        if app.current_tab == TabState::Transcripts {
                            app.toggle_filter_input();
                        }
                    },
                    KeyCode::Char('/') => {
                        if app.current_tab == TabState::Transcripts {
                            app.toggle_search_input();
                        }
                    },
                    KeyCode::Char('c') => {
                        if app.current_tab == TabState::Transcripts && key.modifiers.contains(KeyModifiers::CONTROL) {
                            app.clear_filter();
                            app.clear_search();
                        } else if app.current_tab == TabState::SearchResults {
                            // Create clip from selected search result
                            if let Err(e) = app.create_clip_from_selected_match() {
                                eprintln!("Failed to create clip: {}", e);
                            }
                        }
                    },
                    KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                        if app.current_tab == TabState::Transcripts {
                            if key.code == KeyCode::Char('J') || key.modifiers.contains(KeyModifiers::SHIFT) {
                                app.jump_to_bottom_of_page();
                            } else {
                                app.next();
                            }
                        } else if app.current_tab == TabState::System {
                            app.system_next();
                        } else if app.current_tab == TabState::SearchResults {
                            app.search_next();
                        }
                    },
                    KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K')=> {
                        if app.current_tab == TabState::Transcripts {
                            if key.code == KeyCode::Char('K') || key.modifiers.contains(KeyModifiers::SHIFT) {
                                app.jump_to_top_of_page();
                            } else {
                                app.previous();
                            }
                        } else if app.current_tab == TabState::System {
                            app.system_previous();
                        } else if app.current_tab == TabState::SearchResults {
                            app.search_previous();
                        }
                    },
                    KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('H') => {
                        if app.current_tab == TabState::Transcripts {
                            if key.code == KeyCode::Char('H') || key.modifiers.contains(KeyModifiers::SHIFT) {
                                app.jump_to_first_page();
                            } else {
                                app.prev_page();
                            }
                        }
                    },
                    KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('L') => {
                        if app.current_tab == TabState::Transcripts {
                            if key.code == KeyCode::Char('L') || key.modifiers.contains(KeyModifiers::SHIFT) {
                                app.jump_to_last_page();
                            } else {
                                app.next_page();
                            }
                        }
                    },
                    KeyCode::Enter => {
                        if app.current_tab == TabState::System {
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
                    },
                    KeyCode::Char('1') => {
                        if app.current_tab == TabState::Transcripts {
                            app.sort_by_column(0);
                        }
                    },
                    KeyCode::Char('2') => {
                        if app.current_tab == TabState::Transcripts {
                            app.sort_by_column(1);
                        }
                    },
                    KeyCode::Char('3') => {
                        if app.current_tab == TabState::Transcripts {
                            app.sort_by_column(2);
                        }
                    },
                    KeyCode::Char('4') => {
                        if app.current_tab == TabState::Transcripts {
                            app.sort_by_column(3);
                        }
                    },
                    KeyCode::Char('5') => {
                        if app.current_tab == TabState::Transcripts {
                            app.sort_by_column(4);
                        }
                    },
                    KeyCode::Char('6') => {
                        if app.current_tab == TabState::Transcripts {
                            app.sort_by_column(5);
                        }
                    },
                    _ => {}
                }
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let chunks = if app.current_tab == TabState::Transcripts {
        Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Min(3),        // Content area
                Constraint::Length(3),     // Filter area
                Constraint::Length(3),     // Bottom panes area
            ].as_ref())
            .split(f.area())
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Min(3),        // Content area
                Constraint::Length(3),     // Bottom panes area
            ].as_ref())
            .split(f.area())
    };

    // Split the bottom area into Controls and Page panes (only for Transcripts tab)
    let bottom_chunks = if app.current_tab == TabState::Transcripts {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(90), // Controls area (9/10)
                Constraint::Percentage(10), // Page area (1/10)
            ].as_ref())
            .split(chunks[2]) // Use index 2 since we added filter area
    } else {
        // For System tab, use full width for controls
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(100), // Controls area takes full width
            ].as_ref())
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
        TabState::Transcripts => render_transcripts_tab(f, chunks[0], app, header_style, selected_row_style),
        TabState::System => render_system_tab(f, chunks[0], app),
        TabState::SearchResults => render_search_results_tab(f, chunks[0], app),
    }

    // Render filter and search sections (only on transcripts tab)
    if app.current_tab == TabState::Transcripts {
        render_filter_and_search_sections(f, chunks[1], app);
    }

    // Controls section
    let controls_text = match app.current_tab {
        TabState::Transcripts => {
            if app.filter_input_mode {
                "Enter: Apply  Esc: Cancel  Ctrl+C: Clear  Type to filter...".to_string()
            } else if app.search_input_mode {
                "Enter: Search  Esc: Cancel  Ctrl+C: Clear  Type to search...".to_string()
            } else {
                let base_controls = "↑↓/jk: Navigate  ←→/hl: Page  1-6: Sort  f: Filter  /: Search  Ctrl+C: Clear  t/s";
                if !app.search_results.is_empty() {
                    format!("{}r/Tab: Switch  q: Quit", base_controls)
                } else {
                    format!("{}/Tab: Switch  q: Quit", base_controls)
                }
            }
        },
        TabState::System => {
            let base_controls = "↑↓/jk: Navigate  Enter: Start/Kill Process  t/s";
            if !app.search_results.is_empty() {
                format!("{}r/Tab: Switch  q: Quit", base_controls)
            } else {
                format!("{}/Tab: Switch  q: Quit", base_controls)
            }
        },
        TabState::SearchResults => "↑↓/jk: Navigate  c: Create Clip  t/sr/Tab: Switch  q: Quit".to_string(),
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

fn create_tab_title(current_tab: TabState, colors: &TableColors, has_search_results: bool) -> ratatui::text::Line<'_> {
    use ratatui::text::{Span, Line};
    use ratatui::style::Color;

    let mut spans = vec![
        match current_tab {
            TabState::Transcripts => Span::styled("Transcripts (t)", Style::default().fg(Color::White)),
            _ => Span::styled("Transcripts (t)", Style::default().fg(colors.footer_border_color)),
        },
        Span::styled(" | ", Style::default().fg(colors.row_fg)),
        match current_tab {
            TabState::System => Span::styled("System (s)", Style::default().fg(Color::White)),
            _ => Span::styled("System (s)", Style::default().fg(colors.footer_border_color)),
        },
    ];

    // Only show search results tab if we have results
    if has_search_results {
        spans.push(Span::styled(" | ", Style::default().fg(colors.row_fg)));
        spans.push(match current_tab {
            TabState::SearchResults => Span::styled("Search Results (r)", Style::default().fg(Color::White)),
            _ => Span::styled("Search Results (r)", Style::default().fg(colors.footer_border_color)),
        });
    }

    Line::from(spans)
}

fn render_transcripts_tab(f: &mut Frame, area: ratatui::layout::Rect, app: &mut App, header_style: Style, selected_row_style: Style) {
    let title = create_tab_title(app.current_tab, &app.colors, !app.search_results.is_empty());
    let headers = ["Filename", "Created At", "Generated At", "Lines", "Length", "Source"];
    let header_cells: Vec<Cell> = headers
        .iter()
        .enumerate()
        .map(|(i, &title)| {
            let mut content = format!("{} ({})", title, i + 1);

            // Add sort indicator if this column is being sorted
            if let Some(sort_col) = app.sort_column {
                if sort_col == i {
                    let indicator = match app.sort_order {
                        SortOrder::Ascending => " ↑",
                        SortOrder::Descending => " ↓",
                    };
                    content.push_str(indicator);
                }
            }

            Cell::from(content)
        })
        .collect();

    let header = Row::new(header_cells)
        .style(header_style)
        .height(1);

    let rows = if app.video_data.is_empty() {
        // Show empty state
        vec![Row::new(vec![
            Cell::from("No video files found"),
            Cell::from(""),
            Cell::from(""),
            Cell::from(""),
            Cell::from(""),
            Cell::from(""),
        ]).style(Style::new().fg(app.colors.row_fg).bg(app.colors.normal_row_color))]
    } else {
        app.video_data.iter().enumerate().map(|(i, video)| {
            let color = match i % 2 {
                0 => app.colors.normal_row_color,
                _ => app.colors.alt_row_color,
            };

            // Format the data to match our table columns and create Row directly
            Row::new(vec![
                Cell::from(video.base_name.as_str()),
                Cell::from(video.created_at.split(' ').next().unwrap_or(&video.created_at)),
                Cell::from(video.last_generated.as_ref()
                    .map(|dt| dt.split(' ').next().unwrap_or(dt))
                    .unwrap_or("-")),
                Cell::from(video.line_count.to_string()),
                Cell::from(video.length.as_deref().unwrap_or("-")),
                Cell::from(video.source.as_deref().unwrap_or("-")),
            ])
            .style(Style::new().fg(app.colors.row_fg).bg(color))
            .height(1)
        }).collect()
    };

    let t = Table::new(
        rows,
        [
            Constraint::Percentage(25), // Filename
            Constraint::Percentage(15), // Created At
            Constraint::Percentage(15), // Generated At
            Constraint::Percentage(10), // Lines
            Constraint::Percentage(10), // Length
            Constraint::Percentage(25), // Source
        ]
    )
        .header(header)
        .bg(app.colors.buffer_bg)
        .row_highlight_style(selected_row_style)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::new().fg(app.colors.footer_border_color)),
        );
    f.render_stateful_widget(t, area, &mut app.state);
}

fn get_atci_dir() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let home_dir = dirs::home_dir().ok_or("Could not find home directory")?;
    let atci_dir = home_dir.join(".atci");
    Ok(atci_dir)
}

fn find_existing_pid_files() -> Result<Vec<u32>, Box<dyn std::error::Error>> {
    let atci_dir = get_atci_dir()?;
    let config_sha = config::get_config_path_sha();
    let mut pids = Vec::new();

    if atci_dir.exists() {
        for entry in fs::read_dir(atci_dir)? {
            let entry = entry?;
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();

            let expected_prefix = format!("atci.{}.", config_sha);
            if file_name_str.starts_with(&expected_prefix) && file_name_str.ends_with(".pid") {
                let pid_str = &file_name_str[expected_prefix.len()..file_name_str.len() - 4]; // Remove prefix and ".pid" suffix
                if let Ok(pid) = pid_str.parse::<u32>() {
                    pids.push(pid);
                }
            }
        }
    }

    Ok(pids)
}

fn is_process_running(pid: u32) -> bool {
    #[cfg(unix)]
    {
        use std::process::Command;
        let output = Command::new("ps").arg("-p").arg(pid.to_string()).output();

        match output {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }

    #[cfg(windows)]
    {
        use std::process::Command;
        let output = Command::new("tasklist")
            .arg("/FI")
            .arg(format!("PID eq {}", pid))
            .output();

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                stdout.contains(&pid.to_string())
            }
            Err(_) => false,
        }
    }
}

fn get_system_services() -> Vec<SystemService> {
    let mut services = Vec::new();

    match find_existing_pid_files() {
        Ok(pids) => {
            let running_pids: Vec<u32> = pids.into_iter()
                .filter(|&pid| is_process_running(pid))
                .collect();

            if !running_pids.is_empty() {
                services.push(SystemService {
                    name: "File Watcher".to_string(),
                    status: ServiceStatus::Active,
                    pids: running_pids,
                });
            } else {
                services.push(SystemService {
                    name: "File Watcher".to_string(),
                    status: ServiceStatus::Stopped,
                    pids: Vec::new(),
                });
            }
        }
        Err(_) => {
            services.push(SystemService {
                name: "File Watcher".to_string(),
                status: ServiceStatus::Stopped,
                pids: Vec::new(),
            });
        }
    }

    services
}

fn kill_process(pid: u32) -> Result<(), Box<dyn Error>> {
    #[cfg(unix)]
    {
        use std::process::Command;
        let output = Command::new("kill")
            .arg(pid.to_string())
            .output()?;

        if !output.status.success() {
            return Err(format!("Failed to kill process {}: {}", pid, 
                String::from_utf8_lossy(&output.stderr)).into());
        }
    }

    #[cfg(windows)]
    {
        use std::process::Command;
        let output = Command::new("taskkill")
            .arg("/F")
            .arg("/PID")
            .arg(pid.to_string())
            .output()?;

        if !output.status.success() {
            return Err(format!("Failed to kill process {}: {}", pid,
                String::from_utf8_lossy(&output.stderr)).into());
        }
    }

    Ok(())
}

fn delete_pid_file(pid: u32) -> Result<(), Box<dyn Error>> {
    let atci_dir = get_atci_dir()?;
    let config_sha = config::get_config_path_sha();
    
    // Construct the expected PID file name
    let pid_file_name = format!("atci.{}.{}.pid", config_sha, pid);
    let pid_file_path = atci_dir.join(pid_file_name);
    
    // Only try to delete if the file exists
    if pid_file_path.exists() {
        fs::remove_file(pid_file_path)?;
    }
    
    Ok(())
}

fn start_watcher_process() -> Result<(), Box<dyn Error>> {
    // Get the current executable path
    let current_exe = std::env::current_exe()?;

    // Spawn a new atci watch process
    std::process::Command::new(&current_exe)
        .arg("watch")
        .spawn()?;

    Ok(())
}

async fn ensure_watcher_running() -> Result<(), Box<dyn Error>> {
    // Check if any watcher processes are currently running
    let running_pids: Vec<u32> = match find_existing_pid_files() {
        Ok(pids) => pids.into_iter()
            .filter(|&pid| is_process_running(pid))
            .collect(),
        Err(_) => vec![]
    };

    // If no watchers are running, start them
    if running_pids.is_empty() {
        println!("No file watcher processes detected. Starting standalone watcher...");

        // Get the current executable path
        let current_exe = std::env::current_exe()?;

        // Spawn a new atci watch process
        tokio::spawn(async move {
            let mut cmd = tokio::process::Command::new(&current_exe);
            cmd.arg("watch");

            match cmd.spawn() {
                Ok(mut child) => {
                    // Let it run in the background - don't wait for it
                    tokio::spawn(async move {
                        if let Err(e) = child.wait().await {
                            eprintln!("Watcher process exited with error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("Error spawning watcher process: {}", e);
                }
            }
        });
    }

    Ok(())
}

fn render_filter_and_search_sections(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    // Split the area horizontally for filter and search
    let filter_search_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50), // Filter section
            Constraint::Percentage(50), // Search section
        ].as_ref())
        .split(area);

    // Render filter section
    let filter_text = if app.filter_input.is_empty() {
        "Enter comma-separated filters (e.g., mp4,youtube,2024)".to_string()
    } else {
        app.filter_input.clone()
    };

    let filter_style = if app.filter_input_mode {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(app.colors.row_fg)
    };

    let filter_block = Block::default()
        .title("Filters (f)")
        .borders(Borders::ALL)
        .border_style(Style::new().fg(
            if app.filter_input_mode {
                Color::Yellow
            } else {
                app.colors.footer_border_color
            }
        ));

    let filter_paragraph = Paragraph::new(filter_text)
        .block(filter_block)
        .style(filter_style)
        .alignment(Alignment::Left);

    f.render_widget(filter_paragraph, filter_search_chunks[0]);

    // Render search section
    let search_text = if app.search_input.is_empty() {
        "Enter search terms to search within transcripts".to_string()
    } else {
        app.search_input.clone()
    };

    let search_style = if app.search_input_mode {
        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(app.colors.row_fg)
    };

    let search_block = Block::default()
        .title("Search (/)")
        .borders(Borders::ALL)
        .border_style(Style::new().fg(
            if app.search_input_mode {
                Color::Green
            } else {
                app.colors.footer_border_color
            }
        ));

    let search_paragraph = Paragraph::new(search_text)
        .block(search_block)
        .style(search_style)
        .alignment(Alignment::Left);

    f.render_widget(search_paragraph, filter_search_chunks[1]);
}

fn render_system_tab(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let title = create_tab_title(app.current_tab, &app.colors, !app.search_results.is_empty());

    // Split the main content area into sections
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Min(5),     // Services section (expandable)
            Constraint::Length(3),  // Additional info section
        ].as_ref())
        .split(area);

    // Create main block with tab title
    let main_block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::new().fg(app.colors.footer_border_color));

    f.render_widget(main_block, area);

    // Services section inside the main block
    let services_content = render_services_list(app);
    let services_paragraph = Paragraph::new(services_content)
        .block(
            Block::default()
                .title("Services (↑↓/jk: Navigate, Enter: Start/Kill)")
                .borders(Borders::ALL)
                .border_style(Style::new().fg(app.colors.footer_border_color))
        )
        .style(Style::new().fg(app.colors.row_fg))
        .alignment(Alignment::Left);

    f.render_widget(services_paragraph, main_chunks[0]);

    // Additional content area
    let additional_content = "Additional system information will be displayed here.";
    let additional_paragraph = Paragraph::new(additional_content)
        .style(Style::new().fg(app.colors.row_fg))
        .alignment(Alignment::Center);

    f.render_widget(additional_paragraph, main_chunks[1]);
}

fn render_services_list(app: &App) -> ratatui::text::Text<'static> {
    use ratatui::text::{Line, Span, Text};
    use ratatui::style::{Color, Style};

    let mut lines = Vec::new();

    for (index, service) in app.system_services.iter().enumerate() {
        let is_selected = index == app.system_selected_index;
        
        let mut spans = Vec::new();
        
        // Add selection indicator
        if is_selected {
            spans.push(Span::styled("► ", Style::default().fg(Color::Yellow)));
        } else {
            spans.push(Span::raw("  "));
        }
        
        // Service name
        spans.push(Span::raw(format!("{}: ", service.name)));
        
        // Status and PIDs
        match service.status {
            ServiceStatus::Active => {
                spans.push(Span::styled("active", Style::default().fg(Color::Green)));
                if !service.pids.is_empty() {
                    let pid_list = service.pids.iter()
                        .map(|pid| pid.to_string())
                        .collect::<Vec<_>>()
                        .join(", ");
                    spans.push(Span::raw(" (PID: "));
                    spans.push(Span::styled(pid_list, Style::default().fg(Color::Cyan)));
                    spans.push(Span::raw(")"));
                    
                    // Show kill option if selected
                    if is_selected {
                        spans.push(Span::raw(" "));
                        spans.push(Span::styled("← [KILL]", Style::default().fg(Color::Red).add_modifier(ratatui::style::Modifier::BOLD)));
                    }
                }
            }
            ServiceStatus::Stopped => {
                spans.push(Span::styled("stopped", Style::default().fg(Color::Red)));
                
                // Show start option if selected
                if is_selected {
                    spans.push(Span::raw(" "));
                    spans.push(Span::styled("← [START]", Style::default().fg(Color::Green).add_modifier(ratatui::style::Modifier::BOLD)));
                }
            }
        }
        
        lines.push(Line::from(spans));
    }

    if lines.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("No services found", Style::default().fg(Color::Gray))
        ]));
    }

    Text::from(lines)
}

fn render_search_results_tab(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let title = create_tab_title(app.current_tab, &app.colors, !app.search_results.is_empty());

    // Split the main content area into sections
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),  // Search query info
            Constraint::Min(5),     // Results section (expandable)
        ].as_ref())
        .split(area);

    // Create main block with tab title
    let main_block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::new().fg(app.colors.footer_border_color));

    f.render_widget(main_block, area);

    // Search query info section
    let query_info = if app.search_results.is_empty() {
        if app.last_search_query.is_empty() {
            "No search performed yet. Use '/' to search transcripts.".to_string()
        } else {
            format!("No results found for: '{}'", app.last_search_query)
        }
    } else {
        let total_matches: usize = app.search_results.iter().map(|r| r.matches.len()).sum();
        format!("Search: '{}' - {} matches in {} files", 
                app.last_search_query, 
                total_matches, 
                app.search_results.len())
    };

    let query_paragraph = Paragraph::new(query_info)
        .block(
            Block::default()
                .title("Search Query")
                .borders(Borders::ALL)
                .border_style(Style::new().fg(app.colors.footer_border_color))
        )
        .style(Style::new().fg(app.colors.row_fg))
        .alignment(Alignment::Center);

    f.render_widget(query_paragraph, main_chunks[0]);

    // Results section
    if !app.search_results.is_empty() {
        let results_content = render_search_results_list(app);
        let results_paragraph = Paragraph::new(results_content)
            .block(
                Block::default()
                    .title("Results (↑↓/jk: Navigate)")
                    .borders(Borders::ALL)
                    .border_style(Style::new().fg(app.colors.footer_border_color))
            )
            .style(Style::new().fg(app.colors.row_fg))
            .alignment(Alignment::Left)
            .wrap(ratatui::widgets::Wrap { trim: true })
            .scroll((app.search_scroll_offset as u16, 0));

        f.render_widget(results_paragraph, main_chunks[1]);
    } else {
        // Show empty state
        let empty_content = "Perform a search using '/' to see results here.";
        let empty_paragraph = Paragraph::new(empty_content)
            .style(Style::new().fg(app.colors.row_fg))
            .alignment(Alignment::Center);

        f.render_widget(empty_paragraph, main_chunks[1]);
    }
}

fn render_search_results_list(app: &App) -> ratatui::text::Text<'static> {
    use ratatui::text::{Line, Span, Text};
    use ratatui::style::{Color, Style};

    let mut lines = Vec::new();
    let mut current_match_index = 0;

    for result in &app.search_results {
        // File header
        lines.push(Line::from(vec![
            Span::styled(
                format!("📁 {}", result.file_path),
                Style::default().fg(Color::Cyan).add_modifier(ratatui::style::Modifier::BOLD)
            )
        ]));

        for search_match in &result.matches {
            let is_selected = current_match_index == app.search_selected_index;
            
            let mut spans = Vec::new();
            
            // Add selection indicator
            if is_selected {
                spans.push(Span::styled("► ", Style::default().fg(Color::Yellow)));
            } else {
                spans.push(Span::raw("  "));
            }
            
            // Line number
            spans.push(Span::styled(
                format!("L{}: ", search_match.line_number),
                Style::default().fg(Color::Gray)
            ));
            
            // Timestamp if available
            if let Some(timestamp) = &search_match.timestamp {
                spans.push(Span::styled(
                    format!("[{}] ", timestamp.trim()),
                    Style::default().fg(Color::Green)
                ));
            }
            
            // Match text (highlight the search term if possible)
            let line_text = &search_match.line_text;
            if is_selected {
                spans.push(Span::styled(
                    line_text.clone(),
                    Style::default().fg(Color::White).add_modifier(ratatui::style::Modifier::BOLD)
                ));
            } else {
                spans.push(Span::raw(line_text.clone()));
            }
            
            lines.push(Line::from(spans));
            current_match_index += 1;
        }

        // Add empty line between files for readability
        lines.push(Line::from(vec![Span::raw("")]));
    }

    if lines.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("No search results", Style::default().fg(Color::Gray))
        ]));
    }

    Text::from(lines)
}