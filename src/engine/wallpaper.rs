#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::{
    collections::HashSet,
    env, fs,
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use crate::{
    model::{AppState, MediaKind},
    properties::append_project_overrides,
    scanner::{classify_target, resolve_project_dir},
};

pub fn apply_wallpaper(state: &mut AppState, wallpaper: &str) -> i32 {
    match classify_target(Path::new(wallpaper)) {
        MediaKind::Image => run_swww_image(state, wallpaper),
        MediaKind::Video | MediaKind::Project => run_wallpaperengine_project(state, wallpaper),
        MediaKind::Other => {
            stop_swww();
            run_wallpaperengine(state, wallpaper)
        }
    }
}

pub fn prepare_engine_launch(engine: &str) -> (String, Option<String>, Option<String>) {
    let (resolved_engine, engine_workdir) = resolve_engine_invocation(engine);
    let ld_library_path = build_ld_library_path(&resolved_engine);
    (resolved_engine, engine_workdir, ld_library_path)
}

fn run_wallpaperengine_project(state: &mut AppState, wallpaper: &str) -> i32 {
    let path = Path::new(wallpaper);
    let target = resolve_project_dir(path);
    let Some(target) = target else {
        eprintln!(
            "Native render needs a Wallpaper Engine project directory (with project.json): {}",
            wallpaper
        );
        return 1;
    };

    stop_swww();
    run_wallpaperengine(state, &target)
}

fn run_swww_image(state: &mut AppState, wallpaper: &str) -> i32 {
    stop_wallpaperengine(state);
    let Some(display) = wait_for_wayland_display(Duration::from_secs(15)) else {
        eprintln!("Wayland session is not ready yet; skip applying image wallpaper.");
        return 1;
    };
    if !ensure_swww_daemon_ready(Some(display.as_str())) {
        eprintln!("swww daemon is not ready; skip applying image wallpaper.");
        return 1;
    }
    let output = state.settings.output.trim();

    let mut cmd = Command::new("swww");
    cmd.arg("img");
    cmd.arg(wallpaper);
    cmd.args(["--resize", "crop"]);
    if !output.is_empty() {
        cmd.args(["--outputs", output]);
    }
    configure_wayland_env(&mut cmd, Some(display.as_str()));

    match cmd.status() {
        Ok(status) if status.success() => 0,
        Ok(status) => status.code().unwrap_or(1),
        Err(err) => {
            eprintln!("Failed to launch `swww img`: {err}");
            1
        }
    }
}

fn ensure_swww_daemon_ready(display: Option<&str>) -> bool {
    if swww_query(display) {
        return true;
    }
    start_swww_daemon(display);

    let deadline = Instant::now() + Duration::from_secs(6);
    while Instant::now() < deadline {
        if swww_query(display) {
            return true;
        }
        thread::sleep(Duration::from_millis(150));
    }
    false
}

fn swww_query(display: Option<&str>) -> bool {
    let mut cmd = Command::new("swww");
    cmd.arg("query")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    configure_wayland_env(&mut cmd, display);
    cmd.status().map(|status| status.success()).unwrap_or(false)
}

fn stop_swww() {
    let display = find_wayland_display();
    let mut cmd = Command::new("swww");
    cmd.arg("kill")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    configure_wayland_env(&mut cmd, display.as_deref());
    let _ = cmd.status();
}

fn start_swww_daemon(display: Option<&str>) {
    let mut cmd = Command::new("swww-daemon");
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    configure_wayland_env(&mut cmd, display);
    detach_from_terminal(&mut cmd);
    let _ = cmd.spawn();
}

fn run_wallpaperengine(state: &mut AppState, wallpaper: &str) -> i32 {
    stop_wallpaperengine(state);
    let Some(display) = wait_for_wayland_display(Duration::from_secs(15)) else {
        eprintln!("Wayland session is not ready yet; skip launching wallpaper engine.");
        return 1;
    };
    let output = state.settings.output.trim();
    let mut cmd = Command::new(&state.resolved_engine_bin);
    cmd.args(["--screen-root", output, "--bg", wallpaper, "--fps", "60"]);
    append_project_overrides(state, wallpaper, &mut cmd);
    if let Some(workdir) = &state.engine_workdir {
        cmd.current_dir(workdir);
    }
    configure_wayland_env(&mut cmd, Some(display.as_str()));
    if env::var("WALL_SET_ENGINE_DEBUG")
        .map(|value| value == "1")
        .unwrap_or(false)
    {
        cmd.stdin(Stdio::null());
    } else {
        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
    }
    if let Some(ld_path) = state.engine_ld_library_path.as_deref() {
        cmd.env("LD_LIBRARY_PATH", ld_path);
    }
    detach_from_terminal(&mut cmd);

    match cmd.spawn() {
        Ok(mut child) => {
            state.active_engine_pid = Some(child.id());
            let deadline = Instant::now() + Duration::from_secs(5);
            while Instant::now() < deadline {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        state.active_engine_pid = None;
                        return status.code().unwrap_or(1);
                    }
                    Ok(None) => thread::sleep(Duration::from_millis(200)),
                    Err(err) => {
                        eprintln!(
                            "Failed to query `{}` child status: {err}",
                            state.resolved_engine_bin
                        );
                        state.active_engine_pid = None;
                        return 1;
                    }
                }
            }
            0
        }
        Err(err) => {
            eprintln!("Failed to launch `{}`: {err}", state.resolved_engine_bin);
            1
        }
    }
}

