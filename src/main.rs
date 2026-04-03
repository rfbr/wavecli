mod analysis;
mod app;
mod decoder;
mod player;
mod ui;

use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(name = "wavecli", about = "TUI audio player and analyzer")]
struct Cli {
    /// Path to an audio file or directory (defaults to current directory)
    path: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let path = cli.path.unwrap_or_else(|| PathBuf::from("."));

    if !path.exists() {
        eprintln!("Path not found: {}", path.display());
        std::process::exit(1);
    }

    let app = app::App::new(&path)?;

    let terminal = ratatui::init();
    let result = app.run(terminal);
    ratatui::restore();

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }

    Ok(())
}
