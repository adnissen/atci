use crate::search;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Paragraph};
use throbber_widgets_tui::Throbber;
use tui_big_text::BigText;

use crate::tui::{App, TabState, create_tab_title_with_editor};

impl App {
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
        if !self.search_input.is_empty() {
            self.search_requested = true;
            self.search_in_progress = true;
            // Switch to search results tab (only if we're not already there)
            if self.current_tab != TabState::SearchResults {
                self.current_tab = TabState::SearchResults;
                // Update total records count with current filter
                self.update_total_records();
            }
        }
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

    pub fn perform_search(&mut self) {
        // Clear the requested flag
        self.search_requested = false;

        // Perform actual search functionality
        if !self.search_input.is_empty() {
            let search_input = self.search_input.clone();
            let filter = self.get_filter_option();

            // Use std::thread to completely avoid runtime nesting issues
            // Spawn the thread without waiting - store handle for later polling
            let handle = std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new()
                    .map_err(|e| format!("Failed to create runtime: {}", e))?;
                rt.block_on(search::search(&search_input, filter.as_ref(), false, false))
                    .map_err(|e| format!("Search failed: {}", e))
            });

            // Store the thread handle for polling
            self.search_thread = Some(handle);
        } else {
            // If search input is empty, clear results (but only on SearchResults tab)
            if self.current_tab == TabState::SearchResults {
                self.search_results.clear();
                self.search_selected_index = 0;
                self.search_scroll_offset = 0;
                self.last_search_query.clear();
            }
            self.search_in_progress = false;
        }
    }

    pub fn check_search_thread(&mut self) {
        // Check if search thread is finished
        if let Some(handle) = self.search_thread.take() {
            if handle.is_finished() {
                // Thread is done, get the results
                match handle.join() {
                    Ok(Ok(results)) => {
                        self.search_results = results;
                        self.search_selected_index = 0;
                        self.search_scroll_offset = 0;
                        self.last_search_query = self.search_input.clone();
                        self.search_in_progress = false;
                    }
                    Ok(Err(e)) => {
                        eprintln!("{}", e);
                        self.search_results.clear();
                        self.search_in_progress = false;
                    }
                    Err(e) => {
                        eprintln!("Thread join failed: {:?}", e);
                        self.search_results.clear();
                        self.search_in_progress = false;
                    }
                }
            } else {
                // Thread still running, put handle back
                self.search_thread = Some(handle);
            }
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

    pub fn open_editor_from_selected_match(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(search_match) = self.get_selected_search_match() {
            if let Some(timestamp) = &search_match.timestamp {
                // Parse timestamp from line like "126: 00:05:25.920 --> 00:05:46.060"
                if let Some((start_time, end_time)) = self.parse_timestamp_range(timestamp) {
                    // Open editor with the match information
                    self.open_editor(
                        start_time,
                        end_time,
                        search_match.line_text.clone(),
                        search_match.video_info.full_path.clone(),
                    );
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

    pub fn show_clip_url_popup_from_selected_match(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(search_match) = self.get_selected_search_match() {
            if let Some(timestamp) = &search_match.timestamp {
                // Parse timestamp from line like "126: 00:05:25.920 --> 00:05:46.060"
                if let Some((start_time, end_time)) = self.parse_timestamp_range(timestamp) {
                    // Show clip URL popup with the match information
                    self.show_clip_url_popup(
                        search_match.video_info.full_path.clone(),
                        start_time,
                        end_time,
                        search_match.line_text.clone(),
                    );
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

    pub fn open_file_view_from_selected_match(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(search_match) = self.get_selected_search_match() {
            let video_path = search_match.video_info.full_path.clone();
            let line_number = search_match.line_number;

            // Open file view with the video path
            self.open_file_view(&video_path)?;

            // Jump to the specific line number
            if let Some(file_data) = &mut self.file_view_data {
                file_data.jump_to_line(line_number);
            }

            // Switch to file view tab
            self.current_tab = crate::tui::TabState::FileView;
        } else {
            return Err("No search result selected".into());
        }

        Ok(())
    }

    pub fn parse_timestamp_range(&self, timestamp_line: &str) -> Option<(String, String)> {
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
}

pub fn render_search_results_tab(f: &mut Frame, area: ratatui::layout::Rect, app: &mut App) {
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
                Constraint::Length(3), // Search query info
                Constraint::Min(5),    // Results section (expandable)
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

    // Search query info section
    let query_info = if app.search_results.is_empty() {
        if app.last_search_query.is_empty() {
            format!("Use '/' to search {} transcripts", app.total_records)
        } else {
            format!("No results found for: '{}'", app.last_search_query)
        }
    } else {
        let total_matches: usize = app.search_results.iter().map(|r| r.matches.len()).sum();
        format!(
            "Search: '{}' - {} matches in {} files",
            app.last_search_query,
            total_matches,
            app.search_results.len()
        )
    };

    let query_paragraph = Paragraph::new(query_info)
        .block(
            Block::default()
                .title("Search Query")
                .borders(Borders::ALL)
                .border_style(Style::new().fg(app.colors.footer_border_color)),
        )
        .style(Style::new().fg(app.colors.row_fg))
        .alignment(Alignment::Center);

    f.render_widget(query_paragraph, main_chunks[0]);

    // Results section
    if !app.search_results.is_empty() && !app.search_in_progress {
        let results_content = render_search_results_list(app);
        let results_paragraph = Paragraph::new(results_content)
            .block(
                Block::default()
                    .title("Results (↑↓/jk: Navigate)")
                    .borders(Borders::ALL)
                    .border_style(Style::new().fg(app.colors.footer_border_color)),
            )
            .style(Style::new().fg(app.colors.row_fg))
            .alignment(Alignment::Left)
            .wrap(ratatui::widgets::Wrap { trim: true })
            .scroll((app.search_scroll_offset as u16, 0));

        f.render_widget(results_paragraph, main_chunks[1]);
    } else {
        // Show big text banner
        let big_text = BigText::builder()
            .lines(vec!["atci".into()])
            .style(Style::new().fg(app.colors.row_fg))
            .centered()
            .build();

        f.render_widget(big_text, main_chunks[1]);

        // If searching, overlay the throbber
        if app.search_in_progress {
            app.throbber_state.calc_next();

            let throbber = Throbber::default()
                .label("Searching...")
                .style(Style::new().fg(app.colors.row_fg))
                .throbber_style(Style::new().fg(ratatui::style::Color::Yellow));

            // Calculate center position for throbber (below the big text)
            let throbber_area = ratatui::layout::Rect {
                x: main_chunks[1].x + main_chunks[1].width / 2 - 10,
                y: main_chunks[1].y + main_chunks[1].height * 2 / 3,
                width: 20,
                height: 1,
            };

            f.render_stateful_widget(throbber, throbber_area, &mut app.throbber_state);
        }
    }
}

fn render_search_results_list(app: &App) -> ratatui::text::Text<'static> {
    use ratatui::style::{Color, Style};
    use ratatui::text::{Line, Span, Text};

    let mut lines = Vec::new();
    let mut current_match_index = 0;

    for result in &app.search_results {
        // File header
        lines.push(Line::from(vec![Span::styled(
            format!("{}", result.file_path),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]));

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
                Style::default().fg(Color::Gray),
            ));

            // Timestamp if available
            if let Some(timestamp) = &search_match.timestamp {
                spans.push(Span::styled(
                    format!("[{}] ", timestamp.trim()),
                    Style::default().fg(Color::Green),
                ));
            }

            // Match text (highlight the search term if possible)
            let line_text = &search_match.line_text;
            if is_selected {
                spans.push(Span::styled(
                    line_text.clone(),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
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
        lines.push(Line::from(vec![Span::styled(
            "No search results",
            Style::default().fg(Color::Gray),
        )]));
    }

    Text::from(lines)
}
