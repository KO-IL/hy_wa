use std::{fs, path::PathBuf};

use crate::model::Settings;

pub fn resolve_scan_root(settings: &Settings) -> PathBuf {
    if let Some(saved_root) = &settings.scan_root {
        if let Ok(path) = canonicalize_directory(saved_root) {
            return path;
        }
    }
    canonicalize_directory(".").unwrap_or_else(|_| PathBuf::from("."))
}

pub fn canonicalize_directory(raw: &str) -> std::io::Result<PathBuf> {
    let candidates = path_candidates(raw);
    if candidates.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "empty directory path",
        ));
    }

    let mut last_error: Option<std::io::Error> = None;
    for candidate in candidates {
        match fs::canonicalize(&candidate) {
            Ok(canonical) => {
                if canonical.is_dir() {
                    return Ok(canonical);
                }
                last_error = Some(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "not a directory",
                ));
            }
            Err(err) => last_error = Some(err),
        }
    }
    Err(last_error.unwrap_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "cannot resolve input directory",
        )
    }))
}

pub fn normalize_and_maybe_canonicalize_path(raw: &str) -> String {
    let normalized = normalize_input_path(raw);
    match fs::canonicalize(&normalized) {
        Ok(path) => path.to_string_lossy().to_string(),
        Err(_) => normalized,
    }
}

pub fn normalize_input_path(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let slashed = trimmed.replace('\\', "/");
    let bytes = slashed.as_bytes();
    if bytes.len() >= 3 && bytes[1] == b':' && bytes[2] == b'/' && bytes[0].is_ascii_alphabetic() {
        let drive = (bytes[0] as char).to_ascii_lowercase();
        if drive == 'z' {
            return slashed[2..].to_string();
        }
        return format!("/mnt/{}/{}", drive, &slashed[3..]);
    }

    slashed
}

fn path_candidates(raw: &str) -> Vec<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    let normalized = normalize_input_path(trimmed);
    if normalized == trimmed {
        vec![trimmed.to_string()]
    } else {
        vec![trimmed.to_string(), normalized]
    }
}
