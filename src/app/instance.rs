use std::{
    env,
    fs::{self, OpenOptions},
    io::{ErrorKind, Write},
    path::PathBuf,
    process::{self, Command, Stdio},
};

pub struct GuiInstanceGuard {
    lock_path: PathBuf,
}

pub fn acquire_gui_instance_lock() -> std::io::Result<GuiInstanceGuard> {
    let lock_path = gui_lock_path();

    loop {
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
        {
            Ok(mut file) => {
                let _ = writeln!(file, "{}", process::id());
                return Ok(GuiInstanceGuard { lock_path });
            }
            Err(err) if err.kind() == ErrorKind::AlreadyExists => {
                let is_stale = match read_lock_pid(&lock_path) {
                    Some(pid) => !process_exists(pid),
                    None => true,
                };
                if is_stale {
                    match fs::remove_file(&lock_path) {
                        Ok(_) => continue,
                        Err(remove_err) if remove_err.kind() == ErrorKind::NotFound => continue,
                        Err(remove_err) => return Err(remove_err),
                    }
                }

                let message = match read_lock_pid(&lock_path) {
                    Some(pid) => format!("wall-set GUI is already running (pid {pid})."),
                    None => "wall-set GUI is already running.".to_string(),
                };
                return Err(std::io::Error::new(ErrorKind::AddrInUse, message));
            }
            Err(err) => return Err(err),
        }
    }
}

impl Drop for GuiInstanceGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.lock_path);
    }
}

fn gui_lock_path() -> PathBuf {
    if let Ok(runtime_dir) = env::var("XDG_RUNTIME_DIR") {
        if !runtime_dir.trim().is_empty() {
            return PathBuf::from(runtime_dir).join("wall-set-gui.lock");
        }
    }
    PathBuf::from("/tmp/wall-set-gui.lock")
}

fn read_lock_pid(path: &PathBuf) -> Option<u32> {
    let content = fs::read_to_string(path).ok()?;
    content.trim().parse::<u32>().ok()
}

fn process_exists(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}
