use crate::tui::{App, create_tab_title_with_editor};
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table},
};

pub fn render_queue_tab(f: &mut Frame, area: Rect, app: &App) {
    let tab_title = create_tab_title_with_editor(
        app.current_tab,
        &app.colors,
        !app.search_results.is_empty(),
        app.editor_data.is_some(),
        app.file_view_data.is_some(),
    );

    let block = Block::default()
        .title(tab_title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.colors.footer_border_color));

    let mut rows = Vec::new();

    // Add currently processing item if exists
    if let Some(ref path) = app.currently_processing {
        let age_text = format_age(app.currently_processing_age);
        let status_cell = Cell::from(format!("PROCESSING ({})", age_text)).style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );
        let path_cell = Cell::from(path.clone());

        let row = Row::new(vec![status_cell, path_cell]);
        let row = if app.queue_selected_index == 0 {
            row.style(
                Style::default()
                    .fg(app.colors.selected_style_fg)
                    .add_modifier(Modifier::REVERSED),
            )
        } else {
            row.style(Style::default().fg(app.colors.row_fg))
        };
        rows.push(row);
    }

    // Add queue items
    for (i, path) in app.queue_items.iter().enumerate() {
        let position = i + 1;
        let status_cell =
            Cell::from(format!("#{}", position)).style(Style::default().fg(app.colors.row_fg));
        let path_cell = Cell::from(path.clone());

        let row = Row::new(vec![status_cell, path_cell]);

        // Calculate the actual index (offset by 1 if currently_processing exists)
        let item_index = if app.currently_processing.is_some() {
            i + 1
        } else {
            i
        };

        let row = if app.queue_selected_index == item_index {
            row.style(
                Style::default()
                    .fg(app.colors.selected_style_fg)
                    .add_modifier(Modifier::REVERSED),
            )
        } else {
            row.style(Style::default().fg(app.colors.row_fg))
        };
        rows.push(row);
    }

    // If no items at all, show a message
    if rows.is_empty() {
        let empty_row = Row::new(vec![Cell::from(""), Cell::from("No items in queue")])
            .style(Style::default().fg(app.colors.row_fg));
        rows.push(empty_row);
    }

    let widths = [Constraint::Length(25), Constraint::Min(50)];

    let table = Table::new(rows, widths)
        .block(block)
        .header(
            Row::new(vec![
                Cell::from("Status").style(
                    Style::default()
                        .fg(app.colors.header_fg)
                        .add_modifier(Modifier::BOLD),
                ),
                Cell::from("Path").style(
                    Style::default()
                        .fg(app.colors.header_fg)
                        .add_modifier(Modifier::BOLD),
                ),
            ])
            .style(
                Style::default()
                    .bg(app.colors.header_bg)
                    .fg(app.colors.header_fg),
            )
            .height(1),
        )
        .column_spacing(1);

    f.render_widget(table, area);
}

fn format_age(seconds: u64) -> String {
    if seconds < 60 {
        format!("{}s", seconds)
    } else if seconds < 3600 {
        let minutes = seconds / 60;
        let secs = seconds % 60;
        format!("{}m {}s", minutes, secs)
    } else {
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;
        format!("{}h {}m", hours, minutes)
    }
}
