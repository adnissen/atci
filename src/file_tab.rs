use crate::transcripts;
use crate::tui::{create_tab_title_with_editor, App};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

#[derive(Clone)]
pub struct FileViewData {
    pub video_path: String,
    pub lines: Vec<String>,
    pub selected_line: usize,
    pub scroll_offset: usize,
    pub list_state: ListState,
}

fn is_timestamp_line(line: &str) -> bool {
    // Check if line looks like a timestamp
    // Common patterns: "00:05:25.920 --> 00:05:46.060" or "126: 00:05:25.920 --> 00:05:46.060"
    line.contains("-->") || 
    (line.contains(':') && 
     line.chars().any(|c| c.is_ascii_digit()) && 
     line.matches(':').count() >= 2 &&
     // Check for time format like HH:MM:SS
     line.split(':').any(|part| part.chars().all(|c| c.is_ascii_digit() || c == '.')))
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
        })
    }
    
    pub fn navigate_up(&mut self) {
        if self.selected_line > 0 {
            self.selected_line -= 1;
            self.list_state.select(Some(self.selected_line));
        }
    }
    
    pub fn navigate_down(&mut self) {
        if self.selected_line < self.lines.len().saturating_sub(1) {
            self.selected_line += 1;
            self.list_state.select(Some(self.selected_line));
        }
    }
    
    pub fn jump_to_top(&mut self) {
        self.selected_line = 0;
        self.scroll_offset = 0;
        self.list_state.select(Some(0));
    }
    
    pub fn jump_to_bottom(&mut self) {
        if !self.lines.is_empty() {
            self.selected_line = self.lines.len() - 1;
            self.list_state.select(Some(self.selected_line));
        }
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
                    // Color the entire timestamp line in green
                    spans.push(Span::styled(
                        line.clone(),
                        Style::default().fg(Color::Green)
                    ));
                } else {
                    // Regular text
                    spans.push(Span::raw(line.clone()));
                }
                
                ListItem::new(Line::from(spans))
            })
            .collect();
        
        let content_block = Block::default()
            .title("Transcript Content (↑↓/jk: Navigate, ←→/hl: Page)")
            .borders(Borders::ALL)
            .border_style(Style::new().fg(app.colors.footer_border_color));
        
        let list = List::new(items)
            .block(content_block)
            .style(Style::new().fg(app.colors.row_fg))
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::REVERSED)
                    .fg(app.colors.selected_style_fg)
            );
        
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