use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::DefaultTerminal;

use crate::analysis::spectrogram::{compute_spectrogram, SpectrogramData};
use crate::decoder::{self, AudioData};
use crate::player::Player;
use crate::ui;

const AUDIO_EXTENSIONS: &[&str] = &[
    "wav", "mp3", "flac", "ogg", "aac", "opus", "m4a", "sph", "wma",
];

#[derive(PartialEq)]
enum Focus {
    FileBrowser,
    Player,
}

struct LoadedFile {
    audio: AudioData,
    player: Player,
    spectrogram: SpectrogramData,
    filename: String,
}

pub struct App {
    loaded: Option<LoadedFile>,
    log_lines: Vec<String>,
    show_waveform: bool,
    show_spectrogram: bool,
    show_file_browser: bool,
    should_quit: bool,
    focus: Focus,
    directory: PathBuf,
    audio_files: Vec<String>,
    filtered_files: Vec<String>,
    filter: String,
    selected_index: usize,
}

impl App {
    pub fn new(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let abs_path = std::fs::canonicalize(path)?;

        let (directory, initial_file) = if abs_path.is_dir() {
            (abs_path, None)
        } else {
            let dir = abs_path.parent().unwrap_or(Path::new(".")).to_path_buf();
            let name = abs_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();
            (dir, Some(name))
        };

        let audio_files = scan_audio_files(&directory);
        let filtered_files = audio_files.clone();

        let selected_index = initial_file
            .as_ref()
            .and_then(|name| filtered_files.iter().position(|f| f == name))
            .unwrap_or(0);

        let loaded = match &initial_file {
            Some(name) => {
                let file_path = directory.join(name);
                match load_audio(&file_path) {
                    Ok((audio, spectrogram, player)) => Some(LoadedFile {
                        audio,
                        player,
                        spectrogram,
                        filename: name.clone(),
                    }),
                    Err(_) => None,
                }
            }
            None => None,
        };

        let focus = if loaded.is_some() {
            Focus::Player
        } else {
            Focus::FileBrowser
        };

        Ok(App {
            loaded,
            log_lines: Vec::new(),
            show_waveform: true,
            show_spectrogram: true,
            show_file_browser: true,
            should_quit: false,
            focus,
            directory,
            audio_files,
            filtered_files,
            filter: String::new(),
            selected_index,
        })
    }

    fn update_filtered_files(&mut self) {
        if self.filter.is_empty() {
            self.filtered_files = self.audio_files.clone();
        } else {
            let query = self.filter.to_lowercase();
            self.filtered_files = self
                .audio_files
                .iter()
                .filter(|f| f.to_lowercase().contains(&query))
                .cloned()
                .collect();
        }
        // Clamp selection
        if self.filtered_files.is_empty() {
            self.selected_index = 0;
        } else if self.selected_index >= self.filtered_files.len() {
            self.selected_index = self.filtered_files.len() - 1;
        }
    }

