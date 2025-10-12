use crate::config;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Paragraph};
use std::{error::Error, fs, time::Duration};
use tui_big_text::BigText;

use crate::tui::{App, ServiceStatus, SystemSection, SystemService, create_tab_title_with_editor};

impl App {
    pub fn system_next(&mut self) {
        match self.system_section {
            SystemSection::Services => {
                if !self.system_services.is_empty()
                    && self.system_selected_index < self.system_services.len() - 1
                {
                    self.system_selected_index += 1;
                } else {
                    // Move to config section
                    self.system_section = SystemSection::Config;
                    self.config_selected_field = 0;
                }
            }
            SystemSection::Config => {
                self.config_next_field();
            }
        }
    }

    pub fn system_previous(&mut self) {
        match self.system_section {
            SystemSection::Services => {
                if self.system_selected_index > 0 {
                    self.system_selected_index -= 1;
                }
            }
            SystemSection::Config => {
                if self.config_selected_field > 0 {
                    self.config_previous_field();
                } else {
                    // Move back to services section
                    self.system_section = SystemSection::Services;
                    if !self.system_services.is_empty() {
                        self.system_selected_index = self.system_services.len() - 1;
                    }
                }
            }
        }
    }

    pub fn refresh_system_services(&mut self) {
        self.system_services = get_system_services();
        // Ensure selection is within bounds
        if self.system_selected_index >= self.system_services.len()
            && !self.system_services.is_empty()
        {
            self.system_selected_index = self.system_services.len() - 1;
        }
        self.last_system_refresh = std::time::Instant::now();
    }

    pub fn should_refresh_system_services(&self) -> bool {
        self.last_system_refresh.elapsed() >= Duration::from_millis(200)
    }

