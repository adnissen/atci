use crate::transcripts;
use crate::tui::{create_tab_title_with_editor, App};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

#[derive(Clone, Debug, PartialEq)]
pub enum TimestampPosition {
    Start,  // First timestamp (before -->)
    End,    // Second timestamp (after -->)
}

#[derive(Clone)]
pub struct FileViewData {
    pub video_path: String,
    pub lines: Vec<String>,
    pub selected_line: usize,
    pub scroll_offset: usize,
    pub list_state: ListState,
    pub selected_timestamp_position: Option<TimestampPosition>,
}

fn is_timestamp_line(line: &str) -> bool {
    // Check for VTT format: "xx:xx:xx.xxx --> xx:xx:xx.xxx"
    if !line.contains(" --> ") {
        return false;
    }
    
    // Split on the arrow and check both timestamps
    let parts: Vec<&str> = line.split(" --> ").collect();
    if parts.len() != 2 {
        return false;
    }
    
    // Check if both parts match timestamp format (xx:xx:xx.xxx)
    parts.iter().all(|part| is_timestamp_format(part.trim()))
}

fn is_timestamp_format(s: &str) -> bool {
    // Match format: xx:xx:xx.xxx (e.g., "00:05:25.920")
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 3 {
        return false;
    }
    
    // Check hours and minutes are 2 digits
    if parts[0].len() != 2 || parts[1].len() != 2 {
        return false;
    }
    
    if !parts[0].chars().all(|c| c.is_ascii_digit()) || !parts[1].chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    
    // Check seconds.milliseconds format (xx.xxx)
    let seconds_parts: Vec<&str> = parts[2].split('.').collect();
    if seconds_parts.len() != 2 {
        return false;
    }
    
    // Seconds should be 2 digits, milliseconds should be 3 digits
    seconds_parts[0].len() == 2 && 
    seconds_parts[1].len() == 3 &&
    seconds_parts[0].chars().all(|c| c.is_ascii_digit()) &&
    seconds_parts[1].chars().all(|c| c.is_ascii_digit())
}

fn create_timestamp_spans(line: &str, selected_position: &Option<TimestampPosition>, line_index: usize, selected_line: usize) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    
    if !is_timestamp_line(line) {
        spans.push(Span::styled(line.to_string(), Style::default().fg(Color::Green)));
        return spans;
    }
    
    // Split on " --> " to get start and end timestamps
    let parts: Vec<&str> = line.split(" --> ").collect();
    if parts.len() != 2 {
        spans.push(Span::styled(line.to_string(), Style::default().fg(Color::Green)));
        return spans;
    }
    
    let start_timestamp = parts[0];
    let end_timestamp = parts[1];
    
    // Determine if we should highlight individual timestamps
    let should_highlight = line_index == selected_line && selected_position.is_some();
    
    if should_highlight {
        match selected_position {
            Some(TimestampPosition::Start) => {
                // Highlight start timestamp with blue background, normal end timestamp
                spans.push(Span::styled(
                    start_timestamp.to_string(),
                    Style::default().fg(Color::White).bg(Color::Blue).add_modifier(Modifier::BOLD)
                ));
                spans.push(Span::styled(" --> ".to_string(), Style::default().fg(Color::Green)));
                spans.push(Span::styled(
                    end_timestamp.to_string(),
                    Style::default().fg(Color::Green)
                ));
            }
            Some(TimestampPosition::End) => {
                // Normal start timestamp, highlight end timestamp with blue background
                spans.push(Span::styled(
                    start_timestamp.to_string(),
                    Style::default().fg(Color::Green)
                ));
                spans.push(Span::styled(" --> ".to_string(), Style::default().fg(Color::Green)));
                spans.push(Span::styled(
                    end_timestamp.to_string(),
                    Style::default().fg(Color::White).bg(Color::Blue).add_modifier(Modifier::BOLD)
                ));
            }
            None => {
                // This shouldn't happen, but fallback to normal green
                spans.push(Span::styled(line.to_string(), Style::default().fg(Color::Green)));
            }
        }
    } else {
        // Normal timestamp line coloring
        spans.push(Span::styled(line.to_string(), Style::default().fg(Color::Green)));
    }
    
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_detection() {
        // Valid VTT timestamp lines
        assert!(is_timestamp_line("00:05:25.920 --> 00:05:46.060"));
        assert!(is_timestamp_line("01:23:45.123 --> 01:24:00.456"));
        assert!(is_timestamp_line("00:00:00.000 --> 00:00:05.500"));
        
        // Invalid lines
        assert!(!is_timestamp_line("This is a regular text line"));
        assert!(!is_timestamp_line("00:05:25 --> 00:05:46"));  // Missing milliseconds
        assert!(!is_timestamp_line("0:5:25.920 --> 0:5:46.060"));  // Wrong format
        assert!(!is_timestamp_line("00:05:25.920 -> 00:05:46.060"));  // Wrong arrow
        assert!(!is_timestamp_line("00:05:25.920"));  // Only one timestamp
        assert!(!is_timestamp_line(""));  // Empty line
    }

    #[test]
    fn test_individual_timestamp_navigation() {
        let lines = vec![
            "Some text".to_string(),
            "00:05:25.920 --> 00:05:46.060".to_string(),
            "More text".to_string(),
            "01:23:45.123 --> 01:24:00.456".to_string(),
        ];
        
        let mut file_data = FileViewData {
            video_path: "test.mp4".to_string(),
            lines,
            selected_line: 1,  // Start on first timestamp line
            scroll_offset: 0,
            list_state: ListState::default(),
            selected_timestamp_position: None,
        };
        
        // Navigate to nearest timestamp should select start position
        file_data.navigate_to_nearest_timestamp();
        assert_eq!(file_data.selected_timestamp_position, Some(TimestampPosition::Start));
        assert_eq!(file_data.selected_line, 1);
        
        // Navigate to next should move to end position on same line
        file_data.navigate_to_next_timestamp();
        assert_eq!(file_data.selected_timestamp_position, Some(TimestampPosition::End));
        assert_eq!(file_data.selected_line, 1);
        
        // Navigate to next again should move to start of next timestamp line
        file_data.navigate_to_next_timestamp();
        assert_eq!(file_data.selected_timestamp_position, Some(TimestampPosition::Start));
        assert_eq!(file_data.selected_line, 3);
        
        // Navigate to previous should move to end of previous timestamp line
        file_data.navigate_to_previous_timestamp();
        assert_eq!(file_data.selected_timestamp_position, Some(TimestampPosition::End));
        assert_eq!(file_data.selected_line, 1);
        
        // Navigate to previous should move to start position on same line
        file_data.navigate_to_previous_timestamp();
        assert_eq!(file_data.selected_timestamp_position, Some(TimestampPosition::Start));
        assert_eq!(file_data.selected_line, 1);
    }
}

