use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::canvas::{Canvas, Line as CanvasLine};
use ratatui::widgets::{Block, Borders, Widget};
use ratatui::Frame;

use crate::analysis::waveform::compute_waveform;

const AXIS_WIDTH: u16 = 7; // match spectrogram axis width

pub fn render(
    f: &mut Frame,
    area: Rect,
    samples: &[f32],
    playback_fraction: f64,
    channel_idx: usize,
    total_channels: usize,
) {
    if area.width < 4 || area.height < 3 {
        return;
    }

    let title = if total_channels > 1 {
        format!(" Waveform (ch {}) ", channel_idx + 1)
    } else {
        " Waveform ".to_string()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(title);

    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.width <= AXIS_WIDTH || inner.height == 0 {
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(AXIS_WIDTH), Constraint::Min(1)])
        .split(inner);

    let axis_area = chunks[0];
    let wave_area = chunks[1];

    // Render amplitude axis
    let axis = AmpAxisWidget;
    f.render_widget(axis, axis_area);

    // Render waveform using Canvas (braille dots) — same width as spectrogram content
    let wave_width = wave_area.width as usize;
    let waveform_data = compute_waveform(samples, wave_width);

    let canvas = Canvas::default()
        .x_bounds([0.0, wave_width as f64])
        .y_bounds([-1.0, 1.0])
        .paint(move |ctx| {
            for (i, &(min, max)) in waveform_data.iter().enumerate() {
                let x = i as f64;
                ctx.draw(&CanvasLine {
                    x1: x,
                    y1: min as f64,
                    x2: x,
                    y2: max as f64,
                    color: Color::Cyan,
                });
            }

            ctx.draw(&CanvasLine {
                x1: 0.0,
                y1: 0.0,
                x2: wave_width as f64,
                y2: 0.0,
                color: Color::DarkGray,
            });

            let cursor_x = playback_fraction * wave_width as f64;
            ctx.draw(&CanvasLine {
                x1: cursor_x,
                y1: -1.0,
                x2: cursor_x,
                y2: 1.0,
                color: Color::Yellow,
            });
        });

    f.render_widget(canvas, wave_area);
}

struct AmpAxisWidget;

impl Widget for AmpAxisWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 || area.width == 0 {
            return;
        }

        let height = area.height as usize;

        for row in 0..height {
            let frac = 1.0 - (row as f32 / height.saturating_sub(1).max(1) as f32);
            let amp = frac * 2.0 - 1.0;

            let label = format!("{:+.1} ", amp);
            let label_len = label.len().min(area.width as usize);
            let x_start = area.x + area.width - label_len as u16;
            let y = area.y + row as u16;

            for (i, ch) in label.chars().take(label_len).enumerate() {
                buf[(x_start + i as u16, y)]
                    .set_char(ch)
                    .set_fg(Color::DarkGray);
            }
        }
    }
}
