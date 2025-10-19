use crate::config;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Paragraph};
use std::{error::Error, fs, time::Duration};
use tui_big_text::BigText;

use crate::tui::{App, ServiceStatus, SystemSection, SystemService};

impl App {
    pub fn system_next(&mut self) {
        match self.system_section {
            SystemSection::Config => {
                self.config_next_field();
            }
            SystemSection::WatchDirectories => {
                if self.watch_directories_selected_index
                    < self.config_data.watch_directories.len().saturating_sub(1)
                {
                    self.watch_directories_selected_index += 1;
                } else {
                    // At the bottom of watch directories, switch to config section
                    self.system_section = SystemSection::Config;
                    self.config_selected_field = 0;
                }
            }
        }
    }

    pub fn system_previous(&mut self) {
        match self.system_section {
            SystemSection::Config => {
                if self.config_selected_field > 0 {
                    self.config_previous_field();
                } else {
                    // At the top of config, switch to watch directories section
                    self.system_section = SystemSection::WatchDirectories;
                    // Set to last item in watch directories
                    if !self.config_data.watch_directories.is_empty() {
                        self.watch_directories_selected_index =
                            self.config_data.watch_directories.len() - 1;
                    }
                }
            }
            SystemSection::WatchDirectories => {
                if self.watch_directories_selected_index > 0 {
                    self.watch_directories_selected_index -= 1;
                }
            }
        }
    }

    pub fn refresh_system_services(&mut self) {
        self.system_services = get_system_services();
        self.last_system_refresh = std::time::Instant::now();
    }

    pub fn should_refresh_system_services(&self) -> bool {
        self.last_system_refresh.elapsed() >= Duration::from_millis(200)
    }

    pub fn open_web_server_in_browser(&self) -> Result<(), Box<dyn Error>> {
        // Open the configured hostname in the default browser
        open::that(&self.config_data.hostname)?;
        Ok(())
    }
}

pub fn get_atci_dir() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let home_dir = dirs::home_dir().ok_or("Could not find home directory")?;
    let atci_dir = home_dir.join(".atci");
    Ok(atci_dir)
}

pub struct ServicePids {
    pub watcher: Vec<u32>,
    pub web: Vec<u32>,
}

pub fn find_all_pid_files() -> Result<ServicePids, Box<dyn std::error::Error>> {
    let atci_dir = get_atci_dir()?;
    let config_sha = config::get_config_path_sha();
    let mut watcher_pids = Vec::new();
    let mut web_pids = Vec::new();

    if atci_dir.exists() {
        for entry in fs::read_dir(atci_dir)? {
            let entry = entry?;
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();

            // Check for watcher PID files
            let watcher_prefix = format!("atci.watcher.{}.", config_sha);
            if file_name_str.starts_with(&watcher_prefix) && file_name_str.ends_with(".pid") {
                let pid_str = &file_name_str[watcher_prefix.len()..file_name_str.len() - 4];
                if let Ok(pid) = pid_str.parse::<u32>() {
                    watcher_pids.push(pid);
                }
            }

            // Check for web PID files
            let web_prefix = format!("atci.web.{}.", config_sha);
            if file_name_str.starts_with(&web_prefix) && file_name_str.ends_with(".pid") {
                let pid_str = &file_name_str[web_prefix.len()..file_name_str.len() - 4];
                if let Ok(pid) = pid_str.parse::<u32>() {
                    web_pids.push(pid);
                }
            }
        }
    }

    Ok(ServicePids {
        watcher: watcher_pids,
        web: web_pids,
    })
}

