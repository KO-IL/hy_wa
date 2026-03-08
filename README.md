# wall-set

A wallpaper manager for Linux that integrates with Wallpaper Engine (Linux version).

## Features

- Web-based GUI for browsing and selecting wallpapers
- Command-line support for applying wallpapers
- Automatic scanning of wallpaper directories
- Multi-monitor support (specify output display)
- Restore previous wallpaper on startup

## Requirements

- Linux with a desktop environment
- [Wallpaper Engine for Linux](https://github.com/ALEXandratods/Wallpaper-Engine-for-Linux) (optional, for using Wallpaper Engine backgrounds)
- Rust toolchain (for building)

## Installation

```bash
cargo build --release
sudo cp target/release/wall-set /usr/local/bin/
```

## Usage

### GUI Mode (Default)

Run the web interface:

```bash
wall-set
```

Access the GUI at http://localhost:7878

### Command Line Mode

Apply a wallpaper directly:

```bash
wall-set /path/to/wallpaper/project.json
```

Restore the last saved wallpaper:

```bash
wall-set restore
```

### Configuration

- **Scan Root**: Directory to scan for wallpapers (default: current directory)
- **Output**: Display output to use (e.g., `DP-1`, `HDMI-1`)

Set environment variable for Wallpaper Engine binary:

```bash
export LINUX_WALLPAPERENGINE_BIN=/path/to/wallpaper-engine
```

### Auto-start

Example autostart desktop file is provided in `autostart/`.

## License

MIT
