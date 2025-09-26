use crate::search;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::style::{Style, Modifier};
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::Frame;

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
        self.perform_search();
    }

    pub fn add_char_to_search(&mut self, c: char) {
        if self.search_input_mode {
            self.search_input.push(c);
            // Re-run search immediately as user types, but only on SearchResults tab
            if self.current_tab == TabState::SearchResults {
                self.perform_search();
            }
        }
    }

    pub fn remove_char_from_search(&mut self) {
        if self.search_input_mode {
            self.search_input.pop();
            // Re-run search immediately as user deletes, but only on SearchResults tab
            if self.current_tab == TabState::SearchResults {
                self.perform_search();
            }
        }
    }

    pub fn perform_search(&mut self) {
        // Perform actual search functionality
        if !self.search_input.is_empty() {
            match search::search(&self.search_input, self.get_filter_option().as_ref(), false, false) {
                Ok(results) => {
                    self.search_results = results;
                    self.search_selected_index = 0;
                    self.search_scroll_offset = 0;
                    self.last_search_query = self.search_input.clone();
                    
                    // Switch to search results tab (only if we're not already there)
                    if self.current_tab != TabState::SearchResults {
                        self.current_tab = TabState::SearchResults;
                    }
                }
                Err(e) => {
                    eprintln!("Search failed: {}", e);
                    // Keep search results empty on error
                    self.search_results.clear();
                }
            }
        } else {
            // If search input is empty, clear results (but only on SearchResults tab)
            if self.current_tab == TabState::SearchResults {
                self.search_results.clear();
                self.search_selected_index = 0;
                self.search_scroll_offset = 0;
                self.last_search_query.clear();
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

}

pub fn render_search_results_tab(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let title = create_tab_title_with_editor(app.current_tab, &app.colors, !app.search_results.is_empty(), app.editor_data.is_some());

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
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
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
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
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