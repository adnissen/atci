use clipboard_rs::{Clipboard, ClipboardContext};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui_image::{
    FilterType::Lanczos3, Resize, StatefulImage, picker::Picker, protocol::StatefulProtocol,
};
use std::path::Path;
use std::process::Command;

use crate::clipper;
use crate::tui::{App, TabState, create_tab_title_with_editor};

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
    pub font_size: u32,
    pub editor_selection: EditorSelection,
    pub text_editing_mode: bool,
    pub text_input: String,
    pub format: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FrameSelection {
    Start,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EditorSelection {
    StartFrame,
    EndFrame,
    FontSize,
    TextContent,
    Format,
}

impl App {
    pub fn open_editor(
        &mut self,
        start_time: String,
        end_time: String,
        text: String,
        file_path: String,
    ) {
        // Calculate default font size based on video dimensions
        let video_path = Path::new(&file_path);
        let default_font_size = clipper::calculate_font_size_for_video_path(video_path, text.len());

        // Generate frame images with text overlay by default
        let start_frame_path = clipper::grab_frame(
            video_path,
            &start_time,
            Some(&text),
            Some(default_font_size),
            Some(360),
        )
        .ok();
        let end_frame_path = clipper::grab_frame(
            video_path,
            &end_time,
            Some(&text),
            Some(default_font_size),
            Some(360),
        )
        .ok();

        // Load start frame if available
        let picker = Picker::from_query_stdio().ok().unwrap();

        let start_frame = start_frame_path.as_ref().and_then(|path| {
            image::ImageReader::open(path)
                .ok()
                .and_then(|reader| reader.decode().ok())
                .map(|img| picker.new_resize_protocol(img))
        });

        // Load end frame if available - use separate picker instance
        let end_frame = end_frame_path.as_ref().and_then(|path| {
            image::ImageReader::open(path)
                .ok()
                .and_then(|reader| reader.decode().ok())
                .map(|img| picker.new_resize_protocol(img))
        });

        self.editor_data = Some(EditorData {
            start_time,
            end_time,
            text: text.clone(),
            file_path,
            start_frame_path,
            start_frame,
            end_frame_path,
            end_frame,
            show_overlay_text: true, // Default to showing overlay text
            selected_frame: FrameSelection::Start, // Default to start frame selected
            font_size: default_font_size,
            editor_selection: EditorSelection::StartFrame, // Default selection
            text_editing_mode: false,
            text_input: text,          // Initialize with current text
            format: "mp4".to_string(), // Default format
        });
        self.current_tab = TabState::Editor;
    }

    pub fn switch_to_editor(&mut self) {
        if self.editor_data.is_some() {
            self.current_tab = TabState::Editor;
        }
    }

    // pub fn toggle_editor_overlay(&mut self) {
    //     if let Some(ref mut editor_data) = self.editor_data {
    //         editor_data.show_overlay_text = !editor_data.show_overlay_text;
    //         // Regenerate frames with new overlay setting
    //         self.regenerate_editor_frames();
    //     }
    // }

    // pub fn select_frame(&mut self, frame: FrameSelection) {
    //     if let Some(ref mut editor_data) = self.editor_data {
    //         editor_data.selected_frame = frame;
    //     }
    // }

    pub fn navigate_editor_selection_left_or_right(&mut self, right: bool) {
        if let Some(ref mut editor_data) = self.editor_data {
            // Don't navigate if in text editing mode
            if editor_data.text_editing_mode {
                return;
            }

            editor_data.editor_selection = match (editor_data.editor_selection, right) {
                // start frame
                (EditorSelection::StartFrame, true) => EditorSelection::EndFrame, // right
                (EditorSelection::StartFrame, false) => EditorSelection::EndFrame, // left

                // end frame
                (EditorSelection::EndFrame, true) => EditorSelection::StartFrame, // right
                (EditorSelection::EndFrame, false) => EditorSelection::StartFrame, // left

                // font size
                (EditorSelection::FontSize, true) => EditorSelection::TextContent, // right
                (EditorSelection::FontSize, false) => EditorSelection::TextContent, // left

                // format
                (EditorSelection::Format, true) => EditorSelection::TextContent, // right
                (EditorSelection::Format, false) => EditorSelection::TextContent, // left

                // text content
                (EditorSelection::TextContent, true) => EditorSelection::FontSize, // right
                (EditorSelection::TextContent, false) => EditorSelection::FontSize, // left
            };
        }
    }

    pub fn navigate_editor_selection_up_or_down(&mut self, down: bool) {
        if let Some(ref mut editor_data) = self.editor_data {
            // Don't navigate if in text editing mode
            if editor_data.text_editing_mode {
                return;
            }

            editor_data.editor_selection = match (editor_data.editor_selection, down) {
                // start frame
                (EditorSelection::StartFrame, true) => EditorSelection::TextContent, // down
                (EditorSelection::StartFrame, false) => EditorSelection::TextContent, // up

                // end frame
                (EditorSelection::EndFrame, true) => EditorSelection::FontSize, // down
                (EditorSelection::EndFrame, false) => EditorSelection::Format,  // up

                // font size
                (EditorSelection::FontSize, true) => EditorSelection::Format, // down
                (EditorSelection::FontSize, false) => EditorSelection::EndFrame, // up

                // format
                (EditorSelection::Format, true) => EditorSelection::EndFrame, // down
                (EditorSelection::Format, false) => EditorSelection::FontSize, // up

                // text content
                (EditorSelection::TextContent, true) => EditorSelection::StartFrame, // down
                (EditorSelection::TextContent, false) => EditorSelection::StartFrame, // up
            }
        }
    }

    pub fn activate_selected_element(&mut self) {
        if let Some(ref mut editor_data) = self.editor_data {
            match editor_data.editor_selection {
                EditorSelection::StartFrame => {
                    // Set the internal frame selection for time adjustment
                    editor_data.selected_frame = FrameSelection::Start;
                }
                EditorSelection::EndFrame => {
                    // Set the internal frame selection for time adjustment
                    editor_data.selected_frame = FrameSelection::End;
                }
                EditorSelection::TextContent => {
                    // Enter text editing mode
                    editor_data.text_editing_mode = true;
                    editor_data.text_input = editor_data.text.clone();
                }
                EditorSelection::Format => {
                    // Cycle through formats: mp4 -> gif -> mp3 -> mp4
                    editor_data.format = match editor_data.format.as_str() {
                        "mp4" => "gif".to_string(),
                        "gif" => "mp3".to_string(),
                        "mp3" => "mp4".to_string(),
                        _ => "mp4".to_string(), // Default fallback
                    };
                }
                EditorSelection::FontSize => {
                    // Toggle text overlay state
                    editor_data.show_overlay_text = !editor_data.show_overlay_text;
                    // Regenerate frames with new overlay setting
                    self.regenerate_editor_frames();
                }
            }
        }
    }

    pub fn exit_text_editing(&mut self) {
        if let Some(ref mut editor_data) = self.editor_data
            && editor_data.text_editing_mode
        {
            // Update the text and regenerate frames if overlay is enabled
            editor_data.text = editor_data.text_input.clone();
            editor_data.text_editing_mode = false;

            // Regenerate frames if overlay text is enabled
            if editor_data.show_overlay_text {
                self.regenerate_editor_frames();
            }
        }
    }

    pub fn add_char_to_text(&mut self, c: char) {
        if let Some(ref mut editor_data) = self.editor_data
            && editor_data.text_editing_mode
        {
            editor_data.text_input.push(c);
        }
    }

    pub fn remove_char_from_text(&mut self) {
        if let Some(ref mut editor_data) = self.editor_data
            && editor_data.text_editing_mode
        {
            editor_data.text_input.pop();
        }
    }

    pub fn adjust_font_size(&mut self, increase: bool) {
        if let Some(ref mut editor_data) = self.editor_data {
            let adjustment = if increase { 2 } else { -2 };
            let new_size = (editor_data.font_size as i32 + adjustment).max(8) as u32; // Minimum font size of 8

            if new_size != editor_data.font_size {
                editor_data.font_size = new_size;
                // Regenerate frames with new font size
                self.regenerate_editor_frames();
            }
        }
    }

    pub fn adjust_selected_frame_time(&mut self, forward: bool) {
        self.adjust_selected_frame_time_by_amount(forward, 0.1);
    }

    pub fn adjust_selected_frame_time_by_second(&mut self, forward: bool) {
        self.adjust_selected_frame_time_by_amount(forward, 1.0);
    }

    fn adjust_selected_frame_time_by_amount(&mut self, forward: bool, amount: f64) {
        if let Some(ref editor_data) = self.editor_data {
            // Only allow time adjustment if a frame is selected in the unified selection system
            let selected_frame = match editor_data.editor_selection {
                EditorSelection::StartFrame => FrameSelection::Start,
                EditorSelection::EndFrame => FrameSelection::End,
                _ => return, // Don't adjust time if no frame is selected
            };

            let adjustment = if forward { amount } else { -amount };

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
                            // Update time and regenerate frame immediately
                            if let Some(ref mut editor_data) = self.editor_data {
                                editor_data.start_time = new_time;
                            }
                            self.regenerate_single_frame(FrameSelection::Start);
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
                            // Update time and regenerate frame immediately
                            if let Some(ref mut editor_data) = self.editor_data {
                                editor_data.end_time = new_time;
                            }
                            self.regenerate_single_frame(FrameSelection::End);
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
                Some(editor_data.font_size),
                Some(360),
            )
            .ok();

            // Regenerate end frame
            editor_data.end_frame_path = clipper::grab_frame(
                video_path,
                &editor_data.end_time,
                text_overlay,
                Some(editor_data.font_size),
                Some(360),
            )
            .ok();

            let picker = Picker::from_query_stdio().ok().unwrap();
            // Reload start frame if available
            editor_data.start_frame = editor_data.start_frame_path.as_ref().and_then(|path| {
                image::ImageReader::open(path)
                    .ok()
                    .and_then(|reader| reader.decode().ok())
                    .map(|img| picker.new_resize_protocol(img))
            });

            // Reload end frame if available - use separate picker instance
            editor_data.end_frame = editor_data.end_frame_path.as_ref().and_then(|path| {
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
                        Some(editor_data.font_size),
                        Some(360),
                    )
                    .ok();

                    // Reload start frame
                    editor_data.start_frame =
                        editor_data.start_frame_path.as_ref().and_then(|path| {
                            let picker = Picker::from_query_stdio().ok().unwrap();
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
                        Some(editor_data.font_size),
                        Some(360),
                    )
                    .ok();

                    // Reload end frame
                    editor_data.end_frame = editor_data.end_frame_path.as_ref().and_then(|path| {
                        let picker = Picker::from_query_stdio().ok().unwrap();
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
            _ => Err("Invalid time format".into()),
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

    pub fn copy_clip(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref _editor_data) = self.editor_data {
            let clip_path = self.generate_clip()?;
            self.copy_file_to_clipboard(&clip_path)?;
        }
        Ok(())
    }

    pub fn open_clip(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref _editor_data) = self.editor_data {
            let clip_path = self.generate_clip()?;
            self.open_file(&clip_path)?;
        }
        Ok(())
    }

    fn generate_clip(&self) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
        if let Some(ref editor_data) = self.editor_data {
            let video_path = Path::new(&editor_data.file_path);
            let display_text = editor_data.show_overlay_text;
            let text = if display_text {
                Some(editor_data.text.as_str())
            } else {
                None
            };

            // Generate the clip using the current parameters
            clipper::clip(
                video_path,
                &editor_data.start_time,
                &editor_data.end_time,
                text,
                display_text,
                &editor_data.format, // Use selected format
                Some(editor_data.font_size),
            )
        } else {
            Err("No editor data available".into())
        }
    }

    fn copy_file_to_clipboard(&self, file_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let ctx = ClipboardContext::new()
            .map_err(|e| format!("Failed to create clipboard context: {}", e))?;

        // Copy the actual file to clipboard using file list support
        let file_list = vec![file_path.to_string_lossy().to_string()];
        ctx.set_files(file_list)
            .map_err(|e| format!("Failed to set files in clipboard: {}", e))?;

        Ok(())
    }

    fn open_file(&self, file_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        #[cfg(target_os = "macos")]
        {
            Command::new("open").arg(file_path).spawn()?;
        }

        #[cfg(target_os = "linux")]
        {
            Command::new("xdg-open").arg(file_path).spawn()?;
        }

        #[cfg(target_os = "windows")]
        {
            Command::new("cmd")
                .args(["/C", "start", "", &file_path.display().to_string()])
                .spawn()?;
        }

        Ok(())
    }
}

pub fn render_editor_tab(f: &mut Frame, area: ratatui::layout::Rect, app: &mut App) {
    // Use the same tab title system as other tabs
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
                Constraint::Min(7),    // Frame images section (reduced by 2 lines)
                Constraint::Length(6), // Text content section (increased by 2 lines)
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

    if let Some(editor_data) = &mut app.editor_data {
        // Frame images section
        let frame_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Percentage(50), // Start frame
                    Constraint::Percentage(50), // End frame
                ]
                .as_ref(),
            )
            .split(main_chunks[0]);

        // Start frame
        let start_selected = editor_data.editor_selection == EditorSelection::StartFrame;
        let start_frame_title = if start_selected {
            format!("Start Frame: {} [SELECTED]", editor_data.start_time)
        } else {
            format!("Start Frame: {}", editor_data.start_time)
        };

        let start_frame_block = Block::default()
            .title(start_frame_title)
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
        let end_selected = editor_data.editor_selection == EditorSelection::EndFrame;
        let end_frame_title = if end_selected {
            format!("End Frame: {} [SELECTED]", editor_data.end_time)
        } else {
            format!("End Frame: {}", editor_data.end_time)
        };

        let end_frame_block = Block::default()
            .title(end_frame_title)
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

        // Split text section horizontally: 90% for text content, 10% for font size display
        let text_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Percentage(90), // Text content
                    Constraint::Percentage(10), // Font size display
                ]
                .as_ref(),
            )
            .split(main_chunks[1]);

        // Text content section
        let text_content = if editor_data.text_editing_mode {
            &editor_data.text_input
        } else {
            &editor_data.text
        };

        let text_selected = editor_data.editor_selection == EditorSelection::TextContent;
        let text_title = if editor_data.text_editing_mode {
            format!("Text Content - {} [EDITING]", editor_data.file_path)
        } else if text_selected {
            format!("Text Content - {} [SELECTED]", editor_data.file_path)
        } else {
            format!("Text Content - {}", editor_data.file_path)
        };

        let text_paragraph = Paragraph::new(text_content.as_str())
            .block(
                Block::default()
                    .title(text_title)
                    .borders(Borders::ALL)
                    .border_style(Style::new().fg(
                        if text_selected || editor_data.text_editing_mode {
                            ratatui::style::Color::Yellow
                        } else {
                            app.colors.footer_border_color
                        },
                    )),
            )
            .style(Style::new().fg(app.colors.row_fg))
            .alignment(Alignment::Left)
            .wrap(ratatui::widgets::Wrap { trim: false });

        f.render_widget(text_paragraph, text_chunks[0]);

        // Split font size section vertically
        let font_size_column_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Percentage(50), // Font size display
                    Constraint::Percentage(50), // Format display
                ]
                .as_ref(),
            )
            .split(text_chunks[1]);
        // Font size display section
        let font_size_selected = editor_data.editor_selection == EditorSelection::FontSize;
        let overlay_state = if editor_data.show_overlay_text {
            "[on]"
        } else {
            "[off]"
        };
        let font_size_title = if font_size_selected {
            format!("Font {} [SELECTED]", overlay_state)
        } else {
            format!("Font {}", overlay_state)
        };

        let font_size_paragraph = Paragraph::new(format!("{} (+/-)", editor_data.font_size))
            .block(
                Block::default()
                    .title(font_size_title)
                    .borders(Borders::ALL)
                    .border_style(Style::new().fg(if font_size_selected {
                        ratatui::style::Color::Yellow
                    } else {
                        app.colors.footer_border_color
                    })),
            )
            .style(Style::new().fg(app.colors.row_fg))
            .alignment(Alignment::Center);

        f.render_widget(font_size_paragraph, font_size_column_chunks[0]);

        let format_selected = editor_data.editor_selection == EditorSelection::Format;
        let format_title = if format_selected {
            "Format [SELECTED]"
        } else {
            "Format"
        };

        let format_paragraph = Paragraph::new(editor_data.format.as_str())
            .block(
                Block::default()
                    .title(format_title)
                    .borders(Borders::ALL)
                    .border_style(Style::new().fg(if format_selected {
                        ratatui::style::Color::Yellow
                    } else {
                        app.colors.footer_border_color
                    })),
            )
            .style(Style::new().fg(app.colors.row_fg))
            .alignment(Alignment::Center);

        f.render_widget(format_paragraph, font_size_column_chunks[1]);
    } else {
        // Show empty state
        let empty_content =
            "No editor content. Select a search result and press 'c' to open the editor.";
        let empty_paragraph = Paragraph::new(empty_content)
            .style(Style::new().fg(app.colors.row_fg))
            .alignment(Alignment::Center);

        f.render_widget(empty_paragraph, main_chunks[0]);
    }
}
