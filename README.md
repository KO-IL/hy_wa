<div align="center">
  <h1>wall-set</h1>
  <p>A lightweight Wayland wallpaper manager for images, videos, and Wallpaper Engine projects.</p>
  <p>
    <a href="README.zh-CN.md">中文说明</a> ·
    <a href="docs/assets/gui-demo.mp4">Demo Video</a>
  </p>
  <a href="docs/assets/gui-demo.mp4">
    <img src="docs/assets/githubhy.png" alt="wall-set GUI preview" width="900" />
  </a>
</div>

`wall-set` is a small Rust wallpaper manager for Linux desktop setups that use Wayland. It gives you a local web UI to browse a wallpaper library, apply static images through `swww`, and launch Wallpaper Engine videos or project wallpapers through `linux-wallpaperengine`.

## Highlights

- Built-in local web UI with a wallpaper grid and settings drawer
- Supports still images, video wallpapers, and Wallpaper Engine projects
- Restores the last applied wallpaper automatically on startup
- Saves per-project Wallpaper Engine property overrides
- Lets you change scan root and output target from the UI
- Includes a per-wallpaper privacy blur toggle for hiding previews in the GUI

## Features

- Local web UI served from `127.0.0.1:7878`
- Recursive wallpaper scan under a configurable root directory
- Supports image files, video files, and Wallpaper Engine projects
- Remembers the last applied wallpaper and restores it on startup
- Can apply a wallpaper directly from the CLI
- Stores per-project Wallpaper Engine property overrides
- Lets you change the scan root and target output from the UI
- Lets you blur individual wallpaper previews from the GUI for privacy

## Supported Media

- Images: `jpg`, `jpeg`, `png`, `bmp`, `gif`, `webp`
- Videos: `mp4`, `mkv`, `webm`, `mov`, `avi`, `m4v`
- Wallpaper Engine projects: directories containing `project.json`

## Runtime Requirements

- Linux with Wayland
- Rust toolchain for building
- [`swww`](https://github.com/LGFae/swww) for image wallpapers
- [`linux-wallpaperengine`](https://github.com/Almamu/linux-wallpaperengine) for video and project wallpapers

`wall-set` assumes `linux-wallpaperengine` is available in `PATH`. If it is installed somewhere else, set `LINUX_WALLPAPERENGINE_BIN` before running the app.

## Build

```bash
cargo build --release
sudo install -Dm755 target/release/wall-set /usr/local/bin/wall-set
```

## Usage

### Web UI

Start the built-in server:

```bash
wall-set
```

Then open:

```text
http://127.0.0.1:7878
```

When the GUI starts, it also tries to restore the last wallpaper saved in the config.

### CLI

Apply a wallpaper directly:

```bash
wall-set /path/to/image.png
wall-set /path/to/video.mp4
wall-set /path/to/project-directory
```

Restore the last saved wallpaper:

```bash
wall-set restore
```

## Configuration

The config file is stored at:

```text
~/.config/wall-set/settings.conf
```

Example:

```ini
output=DP-3
root=/path/to/wallpapers
last=/path/to/wallpapers/example.png
prop=/path/to/project\tgod_rays\t0
```

Fields:

- `output`: target display output used by the wallpaper backend
- `root`: directory scanned by the web UI
- `last`: last wallpaper that was applied successfully
- `prop`: saved Wallpaper Engine property override entries

## Environment Variables

- `LINUX_WALLPAPERENGINE_BIN`: override the Wallpaper Engine executable path
- `WALL_SET_ENGINE_DEBUG=1`: show engine command output for debugging

## Autostart Notes

The `autostart/` directory contains desktop entries and helper scripts for restoring wallpapers on login. The shipped `.desktop` files and scripts currently contain machine-specific absolute paths such as `/home/wang/hw/wall-set`, so adjust them before using them on another system.

Typical setup:

```bash
cp autostart/wall-set.desktop ~/.config/autostart/
cp autostart/wall-set-gui.desktop ~/.config/autostart/
```

## Project Layout

- `src/main.rs`: entry point and mode selection
- `src/app/`: settings, runtime state, and state transitions
- `src/fs/`: scan-root resolution and wallpaper discovery
- `src/engine/`: wallpaper engine launch and project-property handling
- `src/gui/`: local HTTP server and embedded HTML UI
- `autostart/`: login startup helpers

## Current Scope

This project is intentionally lightweight. It uses a custom TCP/HTTP server and a single embedded HTML page instead of a full GUI framework, which keeps deployment simple but also means there is no authentication layer, database, or desktop integration beyond the provided scripts.