impl FileViewData {
    pub fn new(video_path: String) -> Result<Self, Box<dyn std::error::Error>> {
        let transcript_content = transcripts::get_transcript(&video_path)?;
        let lines: Vec<String> = transcript_content.lines().map(|s| s.to_string()).collect();
        
        let mut list_state = ListState::default();
        if !lines.is_empty() {
            list_state.select(Some(0));
        }
        
        Ok(FileViewData {
            video_path,
            lines,
            selected_line: 0,
            scroll_offset: 0,
            list_state,
            selected_timestamp_position: None,
        })
    }
    
    fn find_timestamp_lines(&self) -> Vec<usize> {
        self.lines
            .iter()
            .enumerate()
            .filter_map(|(i, line)| if is_timestamp_line(line) { Some(i) } else { None })
            .collect()
    }
    
    pub fn navigate_to_nearest_timestamp(&mut self) {
        let timestamp_lines = self.find_timestamp_lines();
        if timestamp_lines.is_empty() {
            return;
        }
        
        // Find the nearest timestamp line
        let current_line = self.selected_line;
        let nearest = timestamp_lines
            .iter()
            .min_by_key(|&&line| {
                if line >= current_line {
                    line - current_line
                } else {
                    current_line - line
                }
            });
        
        if let Some(&nearest_line) = nearest {
            self.selected_line = nearest_line;
            self.list_state.select(Some(nearest_line));
            // Start with the first timestamp (start time)
            self.selected_timestamp_position = Some(TimestampPosition::Start);
        }
    }
    
