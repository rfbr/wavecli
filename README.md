# wavecli

A terminal audio player with waveform and spectrogram visualization. Inspired by the [Audio Preview](https://marketplace.visualstudio.com/items?itemName=sukumo28.wav-preview) VS Code extension.

![wavecli screenshot](https://github.com/user-attachments/assets/375de0c5-7893-4192-b227-d9ff77ad602a)

## Install

```bash
cargo install wavecli
```

Or from source:

```bash
cargo install --path .
```

On Linux, you'll need ALSA dev headers:

```bash
sudo apt install libasound2-dev
```

## Usage

```bash
# Open a file
wavecli song.mp3

# Browse a directory
wavecli /path/to/audio/

# Browse current directory
wavecli
```

Supports WAV, MP3, FLAC, OGG, AAC, Opus, and M4A.

## Controls

| Key | Action |
|---|---|
| Space | Play / pause |
| Left / Right | Seek 5s |
| Up / Down | Volume |
| Tab | Switch focus (file browser / player) |
| Enter | Load selected file |
| f | Toggle file browser |
| w | Toggle waveform |
| s | Toggle spectrogram |
| Esc | Clear filter / quit |
| q | Quit |

When the file browser is focused, type to filter files by name.

## Features

- Decode and play audio files (via Symphonia + rodio)
- Waveform display with playback cursor
- Spectrogram with frequency axis and viridis colormap
- File browser with search filter
- Works without an audio device (visualization-only mode)
