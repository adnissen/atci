use crate::config;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::style::{Style, Modifier};
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::Frame;
use std::{error::Error, fs, time::Duration};

use crate::tui::{App, SystemService, ServiceStatus, SystemSection, create_tab_title_with_editor};

impl App {
    pub fn system_next(&mut self) {
        match self.system_section {
            SystemSection::Services => {
                if !self.system_services.is_empty() && self.system_selected_index < self.system_services.len() - 1 {
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
        if self.system_selected_index >= self.system_services.len() && !self.system_services.is_empty() {
            self.system_selected_index = self.system_services.len() - 1;
        }
        self.last_system_refresh = std::time::Instant::now();
    }

    pub fn should_refresh_system_services(&self) -> bool {
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

pub fn get_atci_dir() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let home_dir = dirs::home_dir().ok_or("Could not find home directory")?;
    let atci_dir = home_dir.join(".atci");
    Ok(atci_dir)
}

pub fn find_existing_pid_files() -> Result<Vec<u32>, Box<dyn std::error::Error>> {
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

pub fn render_system_tab(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let title = create_tab_title_with_editor(app.current_tab, &app.colors, !app.search_results.is_empty(), app.editor_data.is_some(), app.file_view_data.is_some());

    // Split the main content area into sections
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(8),  // Services section (smaller)
            Constraint::Min(10),    // Config section (expandable)
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
                .border_style(Style::new().fg(services_border_color))
        )
        .style(Style::new().fg(app.colors.row_fg))
        .alignment(Alignment::Left);

    f.render_widget(services_paragraph, main_chunks[0]);

    // Config editing section
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
                .border_style(Style::new().fg(config_border_color))
        )
        .style(Style::new().fg(app.colors.row_fg))
        .alignment(Alignment::Left);

    f.render_widget(config_paragraph, main_chunks[1]);
}

fn render_services_list(app: &App) -> ratatui::text::Text<'static> {
    use ratatui::text::{Line, Span, Text};
    use ratatui::style::{Color, Style};

    let mut lines = Vec::new();

    for (index, service) in app.system_services.iter().enumerate() {
        let is_selected = index == app.system_selected_index && app.system_section == SystemSection::Services;
        
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
                        spans.push(Span::styled("← [KILL]", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)));
                    }
                }
            }
            ServiceStatus::Stopped => {
                spans.push(Span::styled("stopped", Style::default().fg(Color::Red)));
                
                // Show start option if selected
                if is_selected {
                    spans.push(Span::raw(" "));
                    spans.push(Span::styled("← [START]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)));
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

fn render_config_section(app: &App) -> ratatui::text::Text<'static> {
    use ratatui::text::{Line, Span, Text};
    use ratatui::style::{Color, Style};

    let mut lines = Vec::new();
    let field_names = app.get_config_field_names();

    for (index, field_name) in field_names.iter().enumerate() {
        let is_selected = index == app.config_selected_field && app.system_section == SystemSection::Config;
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
            Style::default().fg(Color::Cyan)
        ));

        // Field value
        let field_value = if app.config_editing_mode && is_selected {
            app.config_input_buffer.clone()
        } else {
            app.get_config_field_value(index)
        };

        let value_style = if app.config_editing_mode && is_selected {
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
        } else if is_selected {
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        // Truncate long values for display
        let display_value = if field_value.len() > 60 {
            format!("{}...", &field_value[..57])
        } else {
            field_value
        };

        spans.push(Span::styled(display_value, value_style));

        // Show editing indicator
        if app.config_editing_mode && is_selected {
            spans.push(Span::styled(" [EDITING]", Style::default().fg(Color::Green)));
        }

        lines.push(Line::from(spans));
    }

    if lines.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("No config fields found", Style::default().fg(Color::Gray))
        ]));
    }

    Text::from(lines)
}