    pub fn navigate_to_next_timestamp(&mut self) {
        let timestamp_lines = self.find_timestamp_lines();
        if timestamp_lines.is_empty() {
            return;
        }
        
        // If we're currently on a timestamp line, toggle between start/end positions
        if is_timestamp_line(&self.lines[self.selected_line]) {
            match self.selected_timestamp_position {
                Some(TimestampPosition::Start) => {
                    // Move to end timestamp of same line
                    self.selected_timestamp_position = Some(TimestampPosition::End);
                    return;
                }
                Some(TimestampPosition::End) => {
                    // Move to next timestamp line's start timestamp
                    let current_line = self.selected_line;
                    let next_timestamp = timestamp_lines
                        .iter()
                        .find(|&&line| line > current_line);
                    
                    if let Some(&next_line) = next_timestamp {
                        self.selected_line = next_line;
                        self.list_state.select(Some(next_line));
                        self.selected_timestamp_position = Some(TimestampPosition::Start);
                    } else {
                        // Wrap to first timestamp
                        if let Some(&first_line) = timestamp_lines.first() {
                            self.selected_line = first_line;
                            self.list_state.select(Some(first_line));
                            self.selected_timestamp_position = Some(TimestampPosition::Start);
                        }
                    }
                    return;
                }
                None => {
                    // Start timestamp navigation on current line if it's a timestamp line
                    self.selected_timestamp_position = Some(TimestampPosition::Start);
                    return;
                }
            }
        }
        
        // Find the next timestamp after current line
        let current_line = self.selected_line;
        let next_timestamp = timestamp_lines
            .iter()
            .find(|&&line| line > current_line);
        
        if let Some(&next_line) = next_timestamp {
            self.selected_line = next_line;
            self.list_state.select(Some(next_line));
            self.selected_timestamp_position = Some(TimestampPosition::Start);
        } else {
            // Wrap to first timestamp
            if let Some(&first_line) = timestamp_lines.first() {
                self.selected_line = first_line;
                self.list_state.select(Some(first_line));
                self.selected_timestamp_position = Some(TimestampPosition::Start);
            }
        }
    }
    
    pub fn navigate_to_previous_timestamp(&mut self) {
        let timestamp_lines = self.find_timestamp_lines();
        if timestamp_lines.is_empty() {
            return;
        }
        
        // If we're currently on a timestamp line, toggle between start/end positions
        if is_timestamp_line(&self.lines[self.selected_line]) {
            match self.selected_timestamp_position {
                Some(TimestampPosition::End) => {
                    // Move to start timestamp of same line
                    self.selected_timestamp_position = Some(TimestampPosition::Start);
                    return;
                }
                Some(TimestampPosition::Start) => {
                    // Move to previous timestamp line's end timestamp
                    let current_line = self.selected_line;
                    let prev_timestamp = timestamp_lines
                        .iter()
                        .rev()
                        .find(|&&line| line < current_line);
                    
                    if let Some(&prev_line) = prev_timestamp {
                        self.selected_line = prev_line;
                        self.list_state.select(Some(prev_line));
                        self.selected_timestamp_position = Some(TimestampPosition::End);
                    } else {
                        // Wrap to last timestamp
                        if let Some(&last_line) = timestamp_lines.last() {
                            self.selected_line = last_line;
                            self.list_state.select(Some(last_line));
                            self.selected_timestamp_position = Some(TimestampPosition::End);
                        }
                    }
                    return;
                }
                None => {
                    // Start timestamp navigation on current line if it's a timestamp line
                    self.selected_timestamp_position = Some(TimestampPosition::Start);
                    return;
                }
            }
        }
        
        // Find the previous timestamp before current line
        let current_line = self.selected_line;
        let prev_timestamp = timestamp_lines
            .iter()
            .rev()
            .find(|&&line| line < current_line);
        
        if let Some(&prev_line) = prev_timestamp {
            self.selected_line = prev_line;
            self.list_state.select(Some(prev_line));
            self.selected_timestamp_position = Some(TimestampPosition::End);
        } else {
            // Wrap to last timestamp
            if let Some(&last_line) = timestamp_lines.last() {
                self.selected_line = last_line;
                self.list_state.select(Some(last_line));
                self.selected_timestamp_position = Some(TimestampPosition::End);
            }
        }
    }
    
    pub fn navigate_up(&mut self) {
        if self.selected_line > 0 {
            self.selected_line -= 1;
            self.list_state.select(Some(self.selected_line));
        }
        self.selected_timestamp_position = None;
    }
    
    pub fn navigate_down(&mut self) {
        if self.selected_line < self.lines.len().saturating_sub(1) {
            self.selected_line += 1;
            self.list_state.select(Some(self.selected_line));
        }
        self.selected_timestamp_position = None;
    }
    
    pub fn jump_to_top(&mut self) {
        self.selected_line = 0;
        self.scroll_offset = 0;
        self.list_state.select(Some(0));
        self.selected_timestamp_position = None;
    }
    
    pub fn jump_to_bottom(&mut self) {
        if !self.lines.is_empty() {
            self.selected_line = self.lines.len() - 1;
            self.list_state.select(Some(self.selected_line));
        }
        self.selected_timestamp_position = None;
    }
    
    pub fn page_up(&mut self, page_size: usize) {
        let new_line = self.selected_line.saturating_sub(page_size);
        self.selected_line = new_line;
        self.list_state.select(Some(self.selected_line));
    }
    