    pub fn kill_selected_service(&mut self) -> Result<(), Box<dyn Error>> {
        if self.system_selected_index < self.system_services.len() {
            let service = &self.system_services[self.system_selected_index];
            if !service.pids.is_empty() {
                let pid = service.pids[0]; // Kill first PID for now
                let service_type = get_service_type_from_name(&service.name);
                kill_process(pid)?;
                // Delete the associated PID file
                if let Err(e) = delete_pid_file(pid, service_type) {
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
                    let service_type = get_service_type_from_name(&service.name);
                    match service_type {
                        "watcher" => start_watcher_process()?,
                        "web" => start_web_process()?,
                        _ => return Err("Unknown service type".into()),
                    }
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

fn kill_process(pid: u32) -> Result<(), Box<dyn Error>> {
    #[cfg(unix)]
    {
        use std::process::Command;
        let output = Command::new("kill").arg(pid.to_string()).output()?;

        if !output.status.success() {
            return Err(format!(
                "Failed to kill process {}: {}",
                pid,
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
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
            return Err(format!(
                "Failed to kill process {}: {}",
                pid,
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }
    }

    Ok(())
}

fn get_service_type_from_name(name: &str) -> &str {
    match name {
        "File Watcher" => "watcher",
        "Web Server" => "web",
        _ => "watcher", // Default to watcher for unknown services
    }
}

fn delete_pid_file(pid: u32, service_type: &str) -> Result<(), Box<dyn Error>> {
    let atci_dir = get_atci_dir()?;
    let config_sha = config::get_config_path_sha();

    // Construct the expected PID file name
    let pid_file_name = format!("atci.{}.{}.{}.pid", service_type, config_sha, pid);
    let pid_file_path = atci_dir.join(pid_file_name);

    // Only try to delete if the file exists
    if pid_file_path.exists() {
        fs::remove_file(pid_file_path)?;
    }

    Ok(())
}

fn start_watcher_process() -> Result<(), Box<dyn Error>> {
    use std::fs::OpenOptions;
    use std::process::Stdio;

    // Get the current executable path
    let current_exe = std::env::current_exe()?;

    // Create log file in ~/.atci/watcher.log
    let home_dir = dirs::home_dir().ok_or("Could not find home directory")?;
    let log_path = home_dir.join(".atci").join("watcher.log");

    // Ensure .atci directory exists
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;

    // Clone file descriptors for stdout and stderr
    let stdout_file = log_file.try_clone()?;
    let stderr_file = log_file;

    // Spawn a new atci watch process with output redirected to log
    // Use stdin(Stdio::null()) to detach from terminal
    std::process::Command::new(&current_exe)
        .arg("watch")
        .stdin(Stdio::null())
        .stdout(stdout_file)
        .stderr(stderr_file)
        .spawn()?;

    Ok(())
}

fn start_web_process() -> Result<(), Box<dyn Error>> {
    use std::fs::OpenOptions;
    use std::process::Stdio;

    // Get the current executable path
    let current_exe = std::env::current_exe()?;

    // Create log file in ~/.atci/web.log
    let home_dir = dirs::home_dir().ok_or("Could not find home directory")?;
    let log_path = home_dir.join(".atci").join("web.log");

    // Ensure .atci directory exists
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;

    // Clone file descriptors for stdout and stderr
    let stdout_file = log_file.try_clone()?;
    let stderr_file = log_file;

    // Spawn a new atci web process with output redirected to log
    // Use stdin(Stdio::null()) to detach from terminal
    std::process::Command::new(&current_exe)
        .arg("web")
        .arg("all")
        .stdin(Stdio::null())
        .stdout(stdout_file)
        .stderr(stderr_file)
        .spawn()?;

    Ok(())
}

pub fn render_system_tab(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let title = create_tab_title_with_editor(
        app.current_tab,
        &app.colors,
        !app.search_results.is_empty(),
        app.editor_data.is_some(),
        app.file_view_data.is_some(),
    );

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

    // Create main block with tab title
    let main_block = Block::default()
        .title(title)
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
    let services_title = if app.system_section == SystemSection::Services {
        "Services (↑↓/jk: Navigate, Enter: Start/Kill) [ACTIVE]"
    } else {
        "Services (↑↓/jk: Navigate, Enter: Start/Kill)"
    };
    let services_border_color = if app.system_section == SystemSection::Services {
        ratatui::style::Color::Yellow
    } else {
        app.colors.footer_border_color
    };
    let services_paragraph = Paragraph::new(services_content)
        .block(
            Block::default()
                .title(services_title)
                .borders(Borders::ALL)
                .border_style(Style::new().fg(services_border_color)),
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

    // Config editing section on the left
    let config_content = render_config_section(app);
    let config_title = if app.system_section == SystemSection::Config {
        "Configuration (↑↓/jk: Navigate, Enter: Edit, Auto-save, Shift+R: Reload) [ACTIVE]"
    } else {
        "Configuration (↑↓/jk: Navigate, Enter: Edit, Auto-save, Shift+R: Reload)"
    };
    let config_border_color = if app.system_section == SystemSection::Config {
        ratatui::style::Color::Yellow
    } else {
        app.colors.footer_border_color
    };
    let config_paragraph = Paragraph::new(config_content)
        .block(
            Block::default()
                .title(config_title)
                .borders(Borders::ALL)
                .border_style(Style::new().fg(config_border_color)),
        )
        .style(Style::new().fg(app.colors.row_fg))
        .alignment(Alignment::Left)
        .wrap(ratatui::widgets::Wrap { trim: true });

    f.render_widget(config_paragraph, config_row[0]);

    // Queue section on the top right
    let queue_table = render_queue_section(app, right_column[0]);
    f.render_widget(queue_table, right_column[0]);

    // Stats section on the bottom right
    let stats_table = render_stats_section(app, right_column[1]);
    f.render_widget(stats_table, right_column[1]);
}

fn render_services_list(app: &App) -> ratatui::text::Text<'static> {
    use ratatui::style::{Color, Style};
    use ratatui::text::{Line, Span, Text};

    let mut lines = Vec::new();

    for (index, service) in app.system_services.iter().enumerate() {
        let is_selected =
            index == app.system_selected_index && app.system_section == SystemSection::Services;

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
                    let pid_list = service
                        .pids
                        .iter()
                        .map(|pid| pid.to_string())
                        .collect::<Vec<_>>()
                        .join(", ");
                    spans.push(Span::raw(" (PID: "));
                    spans.push(Span::styled(pid_list, Style::default().fg(Color::Cyan)));
                    spans.push(Span::raw(")"));

                    // Show kill option if selected
                    if is_selected {
                        spans.push(Span::raw(" "));
                        spans.push(Span::styled(
                            "← [KILL]",
                            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                        ));
                    }
                }
            }
            ServiceStatus::Stopped => {
                spans.push(Span::styled("stopped", Style::default().fg(Color::Red)));

                // Show start option if selected
                if is_selected {
                    spans.push(Span::raw(" "));
                    spans.push(Span::styled(
                        "← [START]",
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ));
                }
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

        // Field value
        let field_value = if app.config_editing_mode && is_selected {
            app.config_input_buffer.clone()
        } else {
            app.get_config_field_value(index)
        };

        let value_style = if app.config_editing_mode && is_selected {
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

        // Show full value without truncation (wrapping is handled by the Paragraph widget)
        spans.push(Span::styled(field_value, value_style));

        // Show editing indicator
        if app.config_editing_mode && is_selected {
            spans.push(Span::styled(
                " [EDITING]",
                Style::default().fg(Color::Green),
            ));
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

fn render_queue_section<'a>(app: &App, _area: ratatui::layout::Rect) -> ratatui::widgets::Table<'a> {
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
        rows.push(Row::new(vec![status_cell, path_cell]).style(Style::default().fg(app.colors.row_fg)));
    }

    // Add queue items
    for (i, path) in app.queue_items.iter().enumerate() {
        let position = i + 1;
        let status_cell =
            Cell::from(format!("#{}", position)).style(Style::default().fg(app.colors.row_fg));
        let path_cell = Cell::from(path.clone());
        rows.push(Row::new(vec![status_cell, path_cell]).style(Style::default().fg(app.colors.row_fg)));
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

fn render_stats_section<'a>(app: &App, _area: ratatui::layout::Rect) -> ratatui::widgets::Table<'a> {
    use ratatui::style::{Color, Modifier, Style};
    use ratatui::widgets::{Block, Borders, Cell, Row, Table};

    let mut rows = Vec::new();

    // Use total_records which is the accurate count of all transcripts (without filters)
    let total_transcripts = app.total_records as usize;

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
    ]);
    rows.push(total_row);

    // Add a separator row
    let separator_row = Row::new(vec![
        Cell::from("─".repeat(30)),
        Cell::from("─".repeat(10)),
    ])
    .style(Style::default().fg(app.colors.footer_border_color));
    rows.push(separator_row);

    // Query database for video counts per watch directory
    match get_directory_stats() {
        Ok(dir_counts) => {
            if dir_counts.is_empty() {
                let empty_row = Row::new(vec![
                    Cell::from("No watch directories configured"),
                    Cell::from(""),
                ])
                .style(Style::default().fg(Color::Gray));
                rows.push(empty_row);
            } else {
                for (dir, count) in dir_counts {
                    // Truncate directory path if too long, show last part
                    let display_dir = if dir.len() > 35 {
                        format!("...{}", &dir[dir.len() - 32..])
                    } else {
                        dir
                    };

                    let dir_row = Row::new(vec![
                        Cell::from(display_dir).style(Style::default().fg(app.colors.row_fg)),
                        Cell::from(count.to_string()).style(Style::default().fg(Color::Green)),
                    ]);
                    rows.push(dir_row);
                }
            }
        }
        Err(e) => {
            let error_row = Row::new(vec![
                Cell::from(format!("Error: {}", e)),
                Cell::from(""),
            ])
            .style(Style::default().fg(Color::Red));
            rows.push(error_row);
        }
    }

    let widths = [Constraint::Min(25), Constraint::Length(10)];

    let block = Block::default()
        .title("Stats")
        .borders(Borders::ALL)
        .border_style(Style::new().fg(app.colors.footer_border_color));

    Table::new(rows, widths)
        .block(block)
        .column_spacing(1)
}

fn get_directory_stats() -> Result<Vec<(String, usize)>, Box<dyn std::error::Error>> {
    use crate::db;

    let conn = db::get_connection()?;
    let mut stmt = conn.prepare(
        "SELECT watch_directory, COUNT(*) as count
         FROM video_info
         WHERE watch_directory IS NOT NULL
         GROUP BY watch_directory
         ORDER BY watch_directory"
    )?;

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i64>(1)? as usize,
        ))
    })?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row?);
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