pub fn is_process_running(pid: u32) -> bool {
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

pub fn get_system_services() -> Vec<SystemService> {
    let mut services = Vec::new();

    // Get all PID files in a single scan
    match find_all_pid_files() {
        Ok(service_pids) => {
            // Check watcher service
            let running_watcher_pids: Vec<u32> = service_pids
                .watcher
                .into_iter()
                .filter(|&pid| is_process_running(pid))
                .collect();

            if !running_watcher_pids.is_empty() {
                services.push(SystemService {
                    name: "File Watcher".to_string(),
                    status: ServiceStatus::Active,
                    pids: running_watcher_pids,
                });
            } else {
                services.push(SystemService {
                    name: "File Watcher".to_string(),
                    status: ServiceStatus::Stopped,
                    pids: Vec::new(),
                });
            }

            // Check web service
            let running_web_pids: Vec<u32> = service_pids
                .web
                .into_iter()
                .filter(|&pid| is_process_running(pid))
                .collect();

            if !running_web_pids.is_empty() {
                services.push(SystemService {
                    name: "Web Server".to_string(),
                    status: ServiceStatus::Active,
                    pids: running_web_pids,
                });
            } else {
                services.push(SystemService {
                    name: "Web Server".to_string(),
                    status: ServiceStatus::Stopped,
                    pids: Vec::new(),
                });
            }
        }
        Err(_) => {
            // If we can't read PID files, show both services as stopped
            services.push(SystemService {
                name: "File Watcher".to_string(),
                status: ServiceStatus::Stopped,
                pids: Vec::new(),
            });
            services.push(SystemService {
                name: "Web Server".to_string(),
                status: ServiceStatus::Stopped,
                pids: Vec::new(),
            });
        }
    }

    services
}

pub fn render_system_tab(
    f: &mut Frame,
    area: ratatui::layout::Rect,
    app: &mut App,
    conn: &rusqlite::Connection,
) {
    // Split the main content area into sections
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(8), // Services section (smaller)
                Constraint::Min(10),   // Config section (expandable)
            ]
            .as_ref(),
        )
        .split(area);

    // Create main block
    let main_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(app.colors.footer_border_color));

    f.render_widget(main_block, area);

    // Split the services row horizontally: services on left, bigtext on right
    let services_row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage(50), // Services box (left half)
                Constraint::Percentage(50), // BigText (right half)
            ]
            .as_ref(),
        )
        .split(main_chunks[0]);

    // Services section inside the main block
    let services_content = render_services_list(app);
    let services_paragraph = Paragraph::new(services_content)
        .block(
            Block::default()
                .title("Services")
                .borders(Borders::ALL)
                .border_style(Style::new().fg(app.colors.footer_border_color)),
        )
        .style(Style::new().fg(app.colors.row_fg))
        .alignment(Alignment::Left);

    f.render_widget(services_paragraph, services_row[0]);

    // Split the right side vertically: BigText on top, text below
    let right_side_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Min(5),    // BigText area
                Constraint::Length(1), // Text below
            ]
            .as_ref(),
        )
        .split(services_row[1]);

    // BigText "atci" on the right side
    let big_text = BigText::builder()
        .lines(vec!["atci".into()])
        .style(Style::new().fg(app.colors.row_fg))
        .centered()
        .build();

    f.render_widget(big_text, right_side_chunks[0]);

    // Text below BigText
    let below_text = Paragraph::new("(Andrew's transcript and clipping interface)")
        .style(Style::new().fg(app.colors.row_fg))
        .alignment(Alignment::Center);

    f.render_widget(below_text, right_side_chunks[1]);

    // Split the config row horizontally: config on left, queue+stats on right
    let config_row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage(50), // Config box (left half)
                Constraint::Percentage(50), // Queue+Stats (right half)
            ]
            .as_ref(),
        )
        .split(main_chunks[1]);

    // Split the left column vertically: watch directories on top, config on bottom
    let left_column = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage(50), // Watch directories (top half)
                Constraint::Percentage(50), // Config (bottom half)
            ]
            .as_ref(),
        )
        .split(config_row[0]);

    // Split the right side vertically: queue on top, stats on bottom
    let right_column = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage(50), // Queue box (top half)
                Constraint::Percentage(50), // Stats box (bottom half)
            ]
            .as_ref(),
        )
        .split(config_row[1]);

    // Watch directories section on top left
    let watch_dirs_table = render_watch_directories_section(app, left_column[0]);
    f.render_widget(watch_dirs_table, left_column[0]);

    // Config editing section on the bottom left with custom scrolling
    let config_content = render_config_section(app);
    let config_title = if app.system_section == SystemSection::Config {
        "Configuration (Enter: Edit, Shift+R: Reload From Disk) [ACTIVE]"
    } else {
        "Configuration (Enter: Edit, Shift+R: Reload From Disk)"
    };
    let config_border_color = if app.system_section == SystemSection::Config {
        ratatui::style::Color::Yellow
    } else {
        app.colors.footer_border_color
    };

    // Render the config block border
    let config_block = Block::default()
        .title(config_title)
        .borders(Borders::ALL)
        .border_style(Style::new().fg(config_border_color));

    let config_paragraph = Paragraph::new(config_content)
        .block(config_block)
        .style(Style::new().fg(app.colors.row_fg))
        .alignment(Alignment::Left);

    f.render_widget(config_paragraph, left_column[1]);

    // Queue section on the top right
    let queue_table = render_queue_section(app, right_column[0]);
    f.render_widget(queue_table, right_column[0]);

    // Stats section on the bottom right
    let stats_table = render_stats_section(app, right_column[1], conn);
    f.render_widget(stats_table, right_column[1]);
}

