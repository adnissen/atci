use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::style::Style;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use std::path::Path;
use ratatui::Frame;
use ratatui_image::{picker::Picker, StatefulImage, protocol::StatefulProtocol, Resize};

use crate::tui::{App, TabState, create_tab_title_with_editor};
use crate::clipper;

pub struct EditorData {
    pub start_time: String,
    pub end_time: String,
    pub text: String,
    pub file_path: String,
    pub start_frame_path: Option<std::path::PathBuf>,
    pub start_frame: Option<StatefulProtocol>,
    pub end_frame_path: Option<std::path::PathBuf>,
    pub end_frame: Option<StatefulProtocol>,
    pub show_overlay_text: bool,
}

impl App {
    pub fn open_editor(&mut self, start_time: String, end_time: String, text: String, file_path: String) {
        // Generate frame images with text overlay by default
        let video_path = Path::new(&file_path);
        let start_frame_path = clipper::grab_frame(video_path, &start_time, Some(&text), None).ok();
        let end_frame_path = clipper::grab_frame(video_path, &end_time, Some(&text), None).ok();
        
        // Load start frame if available
        let start_frame = start_frame_path.as_ref()
            .and_then(|path| {
                let picker = Picker::from_fontsize((8, 12));
                image::ImageReader::open(path)
                    .ok()
                    .and_then(|reader| reader.decode().ok())
                    .map(|img| picker.new_resize_protocol(img))
            });
            
        // Load end frame if available - use separate picker instance
        let end_frame = end_frame_path.as_ref()
            .and_then(|path| {
                let picker = Picker::from_fontsize((8, 12));
                image::ImageReader::open(path)
                    .ok()
                    .and_then(|reader| reader.decode().ok())
                    .map(|img| picker.new_resize_protocol(img))
            });
        
        self.editor_data = Some(EditorData {
            start_time,
            end_time,
            text,
            file_path,
            start_frame_path,
            start_frame,
            end_frame_path,
            end_frame,
            show_overlay_text: true, // Default to showing overlay text
        });
        self.current_tab = TabState::Editor;
    }

    pub fn switch_to_editor(&mut self) {
        if self.editor_data.is_some() {
            self.current_tab = TabState::Editor;
        }
    }
    
    pub fn toggle_editor_overlay(&mut self) {
        if let Some(ref mut editor_data) = self.editor_data {
            editor_data.show_overlay_text = !editor_data.show_overlay_text;
            // Regenerate frames with new overlay setting
            self.regenerate_editor_frames();
        }
    }
    
    fn regenerate_editor_frames(&mut self) {
        if let Some(ref mut editor_data) = self.editor_data {
            let video_path = Path::new(&editor_data.file_path);
            let text_overlay = if editor_data.show_overlay_text {
                Some(editor_data.text.as_str())
            } else {
                None
            };
            
            // Regenerate start frame
            editor_data.start_frame_path = clipper::grab_frame(
                video_path, 
                &editor_data.start_time, 
                text_overlay, 
                None
            ).ok();
            
            // Regenerate end frame
            editor_data.end_frame_path = clipper::grab_frame(
                video_path, 
                &editor_data.end_time, 
                text_overlay, 
                None
            ).ok();
            
            // Reload start frame if available
            editor_data.start_frame = editor_data.start_frame_path.as_ref()
                .and_then(|path| {
                    let picker = Picker::from_fontsize((8, 12));
                    image::ImageReader::open(path)
                        .ok()
                        .and_then(|reader| reader.decode().ok())
                        .map(|img| picker.new_resize_protocol(img))
                });
                
            // Reload end frame if available - use separate picker instance
            editor_data.end_frame = editor_data.end_frame_path.as_ref()
                .and_then(|path| {
                    let picker = Picker::from_fontsize((8, 12));
                    image::ImageReader::open(path)
                        .ok()
                        .and_then(|reader| reader.decode().ok())
                        .map(|img| picker.new_resize_protocol(img))
                });
        }
    }
}

