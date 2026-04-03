use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, Paragraph};
use ratatui::Frame;

const AXIS_WIDTH: u16 = 7; // match waveform/spectrogram axis width

pub fn render(
    f: &mut Frame,
    area: Rect,
    is_playing: bool,
    position_secs: f64,
    duration_secs: f64,
    volume: f32,
    position_fraction: f64,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.width <= AXIS_WIDTH {
        return;
    }

    // Left portion (under axis): play status + time
    // Right portion (under viz): progress bar + volume
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(AXIS_WIDTH), Constraint::Min(1)])
        .split(inner);

    // Play/pause + time in the axis-width area
    let play_icon = if is_playing { ">" } else { "||" };
    let pos_min = (position_secs / 60.0) as u64;
    let pos_sec = position_secs % 60.0;

    let time_text = Line::from(vec![Span::styled(
        format!("{play_icon}{pos_min}:{pos_sec:04.1}"),
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
    )]);
    f.render_widget(Paragraph::new(time_text), chunks[0]);

    // Right side: progress bar + duration + volume
    let right_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(10),   // progress bar
            Constraint::Length(18), // duration + volume
        ])
        .split(chunks[1]);

    // Progress bar (aligned with viz content)
    let ratio = position_fraction.clamp(0.0, 1.0);
    let gauge = Gauge::default()
        .ratio(ratio)
        .gauge_style(Style::default().fg(Color::Cyan).bg(Color::DarkGray));
    f.render_widget(gauge, right_chunks[0]);

    // Duration + volume
    let dur_min = (duration_secs / 60.0) as u64;
    let dur_sec = duration_secs % 60.0;
    let vol_pct = (volume * 100.0) as u32;

    let info_text = Line::from(vec![
        Span::styled(
            format!(" /{dur_min}:{dur_sec:04.1}"),
            Style::default().fg(Color::White),
        ),
        Span::styled(
            format!(" Vol:{vol_pct:3}%"),
            Style::default().fg(Color::Yellow),
        ),
    ]);
    f.render_widget(Paragraph::new(info_text), right_chunks[1]);
}