fn render_watch_directories_section<'a>(
    app: &App,
    _area: ratatui::layout::Rect,
) -> ratatui::widgets::Table<'a> {
    use ratatui::style::{Color, Modifier, Style};
    use ratatui::widgets::{Block, Borders, Cell, Row, Table};

    let mut rows = Vec::new();

    // Add watch directories
    for (i, dir) in app.config_data.watch_directories.iter().enumerate() {
        let is_selected = i == app.watch_directories_selected_index
            && app.system_section == SystemSection::WatchDirectories;

        let dir_cell = if is_selected {
            Cell::from(format!("► {}", dir)).style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Cell::from(format!("  {}", dir)).style(Style::default().fg(app.colors.row_fg))
        };

        rows.push(Row::new(vec![dir_cell]).style(Style::default().fg(app.colors.row_fg)));
    }

    // If no directories, show a message
    if rows.is_empty() {
        let empty_row = Row::new(vec![Cell::from("  No watch directories configured")])
            .style(Style::default().fg(Color::Gray));
        rows.push(empty_row);
    }

    let widths = [Constraint::Min(30)];

    let watch_dirs_title = if app.system_section == SystemSection::WatchDirectories {
        "Watch Directories (↑↓/jk: Navigate, n: Add, d: Delete, r: Regenerate) [ACTIVE]"
    } else {
        "Watch Directories (↑↓/jk: Navigate, n: Add, d: Delete, r: Regenerate)"
    };
    let watch_dirs_border_color = if app.system_section == SystemSection::WatchDirectories {
        Color::Yellow
    } else {
        app.colors.footer_border_color
    };

    let block = Block::default()
        .title(watch_dirs_title)
        .borders(Borders::ALL)
        .border_style(Style::new().fg(watch_dirs_border_color));

    Table::new(rows, widths).block(block).column_spacing(1)
}