    pub fn run(mut self, mut terminal: DefaultTerminal) -> io::Result<()> {
        let tick_rate = Duration::from_millis(33);

        loop {
            if let Some(ref mut lf) = self.loaded {
                lf.player.tick();
            }
            terminal.draw(|f| self.draw(f))?;

            if self.should_quit {
                return Ok(());
            }

            if event::poll(tick_rate)? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.handle_key(key.code, &mut terminal);
                    }
                }
            }
        }
    }

    fn draw(&self, f: &mut ratatui::Frame) {
        let area = f.area();
        let layout = ui::layout::build_layout(
            area,
            self.show_waveform && self.loaded.is_some(),
            self.show_spectrogram && self.loaded.is_some(),
            self.show_file_browser,
        );

        if self.show_file_browser {
            let current_name = self
                .loaded
                .as_ref()
                .map(|lf| lf.filename.as_str())
                .unwrap_or("");
            ui::file_browser::render(
                f,
                layout.file_browser,
                &self.filtered_files,
                self.selected_index,
                current_name,
                self.focus == Focus::FileBrowser,
                &self.filter,
            );
        }

        match &self.loaded {
            Some(lf) => {
                ui::info_table::render(
                    f,
                    layout.info,
                    &lf.filename,
                    &lf.audio.format_name,
                    &lf.audio.codec,
                    lf.audio.sample_rate,
                    lf.audio.channels,
                    lf.audio.duration_secs,
                );

                let playback_fraction = lf.player.position_fraction();

                if self.show_waveform {
                    ui::waveform::render(
                        f,
                        layout.waveform,
                        &lf.audio.samples[0],
                        playback_fraction,
                        0,
                        lf.audio.channels,
                    );
                }

                if self.show_spectrogram {
                    ui::spectrogram::render(
                        f,
                        layout.spectrogram,
                        &lf.spectrogram,
                        playback_fraction,
                    );
                }

                ui::controls::render(
                    f,
                    layout.controls,
                    lf.player.is_playing(),
                    lf.player.position_secs(),
                    lf.player.duration_secs(),
                    lf.player.volume(),
                    playback_fraction,
                );
            }
            None => {
                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title(" Audio Preview ");

                let lines: Vec<Line> = if !self.log_lines.is_empty() {
                    self.log_lines
                        .iter()
                        .map(|msg| {
                            let color = if msg.starts_with("Error") {
                                Color::Red
                            } else if msg.starts_with("  ") {
                                Color::White
                            } else {
                                Color::Cyan
                            };
                            Line::from(ratatui::text::Span::styled(
                                msg.clone(),
                                Style::default().fg(color),
                            ))
                        })
                        .collect()
                } else if self.audio_files.is_empty() {
                    vec![Line::from("No audio files found in this directory")]
                } else {
                    vec![Line::from("Select a file and press Enter to load")]
                };

                let paragraph = Paragraph::new(lines)
                    .style(Style::default().fg(Color::DarkGray))
                    .block(block);
                let combined = ratatui::layout::Rect {
                    x: layout.info.x,
                    y: layout.info.y,
                    width: layout.info.width,
                    height: area
                        .height
                        .saturating_sub(layout.info.y.saturating_sub(area.y)),
                };
                f.render_widget(paragraph, combined);
            }
        }
    }

    fn handle_key(&mut self, key: KeyCode, terminal: &mut DefaultTerminal) {
        match key {
            KeyCode::Char('q') if self.focus != Focus::FileBrowser || self.filter.is_empty() => {
                self.should_quit = true;
                return;
            }
            KeyCode::Esc => {
                if self.focus == Focus::FileBrowser && !self.filter.is_empty() {
                    // Clear filter first
                    self.filter.clear();
                    self.update_filtered_files();
                    return;
                }
                self.should_quit = true;
                return;
            }
            KeyCode::Tab => {
                if self.show_file_browser {
                    self.focus = match self.focus {
                        Focus::FileBrowser => {
                            if self.loaded.is_some() {
                                Focus::Player
                            } else {
                                Focus::FileBrowser
                            }
                        }
                        Focus::Player => Focus::FileBrowser,
                    };
                }
                return;
            }
            KeyCode::Char('f') if self.focus == Focus::Player => {
                self.show_file_browser = !self.show_file_browser;
                if !self.show_file_browser && self.loaded.is_some() {
                    self.focus = Focus::Player;
                } else if !self.show_file_browser && self.loaded.is_none() {
                    self.show_file_browser = true;
                }
                return;
            }
            _ => {}
        }

        match self.focus {
            Focus::FileBrowser => self.handle_browser_key(key, terminal),
            Focus::Player => self.handle_player_key(key),
        }
    }

    fn handle_browser_key(&mut self, key: KeyCode, terminal: &mut DefaultTerminal) {
        match key {
            KeyCode::Up | KeyCode::Char('k') if self.filter.is_empty() => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') if self.filter.is_empty() => {
                if self.selected_index + 1 < self.filtered_files.len() {
                    self.selected_index += 1;
                }
            }
            KeyCode::Up => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            KeyCode::Down => {
                if self.selected_index + 1 < self.filtered_files.len() {
                    self.selected_index += 1;
                }
            }
            KeyCode::Enter => {
                self.load_selected_file(terminal);
            }
            KeyCode::Backspace => {
                self.filter.pop();
                self.update_filtered_files();
            }
            KeyCode::Char(c) => {
                self.filter.push(c);
                self.update_filtered_files();
            }
            _ => {}
        }
    }

    fn handle_player_key(&mut self, key: KeyCode) {
        let Some(ref mut lf) = self.loaded else {
            return;
        };
        match key {
            KeyCode::Char(' ') => lf.player.toggle_play(),
            KeyCode::Left => lf.player.seek_relative(-5.0),
            KeyCode::Right => lf.player.seek_relative(5.0),
            KeyCode::Up => lf.player.adjust_volume(0.05),
            KeyCode::Down => lf.player.adjust_volume(-0.05),
            KeyCode::Char('w') => self.show_waveform = !self.show_waveform,
            KeyCode::Char('s') => self.show_spectrogram = !self.show_spectrogram,
            _ => {}
        }
    }

    fn log_and_draw(&mut self, msg: &str, terminal: &mut DefaultTerminal) {
        self.log_lines.push(msg.to_string());
        let _ = terminal.draw(|f| self.draw(f));
    }

    fn load_selected_file(&mut self, terminal: &mut DefaultTerminal) {
        if self.selected_index >= self.filtered_files.len() {
            return;
        }

        let name = self.filtered_files[self.selected_index].clone();
        if self
            .loaded
            .as_ref()
            .map(|lf| lf.filename == name)
            .unwrap_or(false)
        {
            return;
        }

        self.loaded = None;
        self.log_lines.clear();

        let path = self.directory.join(&name);

        // Step 1: Decode
        self.log_and_draw(&format!("Decoding {name}..."), terminal);

        let audio = match decoder::decode_file(&path) {
            Ok(a) => a,
            Err(e) => {
                self.log_and_draw(&format!("Error: {e}"), terminal);
                return;
            }
        };

        self.log_and_draw(
            &format!(
                "  {} | {}Hz | {}ch | {:.1}s | {} samples",
                audio.format_name,
                audio.sample_rate,
                audio.channels,
                audio.duration_secs,
                audio.total_samples,
            ),
            terminal,
        );

        // Step 2: Spectrogram
        self.log_and_draw("Computing spectrogram...", terminal);

        let mono_samples = if audio.channels == 1 {
            audio.samples[0].clone()
        } else {
            let len = audio.samples[0].len();
            let mut mono = vec![0.0f32; len];
            for ch in &audio.samples {
                for (i, s) in ch.iter().enumerate() {
                    mono[i] += s / audio.channels as f32;
                }
            }
            mono
        };

        let window_size = 1024;
        let hop_size = window_size / 2;
        let spectrogram =
            compute_spectrogram(&mono_samples, audio.sample_rate, window_size, hop_size);

        self.log_and_draw(
            &format!(
                "  {} time bins x {} freq bins",
                spectrogram.num_time_bins, spectrogram.num_freq_bins
            ),
            terminal,
        );

        // Step 3: Audio output
        self.log_and_draw("Initializing audio output...", terminal);

        let player = Player::new(&audio.samples, audio.sample_rate, audio.channels);

        if player.has_audio_device() {
            self.log_and_draw("  Audio device ready", terminal);
        } else {
            self.log_and_draw("  No audio device (visualization only)", terminal);
        }

        self.loaded = Some(LoadedFile {
            audio,
            player,
            spectrogram,
            filename: name,
        });
        self.log_lines.clear();
        self.show_file_browser = false;
        self.focus = Focus::Player;
    }
}

fn scan_audio_files(dir: &Path) -> Vec<String> {
    let mut files: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if !path.is_file() {
                return None;
            }
            let ext = path.extension()?.to_str()?.to_lowercase();
            if AUDIO_EXTENSIONS.contains(&ext.as_str()) {
                Some(entry.file_name().to_str()?.to_string())
            } else {
                None
            }
        })
        .collect();

    files.sort_unstable();
    files
}

fn load_audio(
    path: &Path,
) -> Result<(AudioData, SpectrogramData, Player), Box<dyn std::error::Error>> {
    let audio = decoder::decode_file(path)?;

    let mono_samples = if audio.channels == 1 {
        audio.samples[0].clone()
    } else {
        let len = audio.samples[0].len();
        let mut mono = vec![0.0f32; len];
        for ch in &audio.samples {
            for (i, s) in ch.iter().enumerate() {
                mono[i] += s / audio.channels as f32;
            }
        }
        mono
    };

    let window_size = 1024;
    let hop_size = window_size / 2;
    let spectrogram = compute_spectrogram(&mono_samples, audio.sample_rate, window_size, hop_size);

    let player = Player::new(&audio.samples, audio.sample_rate, audio.channels);

    Ok((audio, spectrogram, player))
}