fn detach_from_terminal(cmd: &mut Command) {
    #[cfg(unix)]
    cmd.process_group(0);
}

fn configure_wayland_env(cmd: &mut Command, display: Option<&str>) {
    cmd.env("XDG_SESSION_TYPE", "wayland");
    cmd.env("SDL_VIDEODRIVER", "wayland");
    cmd.env_remove("DISPLAY");

    if let Ok(runtime_dir) = env::var("XDG_RUNTIME_DIR") {
        if !runtime_dir.trim().is_empty() {
            cmd.env("XDG_RUNTIME_DIR", runtime_dir);
        }
    }
    if let Some(display) = display {
        cmd.env("WAYLAND_DISPLAY", display);
    } else if let Some(display) = find_wayland_display() {
        cmd.env("WAYLAND_DISPLAY", display);
    }
}

fn wait_for_wayland_display(timeout: Duration) -> Option<String> {
    if let Some(display) = find_wayland_display() {
        return Some(display);
    }

    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if let Some(display) = find_wayland_display() {
            return Some(display);
        }
        thread::sleep(Duration::from_millis(200));
    }
    None
}

fn find_wayland_display() -> Option<String> {
    if let Ok(display) = env::var("WAYLAND_DISPLAY") {
        if !display.trim().is_empty() {
            return Some(display);
        }
    }

    let runtime_dir = env::var("XDG_RUNTIME_DIR").ok()?;
    let mut sockets: Vec<String> = fs::read_dir(runtime_dir)
        .ok()?
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.starts_with("wayland-"))
        .collect();
    sockets.sort();
    sockets.into_iter().next()
}

fn stop_wallpaperengine(state: &mut AppState) {
    if let Some(pid) = state.active_engine_pid.take() {
        terminate_process(pid);
    }
    stop_wallpaperengine_by_name(&state.engine_bin, &state.resolved_engine_bin);
}

fn resolve_engine_invocation(engine: &str) -> (String, Option<String>) {
    let direct_engine = Path::new("/opt/linux-wallpaperengine/linux-wallpaperengine");
    if direct_engine.is_file()
        && (engine == "linux-wallpaperengine" || engine == "/usr/bin/linux-wallpaperengine")
    {
        return (
            direct_engine.display().to_string(),
            Some("/opt/linux-wallpaperengine".to_string()),
        );
    }
    (engine.to_string(), None)
}

fn terminate_process(pid: u32) {
    let pid_text = pid.to_string();
    let _ = Command::new("kill")
        .args(["-TERM", pid_text.as_str()])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    for _ in 0..12 {
        if !process_exists(pid) {
            return;
        }
        thread::sleep(Duration::from_millis(100));
    }

    let _ = Command::new("kill")
        .args(["-KILL", pid_text.as_str()])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

fn process_exists(pid: u32) -> bool {
    let pid_text = pid.to_string();
    Command::new("kill")
        .args(["-0", pid_text.as_str()])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn stop_wallpaperengine_by_name(engine: &str, resolved_engine: &str) {
    let mut patterns = HashSet::new();
    patterns.insert("linux-wallpaperengine".to_string());
    patterns.insert("/usr/bin/linux-wallpaperengine".to_string());
    patterns.insert("/opt/linux-wallpaperengine/linux-wallpaperengine".to_string());
    patterns.insert(engine.to_string());
    patterns.insert(resolved_engine.to_string());

    for pattern in patterns {
        terminate_process_by_pattern(&pattern);
    }
}

fn terminate_process_by_pattern(pattern: &str) {
    if pattern.trim().is_empty() {
        return;
    }

    let _ = Command::new("pkill")
        .args(["-TERM", "-f", pattern])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    for _ in 0..10 {
        if !process_exists_by_pattern(pattern) {
            return;
        }
        thread::sleep(Duration::from_millis(100));
    }

    let _ = Command::new("pkill")
        .args(["-KILL", "-f", pattern])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

fn process_exists_by_pattern(pattern: &str) -> bool {
    if pattern.trim().is_empty() {
        return false;
    }
    Command::new("pgrep")
        .args(["-f", pattern])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn build_ld_library_path(engine: &str) -> Option<String> {
    let mut paths: Vec<String> = Vec::new();
    let base_opt = Path::new("/opt/linux-wallpaperengine");
    if base_opt.is_dir() {
        paths.push(base_opt.display().to_string());
        paths.push(base_opt.join("lib").display().to_string());
    }

    let engine_path = Path::new(engine);
    if engine_path.is_absolute() {
        if let Some(parent) = engine_path.parent() {
            if parent.is_dir() {
                paths.push(parent.display().to_string());
            }
            let lib_dir = parent.join("lib");
            if lib_dir.is_dir() {
                paths.push(lib_dir.display().to_string());
            }
        }
    }

    if let Ok(existing) = env::var("LD_LIBRARY_PATH") {
        for entry in existing.split(':').filter(|segment| !segment.is_empty()) {
            paths.push(entry.to_string());
        }
    }

    let mut seen = HashSet::new();
    let mut deduped: Vec<String> = Vec::new();
    for entry in paths {
        if seen.insert(entry.clone()) {
            deduped.push(entry);
        }
    }
    if deduped.is_empty() {
        None
    } else {
        Some(deduped.join(":"))
    }
}