fn render_services_list(app: &App) -> ratatui::text::Text<'static> {
    use ratatui::style::{Color, Style};
    use ratatui::text::{Line, Span, Text};

    let mut lines = Vec::new();

    for service in app.system_services.iter() {
        let mut spans = Vec::new();

        // Service name (no selection indicator)
        spans.push(Span::raw(format!("{}: ", service.name)));

        // Status and PIDs
        match service.status {
            ServiceStatus::Active => {
                spans.push(Span::styled("active", Style::default().fg(Color::Green)));
                if !service.pids.is_empty() {
                    let pid_list = service
                        .pids
                        .iter()
                        .map(|pid| pid.to_string())
                        .collect::<Vec<_>>()
                        .join(", ");
                    spans.push(Span::raw(" (PID: "));
                    spans.push(Span::styled(pid_list, Style::default().fg(Color::Cyan)));
                    spans.push(Span::raw(")"));
                }

                // Show hostname for Web Server
                if service.name == "Web Server" {
                    lines.push(Line::from(spans));
                    let hostname_spans = vec![
                        Span::raw("  "), // Indent
                        Span::styled(
                            app.config_data.hostname.clone(),
                            Style::default().fg(Color::Cyan),
                        ),
                        Span::raw(" "),
                        Span::styled(
                            "← [OPEN (o)]",
                            Style::default()
                                .fg(Color::Green)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ];
                    lines.push(Line::from(hostname_spans));
                    continue; // Skip the normal line push at the end
                }
            }
            ServiceStatus::Stopped => {
                spans.push(Span::styled("stopped", Style::default().fg(Color::Red)));
            }
        }

        lines.push(Line::from(spans));
    }

    if lines.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "No services found",
            Style::default().fg(Color::Gray),
        )]));
    }

    Text::from(lines)
}

fn is_boolean_field(field_name: &str) -> bool {
    matches!(field_name, "allow_whisper" | "allow_subtitles")
}

fn render_config_section(app: &App) -> ratatui::text::Text<'static> {
    use ratatui::style::{Color, Style};
    use ratatui::text::{Line, Span, Text};

    let mut lines = Vec::new();
    let field_names = app.get_config_field_names();

    for (index, field_name) in field_names.iter().enumerate() {
        let is_selected =
            index == app.config_selected_field && app.system_section == SystemSection::Config;
        let mut spans = Vec::new();

        // Add selection indicator
        if is_selected {
            spans.push(Span::styled("► ", Style::default().fg(Color::Yellow)));
        } else {
            spans.push(Span::raw("  "));
        }

        // Field name
        let field_display_name = field_name.replace("_", " ");
        spans.push(Span::styled(
            format!("{}: ", field_display_name),
            Style::default().fg(Color::Cyan),
        ));

        // Check if this is a boolean field
        let is_bool = is_boolean_field(field_name);
        let is_password = *field_name == "password";

        // Field value
        let mut field_value = if app.config_editing_mode && is_selected && !is_bool {
            app.config_input_buffer.clone()
        } else {
            app.get_config_field_value(index)
        };

        // Mask password field when not editing
        if is_password && !(app.config_editing_mode && is_selected) && !field_value.is_empty() {
            field_value = "******".to_string();
        }

        let value_style = if app.config_editing_mode && is_selected && !is_bool {
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else if is_selected {
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        // For boolean fields, show checkbox instead of true/false
        if is_bool {
            let (checkbox, label) = if field_value == "true" {
                ("☑", " [true]")
            } else {
                ("☐", " [false]")
            };
            spans.push(Span::styled(checkbox, value_style));
            spans.push(Span::styled(label, value_style));
        } else {
            // Show full value without truncation (wrapping is handled by the Paragraph widget)
            spans.push(Span::styled(field_value, value_style));

            // Show editing indicator
            if app.config_editing_mode && is_selected {
                spans.push(Span::styled("█", Style::default().fg(Color::Green)));
            }
        }

        lines.push(Line::from(spans));
    }

    if lines.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "No config fields found",
            Style::default().fg(Color::Gray),
        )]));
    }

    Text::from(lines)
}

