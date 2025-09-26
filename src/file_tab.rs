use crate::transcripts;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
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

pub fn render_file_view_tab(f: &mut Frame, area: Rect, file_data: &mut FileViewData, colors: &crate::tui::TableColors) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header with file info
            Constraint::Min(1),    // Content area
        ])
        .split(area);
    
    // Header with file info
    let file_info = format!("File: {} | Lines: {} | Current Line: {}", 
        file_data.video_path, 
        file_data.lines.len(),
        file_data.selected_line + 1
    );
    
    let header_block = Block::default()
        .title("File View")
        .borders(Borders::ALL)
        .border_style(Style::new().fg(colors.footer_border_color));
    
    let header_paragraph = Paragraph::new(file_info)
        .block(header_block)
        .style(Style::new().fg(colors.row_fg))
        .alignment(Alignment::Left);
    
    f.render_widget(header_paragraph, chunks[0]);
    
    // Content area with line numbers and text
    let items: Vec<ListItem> = file_data.lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let line_number = format!("{:4}: ", i + 1);
            let content = format!("{}{}", line_number, line);
            ListItem::new(content)
        })
        .collect();
    
    let content_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(colors.footer_border_color));
    
    let list = List::new(items)
        .block(content_block)
        .style(Style::new().fg(colors.row_fg))
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::REVERSED)
                .fg(colors.selected_style_fg)
        );
    
    f.render_stateful_widget(list, chunks[1], &mut file_data.list_state);
}