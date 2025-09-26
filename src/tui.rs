use crate::{files, search};
use crate::transcripts_tab::render_transcripts_tab;
use crate::system_tab::{render_system_tab, find_existing_pid_files, is_process_running};
use crate::search_tab::render_search_results_tab;
use crate::editor_tab::{render_editor_tab, EditorData, FrameSelection};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen, SetTitle},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, TableState},
    Frame, Terminal,
};
use std::{error::Error, io, time::{Duration, Instant}};

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
    SearchResults,
    Editor,
}

pub struct App {
    pub state: TableState,
    pub colors: TableColors,
    pub video_data: Vec<files::VideoInfo>,
    pub sort_column: Option<usize>,
    pub sort_order: SortOrder,
    pub last_refresh: Instant,
    pub terminal_height: u16,
    pub current_page: u32,
    pub total_pages: u32,
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
    pub editor_data: Option<EditorData>,
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
            editor_data: None,
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
            editor_data: None,
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

    pub fn get_page_size(&self) -> u32 {
        Self::calculate_page_size(self.terminal_height)
    }









    fn should_refresh(&self) -> bool {
        self.last_refresh.elapsed() >= Duration::from_secs(60)
    }


    pub fn toggle_tab(&mut self) {
        self.current_tab = match self.current_tab {
            TabState::Transcripts => TabState::System,
            TabState::System => {
                if !self.search_results.is_empty() {
                    TabState::SearchResults
                } else if self.editor_data.is_some() {
                    TabState::Editor
                } else {
                    TabState::Transcripts
                }
            },
            TabState::SearchResults => {
                if self.editor_data.is_some() {
                    TabState::Editor
                } else {
                    TabState::Transcripts
                }
            },
            TabState::Editor => TabState::Transcripts,
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
        // Populate search input with the last search query for easy editing
        if !self.last_search_query.is_empty() {
            self.search_input = self.last_search_query.clone();
        }
    }


    pub fn toggle_filter_input(&mut self) {
        self.filter_input_mode = !self.filter_input_mode;
    }

    pub fn clear_filter(&mut self) {
        self.filter_input.clear();
        self.filter_input_mode = false;
        // Reload data without filter
        if self.current_tab == TabState::SearchResults {
            self.perform_search();
        } else {
            if let Err(e) = self.reload_with_current_sort() {
                eprintln!("Failed to reload data after clearing filter: {}", e);
            }
        }
    }

    pub fn apply_filter(&mut self) {
        self.filter_input_mode = false;
        // If we're on SearchResults tab, re-run the search with the new filter
        if self.current_tab == TabState::SearchResults {
            self.perform_search();
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
                self.perform_search();
            } else {
                if let Err(e) = self.reload_with_current_sort() {
                    eprintln!("Failed to reload data while typing filter: {}", e);
                }
            }
        }
    }

