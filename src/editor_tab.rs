use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::style::Style;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use std::path::Path;
use std::time::{Duration, Instant};
use ratatui::Frame;
use ratatui_image::{picker::Picker, StatefulImage, protocol::StatefulProtocol, Resize, FilterType::Lanczos3};

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
    pub selected_frame: FrameSelection,
    pub frame_regeneration_timer: Option<Instant>,
    pub pending_frame_regeneration: Option<FrameSelection>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FrameSelection {
    Start,
    End,
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
            selected_frame: FrameSelection::Start, // Default to start frame selected
            frame_regeneration_timer: None,
            pending_frame_regeneration: None,
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
    
    pub fn check_frame_regeneration_timer(&mut self) {
        // Check if we need to regenerate a frame
        let should_regenerate = if let Some(ref editor_data) = self.editor_data {
            if let (Some(timer), Some(pending_frame)) = (editor_data.frame_regeneration_timer, editor_data.pending_frame_regeneration) {
                if timer.elapsed() >= Duration::from_millis(250) {
                    Some(pending_frame)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        
        // If we need to regenerate, do it and clear the timer
        if let Some(pending_frame) = should_regenerate {
            self.regenerate_single_frame(pending_frame);
            if let Some(ref mut editor_data) = self.editor_data {
                editor_data.frame_regeneration_timer = None;
                editor_data.pending_frame_regeneration = None;
            }
        }
    }
    
    pub fn select_frame(&mut self, frame: FrameSelection) {
        if let Some(ref mut editor_data) = self.editor_data {
            editor_data.selected_frame = frame;
        }
    }
    
    pub fn adjust_selected_frame_time(&mut self, forward: bool) {
        if let Some(ref editor_data) = self.editor_data {
            let adjustment = if forward { 0.1 } else { -0.1 };
            let selected_frame = editor_data.selected_frame;
            
            // Parse both start and end times for validation
            let start_seconds = Self::parse_time_to_seconds(&editor_data.start_time).unwrap_or(0.0);
            let end_seconds = Self::parse_time_to_seconds(&editor_data.end_time).unwrap_or(0.0);
            
            match selected_frame {
                FrameSelection::Start => {
                    let current_time = editor_data.start_time.clone();
                    if let Ok(mut time_seconds) = Self::parse_time_to_seconds(&current_time) {
                        time_seconds += adjustment;
                        // Validate: start time must be >= 0 and at least 0.5 seconds before end time
                        if time_seconds >= 0.0 && time_seconds <= (end_seconds - 0.5) {
                            let new_time = Self::seconds_to_time_string(time_seconds);
                            // Update time instantly and set up debounce timer
                            if let Some(ref mut editor_data) = self.editor_data {
                                editor_data.start_time = new_time;
                                editor_data.frame_regeneration_timer = Some(Instant::now());
                                editor_data.pending_frame_regeneration = Some(FrameSelection::Start);
                            }
                        }
                    }
                }
                FrameSelection::End => {
                    let current_time = editor_data.end_time.clone();
                    if let Ok(mut time_seconds) = Self::parse_time_to_seconds(&current_time) {
                        time_seconds += adjustment;
                        // Validate: end time must be >= 0 and at least 0.5 seconds after start time
                        if time_seconds >= 0.0 && time_seconds >= (start_seconds + 0.5) {
                            let new_time = Self::seconds_to_time_string(time_seconds);
                            // Update time instantly and set up debounce timer
                            if let Some(ref mut editor_data) = self.editor_data {
                                editor_data.end_time = new_time;
                                editor_data.frame_regeneration_timer = Some(Instant::now());
                                editor_data.pending_frame_regeneration = Some(FrameSelection::End);
                            }
                        }
                    }
                }
            }
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
    
    fn regenerate_single_frame(&mut self, frame: FrameSelection) {
        if let Some(ref mut editor_data) = self.editor_data {
            let video_path = Path::new(&editor_data.file_path);
            let text_overlay = if editor_data.show_overlay_text {
                Some(editor_data.text.as_str())
            } else {
                None
            };
            
            match frame {
                FrameSelection::Start => {
                    editor_data.start_frame_path = clipper::grab_frame(
                        video_path, 
                        &editor_data.start_time, 
                        text_overlay, 
                        None
                    ).ok();
                    
                    // Reload start frame
                    editor_data.start_frame = editor_data.start_frame_path.as_ref()
                        .and_then(|path| {
                            let picker = Picker::from_fontsize((8, 12));
                            image::ImageReader::open(path)
                                .ok()
                                .and_then(|reader| reader.decode().ok())
                                .map(|img| picker.new_resize_protocol(img))
                        });
                }
                FrameSelection::End => {
                    editor_data.end_frame_path = clipper::grab_frame(
                        video_path, 
                        &editor_data.end_time, 
                        text_overlay, 
                        None
                    ).ok();
                    
                    // Reload end frame
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
    }
    
    fn parse_time_to_seconds(time_str: &str) -> Result<f64, Box<dyn std::error::Error>> {
        // Parse time format like "00:01:23.456" or "1:23.456" or "23.456"
        let parts: Vec<&str> = time_str.split(':').collect();
        
        match parts.len() {
            1 => {
                // Just seconds: "23.456"
                Ok(parts[0].parse::<f64>()?)
            }
            2 => {
                // Minutes and seconds: "1:23.456"
                let minutes = parts[0].parse::<f64>()?;
                let seconds = parts[1].parse::<f64>()?;
                Ok(minutes * 60.0 + seconds)
            }
            3 => {
                // Hours, minutes, and seconds: "00:01:23.456"
                let hours = parts[0].parse::<f64>()?;
                let minutes = parts[1].parse::<f64>()?;
                let seconds = parts[2].parse::<f64>()?;
                Ok(hours * 3600.0 + minutes * 60.0 + seconds)
            }
            _ => Err("Invalid time format".into())
        }
    }
    
    fn seconds_to_time_string(seconds: f64) -> String {
        let hours = (seconds / 3600.0) as u32;
        let minutes = ((seconds % 3600.0) / 60.0) as u32;
        let secs = seconds % 60.0;
        
        if hours > 0 {
            format!("{:02}:{:02}:{:06.3}", hours, minutes, secs)
        } else if minutes > 0 {
            format!("{:02}:{:06.3}", minutes, secs)
        } else {
            format!("{:06.3}", secs)
        }
    }
}

pub fn render_editor_tab(f: &mut Frame, area: ratatui::layout::Rect, app: &mut App) {
    // Use the same tab title system as other tabs
    let title = create_tab_title_with_editor(app.current_tab, &app.colors, !app.search_results.is_empty(), app.editor_data.is_some());

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
        let start_selected = editor_data.selected_frame == FrameSelection::Start;
        let start_frame_block = Block::default()
            .title(format!("Start Frame: {} {}", editor_data.start_time, if start_selected { "[SELECTED]" } else { "" }))
            .borders(Borders::ALL)
            .border_style(Style::new().fg(if start_selected { 
                ratatui::style::Color::Yellow 
            } else { 
                app.colors.footer_border_color 
            }));

        if let Some(ref mut start_frame_protocol) = editor_data.start_frame {
            let inner_area = start_frame_block.inner(frame_chunks[0]);
            f.render_widget(start_frame_block, frame_chunks[0]);
            let image_widget = StatefulImage::new().resize(Resize::Scale(Some(Lanczos3)));
            f.render_stateful_widget(image_widget, inner_area, start_frame_protocol);
        } else {
            let no_frame_text = Paragraph::new("No start frame generated")
                .block(start_frame_block)
                .style(Style::new().fg(app.colors.row_fg))
                .alignment(Alignment::Center);
            f.render_widget(no_frame_text, frame_chunks[0]);
        }

        // End frame
        let end_selected = editor_data.selected_frame == FrameSelection::End;
        let end_frame_block = Block::default()
            .title(format!("End Frame: {} {}", editor_data.end_time, if end_selected { "[SELECTED]" } else { "" }))
            .borders(Borders::ALL)
            .border_style(Style::new().fg(if end_selected { 
                ratatui::style::Color::Yellow 
            } else { 
                app.colors.footer_border_color 
            }));

        if let Some(ref mut end_frame_protocol) = editor_data.end_frame {
            let inner_area = end_frame_block.inner(frame_chunks[1]);
            f.render_widget(end_frame_block, frame_chunks[1]);
            let image_widget = StatefulImage::new().resize(Resize::Scale(Some(Lanczos3)));
            f.render_stateful_widget(image_widget, inner_area, end_frame_protocol);
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