pub fn render_editor_tab(f: &mut Frame, area: ratatui::layout::Rect, app: &mut App) {
    // Create custom title with overlay status
    let overlay_status = if app.editor_data.as_ref().map_or(false, |data| data.show_overlay_text) {
        "ON"
    } else {
        "OFF"
    };
    let title = format!("🎬 Editor | Overlay: {} (press 'o' to toggle)", overlay_status);

    // Split the main content area into sections
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Min(10),    // Frame images section (expandable)
            Constraint::Length(3),  // Text content section
        ].as_ref())
        .split(area);

    // Create main block with tab title
    let main_block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::new().fg(app.colors.footer_border_color));

    f.render_widget(main_block, area);

    if let Some(editor_data) = &mut app.editor_data {
        // Frame images section
        let frame_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50), // Start frame
                Constraint::Percentage(50), // End frame
            ].as_ref())
            .split(main_chunks[0]);

        // Start frame
        let start_frame_block = Block::default()
            .title(format!("Start Frame: {}", editor_data.start_time))
            .borders(Borders::ALL)
            .border_style(Style::new().fg(app.colors.footer_border_color));

        if let Some(ref mut start_frame_protocol) = editor_data.start_frame {
            let inner_area = start_frame_block.inner(frame_chunks[0]);
            f.render_widget(start_frame_block, frame_chunks[0]);
            let image_widget = StatefulImage::new().resize(Resize::Scale(None));
            f.render_stateful_widget(image_widget, inner_area, start_frame_protocol);
            
            // Render overlay text if enabled
            if editor_data.show_overlay_text {
                let overlay_text = format!("START\n{}", editor_data.start_time);
                let overlay = Paragraph::new(overlay_text)
                    .style(Style::default().fg(app.colors.row_fg).bg(app.colors.buffer_bg))
                    .alignment(Alignment::Center);
                // Render overlay in the top portion of the inner area
                let overlay_height = 3;
                let overlay_area = ratatui::layout::Rect {
                    x: inner_area.x,
                    y: inner_area.y,
                    width: inner_area.width,
                    height: overlay_height.min(inner_area.height),
                };
                f.render_widget(overlay, overlay_area);
            }
        } else {
            let no_frame_text = Paragraph::new("No start frame generated")
                .block(start_frame_block)
                .style(Style::new().fg(app.colors.row_fg))
                .alignment(Alignment::Center);
            f.render_widget(no_frame_text, frame_chunks[0]);
        }

        // End frame
        let end_frame_block = Block::default()
            .title(format!("End Frame: {}", editor_data.end_time))
            .borders(Borders::ALL)
            .border_style(Style::new().fg(app.colors.footer_border_color));

        if let Some(ref mut end_frame_protocol) = editor_data.end_frame {
            let inner_area = end_frame_block.inner(frame_chunks[1]);
            f.render_widget(end_frame_block, frame_chunks[1]);
            let image_widget = StatefulImage::new().resize(Resize::Scale(None));
            f.render_stateful_widget(image_widget, inner_area, end_frame_protocol);
            
            // Render overlay text if enabled
            if editor_data.show_overlay_text {
                let overlay_text = format!("END\n{}", editor_data.end_time);
                let overlay = Paragraph::new(overlay_text)
                    .style(Style::default().fg(app.colors.row_fg).bg(app.colors.buffer_bg))
                    .alignment(Alignment::Center);
                // Render overlay in the top portion of the inner area
                let overlay_height = 3;
                let overlay_area = ratatui::layout::Rect {
                    x: inner_area.x,
                    y: inner_area.y,
                    width: inner_area.width,
                    height: overlay_height.min(inner_area.height),
                };
                f.render_widget(overlay, overlay_area);
            }
        } else {
            let no_frame_text = Paragraph::new("No end frame generated")
                .block(end_frame_block)
                .style(Style::new().fg(app.colors.row_fg))
                .alignment(Alignment::Center);
            f.render_widget(no_frame_text, frame_chunks[1]);
        }

        // Text content section
        let text_paragraph = Paragraph::new(editor_data.text.as_str())
            .block(
                Block::default()
                    .title(format!("Text Content - {}", editor_data.file_path))
                    .borders(Borders::ALL)
                    .border_style(Style::new().fg(app.colors.footer_border_color))
            )
            .style(Style::new().fg(app.colors.row_fg))
            .alignment(Alignment::Left)
            .wrap(ratatui::widgets::Wrap { trim: false });

        f.render_widget(text_paragraph, main_chunks[1]);
    } else {
        // Show empty state
        let empty_content = "No editor content. Select a search result and press 'c' to open the editor.";
        let empty_paragraph = Paragraph::new(empty_content)
            .style(Style::new().fg(app.colors.row_fg))
            .alignment(Alignment::Center);

        f.render_widget(empty_paragraph, main_chunks[0]);
    }
}