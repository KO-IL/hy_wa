use std::{path::Path, thread, time::Duration};

use crate::{
    config::save_settings,
    model::AppState,
    paths::normalize_and_maybe_canonicalize_path,
    scanner::{classify_target, resolve_allowed_media_path},
    wallpaper::apply_wallpaper,
};

pub fn apply_and_save(state: &mut AppState, path: &str, enforce_root: bool) -> i32 {
    let normalized = normalize_and_maybe_canonicalize_path(path);
    let target = if enforce_root {
        match resolve_allowed_media_path(&state.root, Path::new(&normalized)) {
            Some(path) => path.to_string_lossy().to_string(),
            None => {
                eprintln!(
                    "Rejected path outside scan root or unsupported format: {}",
                    normalized
                );
                return 1;
            }
        }
    } else {
        normalized
    };

    let code = apply_wallpaper(state, &target);
    if code == 0 && state.settings.last_wallpaper.as_deref() != Some(target.as_str()) {
        state.settings.last_wallpaper = Some(target);
        let _ = save_settings(&state.settings);
    }
    code
}

pub fn restore_last_wallpaper(state: &mut AppState) {
    let Some(last) = state.settings.last_wallpaper.clone() else {
        return;
    };
    let normalized = normalize_and_maybe_canonicalize_path(&last);
    if !Path::new(&normalized).exists() {
        return;
    }

    let max_attempts = if matches!(
        classify_target(Path::new(&normalized)),
        crate::model::MediaKind::Video
            | crate::model::MediaKind::Project
            | crate::model::MediaKind::Other
    ) {
        6
    } else {
        2
    };

    for attempt in 1..=max_attempts {
        let code = apply_wallpaper(state, &normalized);
        if code == 0 {
            if normalized != last {
                state.settings.last_wallpaper = Some(normalized);
                let _ = save_settings(&state.settings);
            }
            return;
        }

        if attempt == max_attempts {
            eprintln!(
                "Failed to restore last wallpaper after {} attempts: {}",
                attempt, normalized
            );
            return;
        }

        eprintln!(
            "Restore attempt {attempt}/{max_attempts} failed for {}; retrying in 2s.",
            normalized
        );
        thread::sleep(Duration::from_secs(2));
    }
}