fn render_queue_section<'a>(
    app: &App,
    _area: ratatui::layout::Rect,
) -> ratatui::widgets::Table<'a> {
    use ratatui::style::{Color, Modifier, Style};
    use ratatui::widgets::{Block, Borders, Cell, Row, Table};

    let mut rows = Vec::new();

    // Add currently processing item if exists
    if let Some(ref path) = app.currently_processing {
        let age_text = format_age(app.currently_processing_age);
        let status_cell = Cell::from(format!("PROCESSING ({})", age_text)).style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );
        let path_cell = Cell::from(path.clone());
        rows.push(
            Row::new(vec![status_cell, path_cell]).style(Style::default().fg(app.colors.row_fg)),
        );
    }

    // Add queue items
    for (i, path) in app.queue_items.iter().enumerate() {
        let position = i + 1;
        let status_cell =
            Cell::from(format!("#{}", position)).style(Style::default().fg(app.colors.row_fg));
        let path_cell = Cell::from(path.clone());
        rows.push(
            Row::new(vec![status_cell, path_cell]).style(Style::default().fg(app.colors.row_fg)),
        );
    }

    // If no items at all, show a message
    if rows.is_empty() {
        let empty_row = Row::new(vec![Cell::from(""), Cell::from("No items in queue")])
            .style(Style::default().fg(app.colors.row_fg));
        rows.push(empty_row);
    }

    let widths = [Constraint::Length(20), Constraint::Min(30)];

    let block = Block::default()
        .title("Queue")
        .borders(Borders::ALL)
        .border_style(Style::new().fg(app.colors.footer_border_color));

    Table::new(rows, widths)
        .block(block)
        .header(
            Row::new(vec![
                Cell::from("Status").style(
                    Style::default()
                        .fg(app.colors.header_fg)
                        .add_modifier(Modifier::BOLD),
                ),
                Cell::from("Path").style(
                    Style::default()
                        .fg(app.colors.header_fg)
                        .add_modifier(Modifier::BOLD),
                ),
            ])
            .style(
                Style::default()
                    .bg(app.colors.header_bg)
                    .fg(app.colors.header_fg),
            )
            .height(1),
        )
        .column_spacing(1)
}

