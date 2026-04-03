use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::canvas::{Canvas, Line as CanvasLine};
use ratatui::widgets::{Block, Borders};
use ratatui::Frame;

use crate::analysis::waveform::compute_waveform;

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

    let inner_width = (area.width - 2) as usize; // account for borders
    let waveform_data = compute_waveform(samples, inner_width);

    let title = if total_channels > 1 {
        format!(" Waveform (ch {}) ", channel_idx + 1)
    } else {
        " Waveform ".to_string()
    };

    let canvas = Canvas::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(title),
        )
        .x_bounds([0.0, inner_width as f64])
        .y_bounds([-1.0, 1.0])
        .paint(move |ctx| {
            // Draw waveform envelope
            for (i, &(min, max)) in waveform_data.iter().enumerate() {
                let x = i as f64;
                // Draw line from min to max at each column
                ctx.draw(&CanvasLine {
                    x1: x,
                    y1: min as f64,
                    x2: x,
                    y2: max as f64,
                    color: Color::Cyan,
                });
            }

            // Draw center line
            ctx.draw(&CanvasLine {
                x1: 0.0,
                y1: 0.0,
                x2: inner_width as f64,
                y2: 0.0,
                color: Color::DarkGray,
            });

            // Draw playback cursor
            let cursor_x = playback_fraction * inner_width as f64;
            ctx.draw(&CanvasLine {
                x1: cursor_x,
                y1: -1.0,
                x2: cursor_x,
                y2: 1.0,
                color: Color::Yellow,
            });
        });

    f.render_widget(canvas, area);
}
