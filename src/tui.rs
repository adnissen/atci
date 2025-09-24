use crate::files;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
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
use std::{error::Error, io, time::{Duration, Instant}};

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
        }
    }
}

impl App {
    fn new() -> Result<App, Box<dyn Error>> {
        let terminal_height = crossterm::terminal::size()?.1;
        let page_size = Self::calculate_page_size(terminal_height) + 1;

        // Use database sorting instead of client-side sorting
        let cache_data = files::load_sorted_paginated_cache_data(
            None,        // filter
            0,           // page (first page)
            page_size,   // limit
            "last_generated", // sort by Generated At
            0,           // sort_order (0 = DESC, 1 = ASC)
        )?;

        Ok(App {
            state: TableState::default(),
            colors: TableColors::new(&tailwind::BLUE),
            video_data: cache_data.files,
            sort_column: Some(2), // Generated At column
            sort_order: SortOrder::Descending,
            last_refresh: Instant::now(),
            terminal_height,
            current_page: 0,
            total_pages: cache_data.pages.unwrap_or(1),
        })
    }

    fn calculate_page_size(terminal_height: u16) -> u32 {
        // Account for: margins (2), header (1), controls (3), table header (1), borders (2)
        // Leave some buffer for safety
        let available_height = terminal_height.saturating_sub(9);
        std::cmp::max(available_height as u32, 5) // Minimum 5 rows
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
                    // If no more pages, wrap to first item of current page
                    0
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
            None,        // filter
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
            None,        // filter
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
            None,        // filter
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
}

pub fn run() -> Result<(), Box<dyn Error>> {
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
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<(), Box<dyn Error>>
where
    <B as Backend>::Error: 'static,
{
    loop {
        terminal.draw(|f| ui(f, app))?;

        // Use poll to avoid blocking and allow periodic refreshes
        if event::poll(Duration::from_millis(1000))? {
            // Check if we should refresh data
            if app.should_refresh() {
                if let Err(e) = app.refresh_data() {
                    eprintln!("Failed to refresh data: {}", e);
                }
            }
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Down | KeyCode::Char('j') => app.next(),
                    KeyCode::Up | KeyCode::Char('k') => app.previous(),
                    KeyCode::Left | KeyCode::Char('h') => app.prev_page(),
                    KeyCode::Right | KeyCode::Char('l') => app.next_page(),
                    KeyCode::Char('1') => app.sort_by_column(0),
                    KeyCode::Char('2') => app.sort_by_column(1),
                    KeyCode::Char('3') => app.sort_by_column(2),
                    KeyCode::Char('4') => app.sort_by_column(3),
                    KeyCode::Char('5') => app.sort_by_column(4),
                    KeyCode::Char('6') => app.sort_by_column(5),
                    _ => {}
                }
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Min(3),        // Table area
            Constraint::Length(3),     // Bottom panes area
        ].as_ref())
        .split(f.area());

    // Split the bottom area into Controls and Page panes
    let bottom_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(90), // Controls area (9/10)
            Constraint::Percentage(10), // Page area (1/10)
        ].as_ref())
        .split(chunks[1]);

    let header_style = Style::default()
        .fg(app.colors.header_fg)
        .bg(app.colors.header_bg);
    let selected_row_style = Style::default()
        .add_modifier(Modifier::REVERSED)
        .fg(app.colors.selected_style_fg);

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
                .title("Transcripts")
                .borders(Borders::ALL)
                .border_style(Style::new().fg(app.colors.footer_border_color)),
        );
    f.render_stateful_widget(t, chunks[0], &mut app.state);

    // Controls section
    let controls_text = "↑↓/jk: Navigate  ←→/hl: Page  1-6: Sort  q: Quit";
    let controls_block = Block::default()
        .title("Controls")
        .borders(Borders::ALL)
        .border_style(Style::new().fg(app.colors.footer_border_color));

    let controls_paragraph = Paragraph::new(controls_text)
        .block(controls_block)
        .style(Style::new().fg(app.colors.row_fg))
        .alignment(Alignment::Center);

    f.render_widget(controls_paragraph, bottom_chunks[0]);

    // Page info section
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