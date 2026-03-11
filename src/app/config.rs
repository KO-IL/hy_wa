use std::{env, fs, path::PathBuf};

use crate::{
    constants::CONFIG_RELATIVE_PATH,
    model::Settings,
    text::{url_decode, url_encode},
};

pub fn save_settings(settings: &Settings) -> std::io::Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut body = String::new();
    body.push_str("output=");
    body.push_str(&url_encode(&settings.output));
    body.push('\n');
    body.push_str("root=");
    if let Some(root) = &settings.scan_root {
        body.push_str(&url_encode(root));
    }
    body.push('\n');
    body.push_str("last=");
    if let Some(last) = &settings.last_wallpaper {
        body.push_str(&url_encode(last));
    }
    body.push('\n');
    fs::write(path, body)
}

pub fn load_settings() -> Settings {
    let mut settings = Settings::default();
    let path = config_path();
    let Ok(body) = fs::read_to_string(path) else {
        return settings;
    };

    for line in body.lines() {
        if let Some(value) = line.strip_prefix("output=") {
            settings.output = url_decode(value);
        } else if let Some(value) = line.strip_prefix("root=") {
            let decoded = url_decode(value);
            if !decoded.is_empty() {
                settings.scan_root = Some(decoded);
            }
        } else if let Some(value) = line.strip_prefix("last=") {
            let decoded = url_decode(value);
            if !decoded.is_empty() {
                settings.last_wallpaper = Some(decoded);
            }
        }
    }
    settings
}

fn config_path() -> PathBuf {
    if let Ok(xdg_config) = env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg_config).join(CONFIG_RELATIVE_PATH);
    }
    if let Ok(home) = env::var("HOME") {
        return PathBuf::from(home)
            .join(".config")
            .join(CONFIG_RELATIVE_PATH);
    }
    PathBuf::from(".wall-set-settings.conf")
}
