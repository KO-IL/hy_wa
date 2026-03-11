use std::path::PathBuf;

use crate::constants::DEFAULT_OUTPUT;

#[derive(Clone)]
pub struct Settings {
    pub output: String,
    pub scan_root: Option<String>,
    pub last_wallpaper: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            output: DEFAULT_OUTPUT.to_string(),
            scan_root: None,
            last_wallpaper: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MediaKind {
    Image,
    Video,
    Project,
    Other,
}

impl MediaKind {
    pub fn as_str(self) -> &'static str {
        match self {
            MediaKind::Image => "image",
            MediaKind::Video => "video",
            MediaKind::Project => "project",
            MediaKind::Other => "other",
        }
    }
}

#[derive(Clone)]
pub struct WallpaperEntry {
    pub path: String,
    pub name: String,
    pub kind: MediaKind,
    pub thumb: Option<String>,
}

pub struct AppState {
    pub root: PathBuf,
    pub engine_bin: String,
    pub resolved_engine_bin: String,
    pub engine_workdir: Option<String>,
    pub engine_ld_library_path: Option<String>,
    pub settings: Settings,
    pub wallpapers: Vec<WallpaperEntry>,
    pub active_engine_pid: Option<u32>,
}
