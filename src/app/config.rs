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
    let mut projects: Vec<_> = settings.project_overrides.iter().collect();
    projects.sort_unstable_by(|left, right| left.0.cmp(right.0));
    for (project, values) in projects {
        let mut entries: Vec<_> = values.iter().collect();
        entries.sort_unstable_by(|left, right| left.0.cmp(right.0));
        for (key, value) in entries {
            body.push_str("prop=");
            body.push_str(&url_encode(project));
            body.push('\t');
            body.push_str(&url_encode(key));
            body.push('\t');
            body.push_str(&url_encode(value));
            body.push('\n');
        }
    }
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
        } else if let Some(value) = line.strip_prefix("prop=") {
            let mut parts = value.splitn(3, '\t');
            let Some(project) = parts.next() else {
                continue;
            };
            let Some(key) = parts.next() else {
                continue;
            };
            let Some(raw_value) = parts.next() else {
                continue;
            };

            let project = url_decode(project);
            let key = url_decode(key);
            let raw_value = url_decode(raw_value);
            if project.is_empty() || key.is_empty() {
                continue;
            }

            settings
                .project_overrides
                .entry(project)
                .or_default()
                .insert(key, raw_value);
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

#[cfg(test)]
mod tests {
    use super::load_settings;
    use std::{fs, process, time::{SystemTime, UNIX_EPOCH}};

    #[test]
    fn load_settings_reads_project_overrides() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("wall-set-config-{}-{stamp}", process::id()));
        fs::create_dir_all(&root).unwrap();
        let config_dir = root.join("wall-set");
        fs::create_dir_all(&config_dir).unwrap();
        let config_path = config_dir.join("settings.conf");
        fs::write(
            &config_path,
            "output=DP-3\nroot=%2Ftmp\nlast=%2Ftmp%2Fdemo\nprop=%2Ftmp%2Fdemo\tgod_rays\t0\nprop=%2Ftmp%2Fdemo\taccent\t0.1%2C%200.2%2C%200.3\n",
        )
        .unwrap();

        std::env::set_var("XDG_CONFIG_HOME", &root);
        let settings = load_settings();

        assert_eq!(settings.project_overrides["/tmp/demo"]["god_rays"], "0");
        assert_eq!(
            settings.project_overrides["/tmp/demo"]["accent"],
            "0.1, 0.2, 0.3"
        );

        std::env::remove_var("XDG_CONFIG_HOME");
        fs::remove_dir_all(root).unwrap();
    }
}
