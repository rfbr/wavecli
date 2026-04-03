use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

pub fn render(
    f: &mut Frame,
    area: Rect,
    filename: &str,
    format: &str,
    codec: &str,
    sample_rate: u32,
    channels: usize,
    duration_secs: f64,
) {
    let dur_min = (duration_secs / 60.0) as u64;
    let dur_sec = duration_secs % 60.0;

    let channel_str = match channels {
        1 => "Mono",
        2 => "Stereo",
        n => &format!("{n}ch"),
    };

    let info_line = Line::from(vec![
        Span::styled(
            format!(" {filename} "),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" | ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{format}/{codec}"),
            Style::default().fg(Color::Cyan),
        ),
        Span::styled(" | ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{sample_rate}Hz"),
            Style::default().fg(Color::Yellow),
        ),
        Span::styled(" | ", Style::default().fg(Color::DarkGray)),
        Span::styled(channel_str.to_string(), Style::default().fg(Color::Green)),
        Span::styled(" | ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{dur_min}:{dur_sec:05.2}"),
            Style::default().fg(Color::Magenta),
        ),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" Audio Info ");

    let paragraph = Paragraph::new(info_line).block(block);
    f.render_widget(paragraph, area);
}