    pub fn remove_char_from_filter(&mut self) {
        if self.filter_input_mode {
            self.filter_input.pop();
            // Refresh data immediately as user types
            if self.current_tab == TabState::SearchResults {
                self.perform_search();
            } else {
                if let Err(e) = self.reload_with_current_sort() {
                    eprintln!("Failed to reload data while typing filter: {}", e);
                }
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
                    .collect()
            )
        }
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
        
        // Check for pending frame regeneration in editor
        app.check_frame_regeneration_timer();

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
                    KeyCode::Char('e') => {
                        // Only switch to editor if we have editor data
                        if app.editor_data.is_some() {
                            app.switch_to_editor();
                        }
                    },
                    KeyCode::Char('f') => {
                        if app.current_tab == TabState::Transcripts || app.current_tab == TabState::SearchResults {
                            app.toggle_filter_input();
                        }
                    },
                    KeyCode::Char('/') => {
                        if app.current_tab == TabState::Transcripts || app.current_tab == TabState::SearchResults {
                            app.toggle_search_input();
                        }
                    },
                    KeyCode::Char('o') => {
                        if app.current_tab == TabState::Editor {
                            app.toggle_editor_overlay();
                        }
                    },
                    KeyCode::Char('[') => {
                        if app.current_tab == TabState::Editor {
                            app.adjust_selected_frame_time(false); // Move backward
                        }
                    },
                    KeyCode::Char(']') => {
                        if app.current_tab == TabState::Editor {
                            app.adjust_selected_frame_time(true); // Move forward
                        }
                    },
                    KeyCode::Char('c') => {
                        if (app.current_tab == TabState::Transcripts || app.current_tab == TabState::SearchResults) && key.modifiers.contains(KeyModifiers::CONTROL) {
                            app.clear_filter();
                            app.clear_search();
                        } else if app.current_tab == TabState::SearchResults {
                            // Open editor from selected search result
                            if let Err(e) = app.open_editor_from_selected_match() {
                                eprintln!("Failed to open editor: {}", e);
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
                        } else if app.current_tab == TabState::Editor {
                            app.select_frame(FrameSelection::Start);
                        }
                    },
                    KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('L') => {
                        if app.current_tab == TabState::Transcripts {
                            if key.code == KeyCode::Char('L') || key.modifiers.contains(KeyModifiers::SHIFT) {
                                app.jump_to_last_page();
                            } else {
                                app.next_page();
                            }
                        } else if app.current_tab == TabState::Editor {
                            app.select_frame(FrameSelection::End);
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
    let chunks = if app.current_tab == TabState::Transcripts || app.current_tab == TabState::SearchResults {
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
    } else if app.current_tab == TabState::SearchResults {
        // For SearchResults tab, use full width for controls (no page info)
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(100), // Controls area takes full width
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
        TabState::Editor => render_editor_tab(f, chunks[0], app),
    }

    // Render filter and search sections (on transcripts and search results tabs)
    if app.current_tab == TabState::Transcripts || app.current_tab == TabState::SearchResults {
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
                let mut tab_controls = String::new();
                if !app.search_results.is_empty() {
                    tab_controls.push('r');
                }
                if app.editor_data.is_some() {
                    if !tab_controls.is_empty() {
                        tab_controls.push('/');
                    }
                    tab_controls.push('e');
                }
                if !tab_controls.is_empty() {
                    format!("{}{}/Tab: Switch  q: Quit", base_controls, tab_controls)
                } else {
                    format!("{}/Tab: Switch  q: Quit", base_controls)
                }
            }
        },
        TabState::System => {
            let base_controls = "↑↓/jk: Navigate  Enter: Start/Kill Process  t/s";
            let mut tab_controls = String::new();
            if !app.search_results.is_empty() {
                tab_controls.push('r');
            }
            if app.editor_data.is_some() {
                if !tab_controls.is_empty() {
                    tab_controls.push('/');
                }
                tab_controls.push('e');
            }
            if !tab_controls.is_empty() {
                format!("{}{}/Tab: Switch  q: Quit", base_controls, tab_controls)
            } else {
                format!("{}/Tab: Switch  q: Quit", base_controls)
            }
        },
        TabState::SearchResults => {
            if app.filter_input_mode {
                "Enter: Apply  Esc: Cancel  Ctrl+C: Clear  Type to filter...".to_string()
            } else if app.search_input_mode {
                "Enter: Search  Esc: Cancel  Ctrl+C: Clear  Type to search...".to_string()
            } else {
                let base_controls = "↑↓/jk: Navigate  c: Open Editor  f: Filter  /: Search  Ctrl+C: Clear  t/s";
                let mut tab_controls = String::new();
                tab_controls.push('r');  // Always have 'r' since we're on SearchResults tab
                if app.editor_data.is_some() {
                    tab_controls.push('/');
                    tab_controls.push('e');
                }
                format!("{}{}/Tab: Switch  q: Quit", base_controls, tab_controls)
            }
        },
        TabState::Editor => {
            let overlay_status = if app.editor_data.as_ref().map_or(false, |data| data.show_overlay_text) {
                "ON"
            } else {
                "OFF"
            };
            let selected_frame = if app.editor_data.as_ref().map_or(false, |data| data.selected_frame == FrameSelection::Start) {
                "Start"
            } else {
                "End"
            };
            let pending_regen = if app.editor_data.as_ref().map_or(false, |data| data.pending_frame_regeneration.is_some()) {
                " (regenerating...)"
            } else {
                ""
            };
            let base_controls = format!("h/l: Select Frame ({})  [/]: Adjust Time{}  o: Toggle Overlay ({})  t/s", selected_frame, pending_regen, overlay_status);
            let mut tab_controls = String::new();
            if !app.search_results.is_empty() {
                tab_controls.push('r');
            }
            if app.editor_data.is_some() {
                if !tab_controls.is_empty() {
                    tab_controls.push('/');
                }
                tab_controls.push('e');
            }
            if !tab_controls.is_empty() {
                format!("{}{}/Tab: Switch  q: Quit", base_controls, tab_controls)
            } else {
                format!("{}/Tab: Switch  q: Quit", base_controls)
            }
        },
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

pub fn create_tab_title_with_editor(current_tab: TabState, colors: &TableColors, has_search_results: bool, has_editor_data: bool) -> ratatui::text::Line<'_> {
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

    // Only show editor tab if we have editor data
    if has_editor_data {
        spans.push(Span::styled(" | ", Style::default().fg(colors.row_fg)));
        spans.push(match current_tab {
            TabState::Editor => Span::styled("Editor (e)", Style::default().fg(Color::White)),
            _ => Span::styled("Editor (e)", Style::default().fg(colors.footer_border_color)),
        });
    }

    Line::from(spans)
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