fn render_stats_section<'a>(
    app: &App,
    _area: ratatui::layout::Rect,
    conn: &rusqlite::Connection,
) -> ratatui::widgets::Table<'a> {
    use ratatui::style::{Color, Modifier, Style};
    use ratatui::widgets::{Block, Borders, Cell, Row, Table};

    let mut rows = Vec::new();

    let mut stmt = conn
        .prepare("SELECT COUNT(*) as count FROM video_info")
        .expect("couldn't prepare statement to get total transcripts");

    let total_transcripts = stmt
        .query_row([], |row| Ok(row.get::<_, i64>(0)? as usize))
        .expect("couldn't get total transcripts");

    // Get all durations and sum them
    let mut duration_stmt = conn
        .prepare("SELECT duration FROM video_info WHERE duration IS NOT NULL")
        .expect("couldn't prepare statement to get durations");

    let durations = duration_stmt
        .query_map([], |row| row.get::<_, String>(0))
        .expect("couldn't query durations");

    let mut total_seconds = 0u64;
    for duration_result in durations {
        if let Ok(duration_str) = duration_result
            && let Some(seconds) = parse_duration_to_seconds(&duration_str)
        {
            total_seconds += seconds;
        }
    }

    let total_runtime = format_seconds_to_duration(total_seconds);

    // Add total transcripts row with a divider style
    let total_row = Row::new(vec![
        Cell::from("Total Transcripts").style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Cell::from(total_transcripts.to_string()).style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Cell::from(total_runtime).style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    rows.push(total_row);

    // Add a separator row
    let separator_row = Row::new(vec![
        Cell::from("─".repeat(30)),
        Cell::from("─".repeat(10)),
        Cell::from("─".repeat(10)),
    ])
    .style(Style::default().fg(app.colors.footer_border_color));
    rows.push(separator_row);

    // Query database for video counts per watch directory
    match get_directory_stats(conn, &app.config_data.watch_directories) {
        Ok(dir_counts) => {
            if dir_counts.is_empty() {
                let empty_row = Row::new(vec![
                    Cell::from("No watch directories configured"),
                    Cell::from(""),
                    Cell::from(""),
                ])
                .style(Style::default().fg(Color::Gray));
                rows.push(empty_row);
            } else {
                for (dir, count, duration) in dir_counts {
                    // Truncate directory path if too long, show last part
                    let display_dir = if dir.len() > 35 {
                        format!("...{}", &dir[dir.len() - 32..])
                    } else {
                        dir
                    };

                    let dir_row = Row::new(vec![
                        Cell::from(display_dir).style(Style::default().fg(app.colors.row_fg)),
                        Cell::from(count.to_string()).style(Style::default().fg(Color::Green)),
                        Cell::from(duration).style(Style::default().fg(Color::Green)),
                    ]);
                    rows.push(dir_row);
                }
            }
        }
        Err(e) => {
            let error_row = Row::new(vec![
                Cell::from(format!("Error: {}", e)),
                Cell::from(""),
                Cell::from(""),
            ])
            .style(Style::default().fg(Color::Red));
            rows.push(error_row);
        }
    }

    let widths = [
        Constraint::Min(25),
        Constraint::Length(10),
        Constraint::Length(10),
    ];

    let block = Block::default()
        .title("Stats")
        .borders(Borders::ALL)
        .border_style(Style::new().fg(app.colors.footer_border_color));

    Table::new(rows, widths).block(block).column_spacing(1)
}

type DirectoryStat = (String, usize, String);

fn get_directory_stats(
    conn: &rusqlite::Connection,
    configured_watch_dirs: &[String],
) -> Result<Vec<DirectoryStat>, Box<dyn std::error::Error>> {
    use std::collections::HashMap;

    // First, query the database for actual stats
    let mut stmt = conn.prepare(
        "SELECT watch_directory, COUNT(*) as count
         FROM video_info
         WHERE watch_directory IS NOT NULL
         GROUP BY watch_directory
         ORDER BY watch_directory",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize))
    })?;

    let mut stats_map: HashMap<String, (usize, u64)> = HashMap::new();

    for row in rows {
        let (dir, count) = row?;

        // Get all durations for this directory
        let mut duration_stmt = conn.prepare(
            "SELECT duration FROM video_info WHERE watch_directory = ? AND duration IS NOT NULL",
        )?;
        let durations = duration_stmt.query_map([&dir], |row| row.get::<_, String>(0))?;

        let mut total_seconds = 0u64;
        for duration_result in durations {
            if let Ok(duration_str) = duration_result
                && let Some(seconds) = parse_duration_to_seconds(&duration_str)
            {
                total_seconds += seconds;
            }
        }

        stats_map.insert(dir, (count, total_seconds));
    }

    // Now iterate through all configured watch directories
    let mut results = Vec::new();
    for dir in configured_watch_dirs {
        let (count, total_seconds) = stats_map.get(dir).copied().unwrap_or((0, 0));
        results.push((
            dir.clone(),
            count,
            format_seconds_to_duration(total_seconds),
        ));
    }

    Ok(results)
}

fn format_age(seconds: u64) -> String {
    if seconds < 60 {
        format!("{}s", seconds)
    } else if seconds < 3600 {
        let minutes = seconds / 60;
        let secs = seconds % 60;
        format!("{}m {}s", minutes, secs)
    } else {
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;
        format!("{}h {}m", hours, minutes)
    }
}

fn parse_duration_to_seconds(duration_str: &str) -> Option<u64> {
    let parts: Vec<&str> = duration_str.split(':').collect();
    if parts.len() != 3 {
        return None;
    }

    let hours = parts[0].parse::<u64>().ok()?;
    let minutes = parts[1].parse::<u64>().ok()?;
    let seconds = parts[2].parse::<u64>().ok()?;

    Some(hours * 3600 + minutes * 60 + seconds)
}

fn format_seconds_to_duration(total_seconds: u64) -> String {
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}
