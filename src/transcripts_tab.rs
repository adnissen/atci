use crate::files;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::widgets::{Block, Borders, Cell, List, ListItem, Row, Table};
use std::error::Error;

use crate::tui::{App, SortOrder, create_tab_title_with_editor};

impl App {
    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.video_data.len().saturating_sub(1) {
                    // Reached bottom, try to load next page
                    if self.current_page < self.total_pages.saturating_sub(1) {
                        if let Err(e) = self.load_next_page() {
                            eprintln!("Failed to load next page: {}", e);
                        }
                        return; // Selection will be set in load_next_page
                    }
                    // If no more pages, stop
                    return;
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    // Reached top, try to load previous page
                    if self.current_page > 0 {
                        if let Err(e) = self.load_previous_page() {
                            eprintln!("Failed to load previous page: {}", e);
                        }
                        return; // Selection will be set in load_previous_page
                    }
                    // If no previous page (page 1), stay at top - don't wrap
                    0
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn sort_by_column(&mut self, column_index: usize) {
        if column_index >= 6 {
            return; // Invalid column
        }

        // Cycle sort order: None -> Asc -> Desc -> None
        if let Some(current_column) = self.sort_column {
            if current_column == column_index {
                // Same column, cycle sort order
                match self.sort_order {
                    SortOrder::Ascending => self.sort_order = SortOrder::Descending,
                    SortOrder::Descending => {
                        // Reset to default sort (Generated At descending)
                        self.sort_column = Some(2); // Generated At column
                        self.sort_order = SortOrder::Descending;
                    }
                }
            } else {
                // Different column, start with ascending
                self.sort_column = Some(column_index);
                self.sort_order = SortOrder::Ascending;
            }
        } else {
            // No current sort, start with ascending
            self.sort_column = Some(column_index);
            self.sort_order = SortOrder::Ascending;
        }

        // Reload data with new sorting from database
        if let Err(e) = self.reload_with_current_sort() {
            eprintln!("Failed to reload data with new sort: {}", e);
        }
    }

    pub fn reload_with_current_sort(&mut self) -> Result<(), Box<dyn Error>> {
        let (sort_by, sort_order) = self.get_sort_params();
        let page_size = self.get_page_size();

        let cache_data = files::load_sorted_paginated_cache_data(
            self.get_filter_option().as_ref(), // filter
            0,                                 // page (reset to first page when sorting)
            page_size,                         // limit
            sort_by,                           // sort column
            sort_order,                        // sort order
        )?;

        self.video_data = cache_data.files;
        self.current_page = 0;
        self.total_pages = cache_data.pages.unwrap_or(1);
        self.total_records = cache_data.total_records.unwrap_or(0);

        // Reset selection to first item when sorting changes
        if !self.video_data.is_empty() {
            self.state.select(Some(0));
        }

        Ok(())
    }

    fn load_next_page(&mut self) -> Result<(), Box<dyn Error>> {
        if self.current_page >= self.total_pages.saturating_sub(1) {
            return Ok(()); // Already at last page
        }

        let next_page = self.current_page + 1;
        self.load_page(next_page)?;

        // Select first item of new page (for automatic page loading when scrolling)
        if !self.video_data.is_empty() {
            self.state.select(Some(0));
        }

        Ok(())
    }

    fn load_previous_page(&mut self) -> Result<(), Box<dyn Error>> {
        if self.current_page == 0 {
            return Ok(()); // Already at first page
        }

        let prev_page = self.current_page - 1;
        self.load_page(prev_page)?;

        // Select last item of new page (for automatic page loading when scrolling)
        if !self.video_data.is_empty() {
            self.state.select(Some(self.video_data.len() - 1));
        }

        Ok(())
    }

    fn load_next_page_preserve_cursor(&mut self) -> Result<(), Box<dyn Error>> {
        if self.current_page >= self.total_pages.saturating_sub(1) {
            return Ok(()); // Already at last page
        }

        // Remember current row position
        let current_row = self.state.selected().unwrap_or(0);

        let next_page = self.current_page + 1;
        self.load_page(next_page)?;

        // Try to keep same row position, or select last available row
        if !self.video_data.is_empty() {
            let target_row = if current_row < self.video_data.len() {
                current_row
            } else {
                self.video_data.len() - 1
            };
            self.state.select(Some(target_row));
        }

        Ok(())
    }

    fn load_previous_page_preserve_cursor(&mut self) -> Result<(), Box<dyn Error>> {
        if self.current_page == 0 {
            return Ok(()); // Already at first page
        }

        // Remember current row position
        let current_row = self.state.selected().unwrap_or(0);

        let prev_page = self.current_page - 1;
        self.load_page(prev_page)?;

        // Try to keep same row position, or select last available row
        if !self.video_data.is_empty() {
            let target_row = if current_row < self.video_data.len() {
                current_row
            } else {
                self.video_data.len() - 1
            };
            self.state.select(Some(target_row));
        }

        Ok(())
    }

    fn load_page(&mut self, page: u32) -> Result<(), Box<dyn Error>> {
        let (sort_by, sort_order) = self.get_sort_params();
        let page_size = self.get_page_size();

        let cache_data = files::load_sorted_paginated_cache_data(
            self.get_filter_option().as_ref(), // filter
            page,                              // specific page
            page_size,                         // limit
            sort_by,                           // sort column
            sort_order,                        // sort order
        )?;

        self.video_data = cache_data.files;
        self.current_page = page;
        self.total_pages = cache_data.pages.unwrap_or(1);
        self.total_records = cache_data.total_records.unwrap_or(0);

        Ok(())
    }

    // pub fn refresh_data(&mut self) -> Result<(), Box<dyn Error>> {
    //     // Get currently selected item for preservation
    //     let selected_path = self.state.selected()
    //         .and_then(|i| self.video_data.get(i))
    //         .map(|v| v.full_path.clone());

    //     // Update disk cache and reload data with current sorting
    //     files::get_and_save_video_info_from_disk()?;

    //     let (sort_by, sort_order) = self.get_sort_params();
    //     let page_size = self.get_page_size();

    //     let cache_data = files::load_sorted_paginated_cache_data(
    //         self.get_filter_option().as_ref(), // filter
    //         self.current_page, // current page
    //         page_size,   // limit
    //         sort_by,     // sort column
    //         sort_order,  // sort order
    //     )?;

    //     self.video_data = cache_data.files;
    //     self.total_pages = cache_data.pages.unwrap_or(1);

    //     // Restore selection if possible
    //     if let Some(path) = selected_path {
    //         if let Some(new_index) = self.video_data.iter().position(|v| v.full_path == path) {
    //             self.state.select(Some(new_index));
    //         } else {
    //             // If selected item no longer exists, select first item
    //             if !self.video_data.is_empty() {
    //                 self.state.select(Some(0));
    //             }
    //         }
    //     }

    //     self.last_refresh = std::time::Instant::now();
    //     Ok(())
    // }

    pub fn get_sort_params(&self) -> (&str, u8) {
        let sort_by = if let Some(column) = self.sort_column {
            match column {
                0 => "base_name",      // Filename
                1 => "created_at",     // Created At
                2 => "last_generated", // Generated At
                3 => "line_count",     // Lines
                4 => "length",         // Length
                5 => "source",         // Source
                _ => "last_generated", // Default fallback
            }
        } else {
            "last_generated" // Default
        };

        let sort_order = match self.sort_order {
            SortOrder::Ascending => 1,  // ASC
            SortOrder::Descending => 0, // DESC
        };

        (sort_by, sort_order)
    }

    pub fn next_page(&mut self) {
        if let Err(e) = self.load_next_page_preserve_cursor() {
            eprintln!("Failed to load next page: {}", e);
        }
    }

    pub fn prev_page(&mut self) {
        if let Err(e) = self.load_previous_page_preserve_cursor() {
            eprintln!("Failed to load previous page: {}", e);
        }
    }

    pub fn jump_to_top_of_page(&mut self) {
        if !self.video_data.is_empty() {
            self.state.select(Some(0));
        }
    }

    pub fn jump_to_bottom_of_page(&mut self) {
        if !self.video_data.is_empty() {
            self.state.select(Some(self.video_data.len() - 1));
        }
    }

    pub fn jump_to_first_page(&mut self) {
        if let Err(e) = self.load_page(0) {
            eprintln!("Failed to load first page: {}", e);
            return;
        }
        if !self.video_data.is_empty() {
            self.state.select(Some(0));
        }
    }

    pub fn jump_to_last_page(&mut self) {
        let last_page = self.total_pages.saturating_sub(1);
        if let Err(e) = self.load_page(last_page) {
            eprintln!("Failed to load last page: {}", e);
            return;
        }
        if !self.video_data.is_empty() {
            self.state.select(Some(self.video_data.len() - 1));
        }
    }
}

pub fn render_transcripts_tab(
    f: &mut Frame,
    area: ratatui::layout::Rect,
    app: &mut App,
    header_style: Style,
    selected_row_style: Style,
) {
    let title = create_tab_title_with_editor(
        app.current_tab,
        &app.colors,
        !app.search_results.is_empty(),
        app.editor_data.is_some(),
        app.file_view_data.is_some(),
    );
    let headers = [
        "Filename",
        "Created At",
        "Generated At",
        "Lines",
        "Length",
        "Source",
    ];
    let header_cells: Vec<Cell> = headers
        .iter()
        .enumerate()
        .map(|(i, &title)| {
            let mut content = format!("{} ({})", title, i + 1);

            // Add sort indicator if this column is being sorted
            if let Some(sort_col) = app.sort_column
                && sort_col == i
            {
                let indicator = match app.sort_order {
                    SortOrder::Ascending => " ↑",
                    SortOrder::Descending => " ↓",
                };
                content.push_str(indicator);
            }

            Cell::from(content)
        })
        .collect();

    let header = Row::new(header_cells).style(header_style).height(1);

    let rows = if app.video_data.is_empty() {
        // Show empty state
        vec![
            Row::new(vec![
                Cell::from("No video files found"),
                Cell::from(""),
                Cell::from(""),
                Cell::from(""),
                Cell::from(""),
                Cell::from(""),
            ])
            .style(
                Style::new()
                    .fg(app.colors.row_fg)
                    .bg(app.colors.normal_row_color),
            ),
        ]
    } else {
        app.video_data
            .iter()
            .enumerate()
            .map(|(i, video)| {
                let color = match i % 2 {
                    0 => app.colors.normal_row_color,
                    _ => app.colors.alt_row_color,
                };

                // Format the data to match our table columns and create Row directly
                Row::new(vec![
                    Cell::from(video.base_name.as_str()),
                    Cell::from(
                        video
                            .created_at
                            .split(' ')
                            .next()
                            .unwrap_or(&video.created_at),
                    ),
                    Cell::from(
                        video
                            .last_generated
                            .as_ref()
                            .map(|dt| dt.split(' ').next().unwrap_or(dt))
                            .unwrap_or("-"),
                    ),
                    Cell::from(video.line_count.to_string()),
                    Cell::from(video.length.as_deref().unwrap_or("-")),
                    Cell::from(video.source.as_deref().unwrap_or("-")),
                ])
                .style(Style::new().fg(app.colors.row_fg).bg(color))
                .height(1)
            })
            .collect()
    };

    let t = Table::new(
        rows,
        [
            Constraint::Percentage(25), // Filename
            Constraint::Percentage(15), // Created At
            Constraint::Percentage(15), // Generated At
            Constraint::Percentage(10), // Lines
            Constraint::Percentage(10), // Length
            Constraint::Percentage(25), // Source
        ],
    )
    .header(header)
    .bg(app.colors.buffer_bg)
    .row_highlight_style(selected_row_style)
    .block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::new().fg(app.colors.footer_border_color)),
    );
    f.render_stateful_widget(t, area, &mut app.state);

    // Render regenerate popup if shown
    if app.show_regenerate_popup {
        render_regenerate_popup(f, app);
    }
}

fn render_regenerate_popup(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 50, f.area());

    // Create the popup block
    let block = Block::default()
        .title("Choose Processing Method")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    // Create list items
    let items: Vec<ListItem> = app
        .regenerate_popup_options
        .iter()
        .enumerate()
        .map(|(i, option)| {
            let style = if i == app.regenerate_popup_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(option.as_str()).style(style)
        })
        .collect();

    let list = List::new(items).block(block);

    // Clear the area first to create the popup effect
    f.render_widget(ratatui::widgets::Clear, area);
    f.render_widget(list, area);
}

/// Helper function to create a centered rect using up certain percentage of the available rect `r`
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(r);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}
