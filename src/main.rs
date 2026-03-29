#[path = "app/config.rs"]
mod config;
#[path = "app/constants.rs"]
mod constants;
#[path = "app/instance.rs"]
mod instance;
#[path = "app/model.rs"]
mod model;
#[path = "fs/paths.rs"]
mod paths;
#[path = "fs/scanner.rs"]
mod scanner;
#[path = "app/state_ops.rs"]
mod state_ops;
#[path = "util/text.rs"]
mod text;
#[path = "engine/wallpaper.rs"]
mod wallpaper;
#[path = "engine/properties.rs"]
mod properties;
#[path = "gui/web.rs"]
mod web;

use std::{env, thread, time::Duration};

use config::load_settings;
use constants::{DEFAULT_ENGINE_BIN, DEFAULT_OUTPUT};
use instance::acquire_gui_instance_lock;
use model::AppState;
use paths::resolve_scan_root;
use scanner::scan_wallpapers;
use state_ops::{apply_and_save, restore_last_wallpaper};
use wallpaper::prepare_engine_launch;
use web::run_gui_server;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let restore_mode = matches!(args.as_slice(), [arg] if arg == "restore" || arg == "--restore");
    let apply_mode = matches!(args.as_slice(), [arg] if arg != "gui" && arg != "--gui" && arg != "restore" && arg != "--restore");

    let engine_bin =
        env::var("LINUX_WALLPAPERENGINE_BIN").unwrap_or_else(|_| DEFAULT_ENGINE_BIN.to_string());
    let (resolved_engine_bin, engine_workdir, engine_ld_library_path) =
        prepare_engine_launch(&engine_bin);
    let mut settings = load_settings();
    let root = resolve_scan_root(&settings);
    settings.scan_root = Some(root.to_string_lossy().to_string());
    if settings.output.trim().is_empty() {
        settings.output = DEFAULT_OUTPUT.to_string();
    }

    let wallpapers = if apply_mode || restore_mode {
        Vec::new()
    } else {
        scan_wallpapers(&root)
    };
    let mut state = AppState {
        root,
        engine_bin,
        resolved_engine_bin,
        engine_workdir,
        engine_ld_library_path,
        project_overrides: settings.project_overrides.clone(),
        settings,
        wallpapers,
        active_engine_pid: None,
    };

    if restore_mode {
        restore_last_wallpaper(&mut state);
        std::process::exit(0);
    }

    if apply_mode {
        println!("Apply mode: {}", args[0]);
        println!("To restore the saved wallpaper, run: cargo run -- restore");
        println!("For web GUI, run: cargo run");
        let code = apply_and_save(&mut state, &args[0], false);
        std::process::exit(code);
    }

    let _gui_instance_guard = match acquire_gui_instance_lock() {
        Ok(guard) => guard,
        Err(err) => {
            eprintln!("Failed to start GUI server: {err}");
            std::process::exit(1);
        }
    };

    let shared = std::sync::Arc::new(std::sync::Mutex::new(state));

    {
        let mut guard = shared.lock().unwrap();
        restore_last_wallpaper(&mut guard);
    }

    let shared_for_bg = std::sync::Arc::clone(&shared);
    thread::spawn(move || {
        const SCAN_INTERVAL: Duration = Duration::from_secs(10);
        loop {
            thread::sleep(SCAN_INTERVAL);
            let root = {
                let guard = shared_for_bg.lock().unwrap();
                guard.root.clone()
            };
            let wallpapers = scan_wallpapers(&root);
            let mut guard = shared_for_bg.lock().unwrap();
            guard.wallpapers = wallpapers;
        }
    });

    if let Err(err) = run_gui_server(shared) {
        eprintln!("Failed to start GUI server: {err}");
        std::process::exit(1);
    }
}
