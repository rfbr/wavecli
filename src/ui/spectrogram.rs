use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Widget};
use ratatui::Frame;

use crate::analysis::spectrogram::SpectrogramData;

const AXIS_WIDTH: u16 = 7; // e.g. "22.0k " or " 500  "

pub fn render(
    f: &mut Frame,
    area: Rect,
    spectrogram: &SpectrogramData,
    playback_fraction: f64,
) {
    if area.width < 4 || area.height < 3 {
        return;
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" Spectrogram ");

    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.width <= AXIS_WIDTH || inner.height == 0 {
        return;
    }

    // Split inner into frequency axis | spectrogram content
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(AXIS_WIDTH), Constraint::Min(1)])
        .split(inner);

    let axis_area = chunks[0];
    let spec_area = chunks[1];

    // Render frequency axis
    let axis = FreqAxisWidget {
        max_freq: spectrogram.max_freq,
    };
    f.render_widget(axis, axis_area);

    // Render spectrogram
    let widget = SpectrogramWidget {
        data: spectrogram,
        playback_fraction,
    };
    f.render_widget(widget, spec_area);
}

struct FreqAxisWidget {
    max_freq: f32,
}

impl Widget for FreqAxisWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 || area.width == 0 {
            return;
        }

        let height = area.height as usize;

        for row in 0..height {
            // Map row to frequency (top = max_freq, bottom = 0)
            let frac = 1.0 - (row as f32 / height as f32);
            let freq = frac * self.max_freq;

            let label = format_freq(freq);

            // Right-align the label in the axis area
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

fn format_freq(freq: f32) -> String {
    if freq >= 1000.0 {
        let khz = freq / 1000.0;
        if khz >= 10.0 {
            format!("{:.0}k ", khz)
        } else {
            format!("{:.1}k ", khz)
        }
    } else {
        format!("{:.0} ", freq)
    }
}

struct SpectrogramWidget<'a> {
    data: &'a SpectrogramData,
    playback_fraction: f64,
}

impl<'a> Widget for SpectrogramWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 || self.data.num_time_bins == 0 {
            return;
        }

        let width = area.width as usize;
        let display_rows = area.height as usize * 2;

        let range_db = (self.data.max_db - self.data.min_db).max(1.0);

        let cursor_col = (self.playback_fraction * width as f64) as usize;

        for col in 0..width {
            let time_bin =
                (col as f64 / width as f64 * self.data.num_time_bins as f64) as usize;
            let time_bin = time_bin.min(self.data.num_time_bins - 1);

            let freq_data = &self.data.magnitudes[time_bin];

            for row in 0..area.height as usize {
                let freq_row_top = display_rows - 1 - (row * 2);
                let freq_row_bot = display_rows.saturating_sub(1).saturating_sub(row * 2 + 1);

                let val_top =
                    get_freq_value(freq_data, freq_row_top, display_rows, self.data.num_freq_bins);
                let val_bot =
                    get_freq_value(freq_data, freq_row_bot, display_rows, self.data.num_freq_bins);

                let norm_top = ((val_top - self.data.min_db) / range_db).clamp(0.0, 1.0);
                let norm_bot = ((val_bot - self.data.min_db) / range_db).clamp(0.0, 1.0);

                let color_top = value_to_color(norm_top);
                let color_bot = value_to_color(norm_bot);

                let x = area.x + col as u16;
                let y = area.y + row as u16;

                if col == cursor_col {
                    buf[(x, y)]
                        .set_char('|')
                        .set_fg(Color::Yellow)
                        .set_bg(Color::Reset);
                } else {
                    buf[(x, y)]
                        .set_char('\u{2580}')
                        .set_fg(color_top)
                        .set_bg(color_bot);
                }
            }
        }
    }
}

fn get_freq_value(
    freq_data: &[f32],
    display_row: usize,
    display_rows: usize,
    num_freq_bins: usize,
) -> f32 {
    let freq_bin = (display_row as f64 / display_rows as f64 * num_freq_bins as f64) as usize;
    let freq_bin = freq_bin.min(num_freq_bins.saturating_sub(1));
    freq_data.get(freq_bin).copied().unwrap_or(-100.0)
}

fn value_to_color(v: f32) -> Color {
    let gradient = colorous::VIRIDIS;
    let c = gradient.eval_continuous(v as f64);
    Color::Rgb(c.r, c.g, c.b)
}