    pub fn page_down(&mut self, page_size: usize) {
        let max_line = self.lines.len().saturating_sub(1);
        let new_line = std::cmp::min(self.selected_line + page_size, max_line);
        self.selected_line = new_line;
        self.list_state.select(Some(self.selected_line));
    }
    
    pub fn jump_to_line(&mut self, line_number: usize) {
        let target_line = line_number.saturating_sub(1); // Convert 1-based to 0-based
        if target_line < self.lines.len() {
            self.selected_line = target_line;
            self.list_state.select(Some(target_line));
        }
    }
    
    pub fn get_timestamp_for_current_line(&self) -> Option<String> {
        // Check if current line has timestamps
        if let Some(current_line) = self.lines.get(self.selected_line) {
            if is_timestamp_line(current_line) {
                return Some(current_line.clone());
            }
        }
        
        // Check if the line above has timestamps
        if self.selected_line > 0 {
            if let Some(previous_line) = self.lines.get(self.selected_line - 1) {
                if is_timestamp_line(previous_line) {
                    return Some(previous_line.clone());
                }
            }
        }
        
        None
    }
    
    pub fn get_text_for_current_line(&self) -> String {
        self.lines.get(self.selected_line)
            .cloned()
            .unwrap_or_default()
    }
}

pub fn render_file_view_tab(f: &mut Frame, area: Rect, app: &mut App) {
    let title = create_tab_title_with_editor(
        app.current_tab, 
        &app.colors, 
        !app.search_results.is_empty(), 
        app.editor_data.is_some(), 
        app.file_view_data.is_some()
    );

    if let Some(file_data) = &mut app.file_view_data {
        // Split the main content area into sections
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3), // File info section
                Constraint::Min(1),    // Content area
            ].as_ref())
            .split(area);

        // Create main block with tab title
        let main_block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::new().fg(app.colors.footer_border_color));

        f.render_widget(main_block, area);

        // File info section
        let file_info = format!("File: {} | Lines: {} | Current Line: {}", 
            file_data.video_path, 
            file_data.lines.len(),
            file_data.selected_line + 1
        );
        
        let info_paragraph = Paragraph::new(file_info)
            .block(
                Block::default()
                    .title("File Information")
                    .borders(Borders::ALL)
                    .border_style(Style::new().fg(app.colors.footer_border_color))
            )
            .style(Style::new().fg(app.colors.row_fg))
            .alignment(Alignment::Left);
        
        f.render_widget(info_paragraph, main_chunks[0]);
        
        // Content area with line numbers and text
        let items: Vec<ListItem> = file_data.lines
            .iter()
            .enumerate()
            .map(|(i, line)| {
                let mut spans = Vec::new();
                
                // Line number in gray
                spans.push(Span::styled(
                    format!("{:4}: ", i + 1),
                    Style::default().fg(Color::Gray)
                ));
                
                // Check if this line is a timestamp
                if is_timestamp_line(line) {
                    // Use the timestamp highlighting function
                    let timestamp_spans = create_timestamp_spans(
                        line, 
                        &file_data.selected_timestamp_position, 
                        i, 
                        file_data.selected_line
                    );
                    spans.extend(timestamp_spans);
                } else {
                    // Regular text
                    spans.push(Span::raw(line.clone()));
                }
                
                ListItem::new(Line::from(spans))
            })
            .collect();
        
        let content_block = Block::default()
            .title("Transcript Content (↑↓/jk: Navigate, h/l: Timestamps, c: Open Editor)")
            .borders(Borders::ALL)
            .border_style(Style::new().fg(app.colors.footer_border_color));
        
        let list = if file_data.selected_timestamp_position.is_some() {
            // When in timestamp mode, don't apply default line highlighting
            List::new(items)
                .block(content_block)
                .style(Style::new().fg(app.colors.row_fg))
        } else {
            // Normal line highlighting when not in timestamp mode
            List::new(items)
                .block(content_block)
                .style(Style::new().fg(app.colors.row_fg))
                .highlight_style(
                    Style::default()
                        .add_modifier(Modifier::REVERSED)
                        .fg(app.colors.selected_style_fg)
                )
        };
        
        f.render_stateful_widget(list, main_chunks[1], &mut file_data.list_state);
    } else {
        // Show empty state when no file view data
        let empty_block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::new().fg(app.colors.footer_border_color));

        let empty_content = "No file opened in file view.";
        let empty_paragraph = Paragraph::new(empty_content)
            .block(empty_block)
            .style(Style::new().fg(app.colors.row_fg))
            .alignment(Alignment::Center);

        f.render_widget(empty_paragraph, area);
    }
}