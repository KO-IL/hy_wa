use std::{
    borrow::Cow,
    fs,
    path::{Path, PathBuf},
};

use crate::{
    constants::{IMAGE_EXTENSIONS, VIDEO_EXTENSIONS},
    model::{MediaKind, WallpaperEntry},
};

pub fn scan_wallpapers(root: &Path) -> Vec<WallpaperEntry> {
    let mut output: Vec<WallpaperEntry> = Vec::new();
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        if let Some(project_entry) = scan_wallpaperengine_project(&dir) {
            output.push(project_entry);
            continue;
        }

        let Ok(read_dir) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }

            let kind = classify_path(&path);
            if kind == MediaKind::Other {
                continue;
            }

            let Some(path_str) = path.to_str() else {
                continue;
            };
            let name = path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or(path_str)
                .to_string();
            output.push(WallpaperEntry {
                path: path_str.to_string(),
                name,
                kind,
                thumb: (kind == MediaKind::Image).then(|| path_str.to_string()),
            });
        }
    }

    output.sort_unstable_by(|left, right| left.name.cmp(&right.name));
    output
}

pub fn resolve_project_dir(path: &Path) -> Option<String> {
    if path.is_dir() && path.join("project.json").is_file() {
        return path.to_str().map(|value| value.to_string());
    }

    if path.is_file() {
        let parent = path.parent()?;
        let project_file = parent.join("project.json");
        if project_file.is_file() {
            return parent.to_str().map(|value| value.to_string());
        }
    }

    None
}

pub fn classify_target(path: &Path) -> MediaKind {
    if path.is_dir() && path.join("project.json").is_file() {
        return MediaKind::Project;
    }
    classify_path(path)
}

pub fn resolve_allowed_media_path(root: &Path, path: &Path) -> Option<PathBuf> {
    let root_prefix = if root.is_absolute() {
        Cow::Borrowed(root)
    } else {
        Cow::Owned(fs::canonicalize(root).ok()?)
    };
    let path_canonical = fs::canonicalize(path).ok()?;
    if !path_canonical.starts_with(root_prefix.as_ref()) {
        return None;
    }

    if classify_path(&path_canonical) != MediaKind::Other {
        return Some(path_canonical);
    }

    (path_canonical.is_dir() && path_canonical.join("project.json").is_file())
        .then_some(path_canonical)
}

pub fn classify_path(path: &Path) -> MediaKind {
    let Some(ext) = path.extension().and_then(|value| value.to_str()) else {
        return MediaKind::Other;
    };
    if IMAGE_EXTENSIONS
        .iter()
        .any(|item| item.eq_ignore_ascii_case(ext))
    {
        return MediaKind::Image;
    }
    if VIDEO_EXTENSIONS
        .iter()
        .any(|item| item.eq_ignore_ascii_case(ext))
    {
        return MediaKind::Video;
    }
    MediaKind::Other
}

fn scan_wallpaperengine_project(dir: &Path) -> Option<WallpaperEntry> {
    let project_file = dir.join("project.json");
    if !project_file.is_file() {
        return None;
    }

    let content = fs::read_to_string(project_file).ok()?;
    let name = extract_json_string_field(&content, "title").unwrap_or_else(|| {
        dir.file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("Wallpaper Engine Project")
            .to_string()
    });
    let thumb = extract_json_string_field(&content, "preview")
        .map(|relative| dir.join(relative))
        .filter(|path| path.is_file())
        .and_then(|path| path.to_str().map(|value| value.to_string()));
    let path = dir.to_str()?.to_string();

    Some(WallpaperEntry {
        path,
        name,
        kind: MediaKind::Project,
        thumb,
    })
}

fn extract_json_string_field(content: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\"", key);
    let start = content.find(&needle)?;
    let rest = &content[start + needle.len()..];
    let colon = rest.find(':')?;
    let mut chars = rest[colon + 1..].trim_start().chars();
    if chars.next()? != '"' {
        return None;
    }

    let mut value = String::new();
    let mut escaped = false;
    for ch in chars {
        if escaped {
            value.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '"' {
            return Some(value);
        }
        value.push(ch);
    }
    None
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        process,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{resolve_allowed_media_path, scan_wallpapers};
    use crate::model::MediaKind;

    fn make_temp_dir(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("wall-set-{name}-{}-{stamp}", process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn resolve_allowed_media_path_rejects_files_outside_root() {
        let root = make_temp_dir("root");
        let outside = make_temp_dir("outside");
        let file = outside.join("preview.png");
        fs::write(&file, b"png").unwrap();

        let allowed = resolve_allowed_media_path(&root, &file);
        assert!(allowed.is_none());

        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(outside).unwrap();
    }

    #[test]
    fn scan_wallpapers_detects_project_preview() {
        let root = make_temp_dir("scan");
        let project = root.join("demo-project");
        fs::create_dir_all(&project).unwrap();
        fs::write(project.join("preview.png"), b"png").unwrap();
        fs::write(
            project.join("project.json"),
            r#"{"title":"Demo","preview":"preview.png"}"#,
        )
        .unwrap();

        let wallpapers = scan_wallpapers(&root);
        assert_eq!(wallpapers.len(), 1);
        assert_eq!(wallpapers[0].name, "Demo");
        assert_eq!(wallpapers[0].kind, MediaKind::Project);
        assert_eq!(
            wallpapers[0].thumb.as_deref(),
            project.join("preview.png").to_str()
        );

        fs::remove_dir_all(root).unwrap();
    }
}
