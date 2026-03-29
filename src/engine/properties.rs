use std::{
    io,
    process::{Command, Stdio},
};

use crate::{
    model::{AppState, ProjectProperty, ProjectPropertyOption},
    scanner::resolve_project_dir,
};

pub fn list_project_properties(state: &AppState, wallpaper: &str) -> io::Result<Vec<ProjectProperty>> {
    let Some(project_dir) = resolve_project_dir(std::path::Path::new(wallpaper)) else {
        return Ok(Vec::new());
    };

    let mut cmd = Command::new(&state.resolved_engine_bin);
    cmd.arg("--list-properties");
    append_project_overrides(state, &project_dir, &mut cmd);
    cmd.arg(&project_dir);
    if let Some(workdir) = &state.engine_workdir {
        cmd.current_dir(workdir);
    }
    if let Some(ld_path) = state.engine_ld_library_path.as_deref() {
        cmd.env("LD_LIBRARY_PATH", ld_path);
    }
    cmd.stdin(Stdio::null()).stderr(Stdio::piped()).stdout(Stdio::piped());

    let output = cmd.output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let message = stderr.trim().if_empty(stdout.trim()).unwrap_or("failed to list properties");
        return Err(io::Error::other(message.to_string()));
    }

    Ok(parse_project_properties(&String::from_utf8_lossy(&output.stdout)))
}

pub fn append_project_overrides(state: &AppState, wallpaper: &str, cmd: &mut Command) {
    let Some(project_dir) = resolve_project_dir(std::path::Path::new(wallpaper))
        .or_else(|| std::path::Path::new(wallpaper).is_dir().then(|| wallpaper.to_string()))
    else {
        return;
    };

    let Some(overrides) = state.project_overrides.get(&project_dir) else {
        return;
    };

    let mut pairs: Vec<(&String, &String)> = overrides.iter().collect();
    pairs.sort_unstable_by(|left, right| left.0.cmp(right.0));
    for (key, value) in pairs {
        cmd.args(["--set-property", &format!("{key}={value}")]);
    }
}

fn parse_project_properties(output: &str) -> Vec<ProjectProperty> {
    let mut properties = Vec::new();
    let mut current: Option<ProjectProperty> = None;
    let mut in_options = false;

    for raw_line in output.lines() {
        let line = raw_line.trim_end();
        let trimmed = line.trim();

        if trimmed.is_empty() {
            in_options = false;
            continue;
        }

        if line.starts_with("Running with:") || line.starts_with("Applying override value for ") {
            continue;
        }

        if !line.starts_with(char::is_whitespace) && trimmed.contains(" - ") {
            if let Some(property) = current.take() {
                properties.push(finalize_property(property));
            }
            let (key, kind) = trimmed.split_once(" - ").unwrap();
            current = Some(ProjectProperty {
                key: key.trim().to_string(),
                kind: kind.trim().to_string(),
                ..ProjectProperty::default()
            });
            in_options = false;
            continue;
        }

        let Some(property) = current.as_mut() else {
            continue;
        };

        if in_options {
            if let Some(option) = parse_property_option(trimmed) {
                property.options.push(option);
                continue;
            }
            in_options = false;
        }

        let Some((field, value)) = trimmed.split_once(':') else {
            continue;
        };
        let value = value.trim();
        match field.trim().to_ascii_lowercase().as_str() {
            "text" => property.label = value.to_string(),
            "min" => property.min = Some(value.to_string()),
            "max" => property.max = Some(value.to_string()),
            "step" => property.step = Some(value.to_string()),
            "value" => property.value = value.to_string(),
            "options" | "values" => {
                in_options = true;
                if let Some(option) = parse_property_option(value) {
                    property.options.push(option);
                }
            }
            _ => {}
        }
    }

    if let Some(property) = current.take() {
        properties.push(finalize_property(property));
    }

    properties
}

fn finalize_property(mut property: ProjectProperty) -> ProjectProperty {
    if property.label.trim().is_empty() {
        property.label = property.key.replace('_', " ");
    }
    property
}

fn parse_property_option(value: &str) -> Option<ProjectPropertyOption> {
    let trimmed = value.trim().trim_start_matches('-').trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some((label, raw_value)) = trimmed.split_once('=') {
        return Some(ProjectPropertyOption {
            value: raw_value.trim().to_string(),
            label: label.trim().to_string(),
        });
    }
    if let Some((raw_value, label)) = trimmed.split_once(" - ") {
        return Some(ProjectPropertyOption {
            value: raw_value.trim().to_string(),
            label: label.trim().to_string(),
        });
    }
    if let Some((raw_value, label)) = trimmed.split_once(':') {
        return Some(ProjectPropertyOption {
            value: raw_value.trim().to_string(),
            label: label.trim().to_string(),
        });
    }

    Some(ProjectPropertyOption {
        value: trimmed.to_string(),
        label: trimmed.to_string(),
    })
}

trait IfEmpty {
    fn if_empty<'a>(&'a self, fallback: &'a str) -> Option<&'a str>;
}

impl IfEmpty for str {
    fn if_empty<'a>(&'a self, fallback: &'a str) -> Option<&'a str> {
        if !self.is_empty() {
            Some(self)
        } else if !fallback.is_empty() {
            Some(fallback)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_project_properties;

    #[test]
    fn parse_project_properties_extracts_supported_fields() {
        let body = r#"Running with: ./linux-wallpaperengine --list-properties /tmp/demo
toggle_feature - boolean
    Text: Toggle Feature
    Value: 1

speed - slider
    Text: Speed
    Min: 0
    Max: 100
    Step: 5
    Value: 45

color_key - color
    Value: 0.1, 0.2, 0.3
"#;

        let properties = parse_project_properties(body);
        assert_eq!(properties.len(), 3);
        assert_eq!(properties[0].label, "Toggle Feature");
        assert_eq!(properties[0].kind, "boolean");
        assert_eq!(properties[1].min.as_deref(), Some("0"));
        assert_eq!(properties[1].step.as_deref(), Some("5"));
        assert_eq!(properties[2].label, "color key");
    }
}
