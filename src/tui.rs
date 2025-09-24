use crate::files;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen, SetTitle},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style, Stylize},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
    Frame, Terminal,
};
use std::{error::Error, io};

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
        pub c50: Color,
        pub c100: Color,
        pub c200: Color,
        pub c300: Color,
        pub c400: Color,
        pub c500: Color,
        pub c600: Color,
        pub c700: Color,
        pub c800: Color,
        pub c900: Color,
        pub c950: Color,
    }

    pub const SLATE: Palette = Palette {
        c50: Color::Rgb(248, 250, 252),
        c100: Color::Rgb(241, 245, 249),
        c200: Color::Rgb(226, 232, 240),
        c300: Color::Rgb(203, 213, 225),
        c400: Color::Rgb(148, 163, 184),
        c500: Color::Rgb(100, 116, 139),
        c600: Color::Rgb(71, 85, 105),
        c700: Color::Rgb(51, 65, 85),
        c800: Color::Rgb(30, 41, 59),
        c900: Color::Rgb(15, 23, 42),
        c950: Color::Rgb(2, 6, 23),
    };

    pub const BLUE: Palette = Palette {
        c50: Color::Rgb(239, 246, 255),
        c100: Color::Rgb(219, 234, 254),
        c200: Color::Rgb(191, 219, 254),
        c300: Color::Rgb(147, 197, 253),
        c400: Color::Rgb(96, 165, 250),
        c500: Color::Rgb(59, 130, 246),
        c600: Color::Rgb(37, 99, 235),
        c700: Color::Rgb(29, 78, 216),
        c800: Color::Rgb(30, 64, 175),
        c900: Color::Rgb(30, 58, 138),
        c950: Color::Rgb(23, 37, 84),
    };
}

struct App {
    state: TableState,
    colors: TableColors,
    video_data: Vec<files::VideoInfo>,
}

impl Default for App {
    fn default() -> App {
        App {
            state: TableState::default(),
            colors: TableColors::new(&tailwind::BLUE),
            video_data: Vec::new(),
        }
    }
}

impl App {
    fn new() -> Result<App, Box<dyn Error>> {
        let video_data = files::load_video_info_from_cache(None)?;
        Ok(App {
            state: TableState::default(),
            colors: TableColors::new(&tailwind::BLUE),
            video_data,
        })
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.video_data.len().saturating_sub(1) {
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
                    self.video_data.len().saturating_sub(1)
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
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

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Down | KeyCode::Char('j') => app.next(),
                KeyCode::Up | KeyCode::Char('k') => app.previous(),
                _ => {}
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let rects = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Percentage(100)].as_ref())
        .split(f.area());

    let header_style = Style::default()
        .fg(app.colors.header_fg)
        .bg(app.colors.header_bg);
    let selected_row_style = Style::default()
        .add_modifier(Modifier::REVERSED)
        .fg(app.colors.selected_style_fg);

    let header = ["Filename", "Created At", "Generated At", "Lines", "Length", "Source"]
        .into_iter()
        .map(Cell::from)
        .collect::<Row>()
        .style(header_style)
        .height(1);

    let bar = " █ ";
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
        .highlight_symbol(bar)
        .bg(app.colors.buffer_bg)
        .row_highlight_style(selected_row_style)
        .block(
            Block::default()
                .title("Transcripts")
                .borders(Borders::ALL)
                .border_style(Style::new().fg(app.colors.footer_border_color)),
        );
    f.render_stateful_widget(t, rects[0], &mut app.state);
}