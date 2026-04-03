use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

pub fn render(
    f: &mut Frame,
    area: Rect,
    files: &[String],
    selected: usize,
    current_file: &str,
    focused: bool,
    filter: &str,
) {
    let border_color = if focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    // Split: filter input (1 line + border) at top, file list below
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(area);

    // Filter input
    let filter_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if focused {
            Color::Yellow
        } else {
            Color::DarkGray
        }))
        .title(" Filter ");

    let filter_text = if filter.is_empty() && focused {
        Line::from(Span::styled(
            "type to filter...",
            Style::default().fg(Color::DarkGray),
        ))
    } else {
        Line::from(vec![
            Span::styled(filter, Style::default().fg(Color::Yellow)),
            if focused {
                Span::styled("_", Style::default().fg(Color::Yellow))
            } else {
                Span::raw("")
            },
        ])
    };

    f.render_widget(
        Paragraph::new(filter_text).block(filter_block),
        chunks[0],
    );

    // File list
    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(format!(" Files ({}) ", files.len()));

    let items: Vec<ListItem> = files
        .iter()
        .map(|name| {
            let is_current = name == current_file;
            let style = if is_current {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let prefix = if is_current { "> " } else { "  " };
            ListItem::new(Line::from(Span::styled(
                format!("{prefix}{name}"),
                style,
            )))
        })
        .collect();

    let list = List::new(items)
        .block(list_block)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    let mut state = ListState::default();
    if !files.is_empty() {
        state.select(Some(selected));
    }

    f.render_stateful_widget(list, chunks[1], &mut state);
}
