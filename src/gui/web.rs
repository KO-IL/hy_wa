use std::{
    collections::HashMap,
    fs::File,
    io::{self, ErrorKind, Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
};

use crate::{
    config::save_settings,
    constants::{DEFAULT_OUTPUT, SERVER_ADDR},
    model::AppState,
    paths::canonicalize_directory,
    scanner::{resolve_allowed_media_path, scan_wallpapers},
    state_ops::apply_and_save,
    text::{json_escape, url_decode},
};

const PAGE_HTML: &str = include_str!("page.html");

pub fn run_gui_server(state: AppState) -> std::io::Result<()> {
    let listener = TcpListener::bind(SERVER_ADDR)?;
    println!("wall-set GUI running at http://{SERVER_ADDR}");
    println!("scan root: {}", state.root.display());
    let shared = Arc::new(Mutex::new(state));

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let shared = Arc::clone(&shared);
                thread::spawn(move || {
                    if let Err(err) = handle_connection(stream, shared) {
                        if !is_client_disconnect(&err) {
                            eprintln!("Request error: {err}");
                        }
                    }
                });
            }
            Err(err) => {
                eprintln!("Accept error: {err}");
            }
        }
    }
    Ok(())
}

fn is_client_disconnect(err: &std::io::Error) -> bool {
    matches!(
        err.kind(),
        ErrorKind::BrokenPipe
            | ErrorKind::ConnectionReset
            | ErrorKind::ConnectionAborted
            | ErrorKind::UnexpectedEof
    )
}

fn handle_connection(mut stream: TcpStream, shared: Arc<Mutex<AppState>>) -> std::io::Result<()> {
    let mut buffer = [0_u8; 32 * 1024];
    let size = stream.read(&mut buffer)?;
    if size == 0 {
        return Ok(());
    }
    let request = String::from_utf8_lossy(&buffer[..size]);
    let mut first_line_parts = request.lines().next().unwrap_or("").split_whitespace();
    let method = first_line_parts.next().unwrap_or("");
    let target = first_line_parts.next().unwrap_or("/");
    if method != "GET" {
        return write_text_response(
            &mut stream,
            "405 Method Not Allowed",
            "Only GET is supported.",
        );
    }

    let (path, query) = split_target(target);
    let params = parse_query(query);

    match path {
        "/" => write_bytes_response(
            &mut stream,
            "200 OK",
            "text/html; charset=utf-8",
            PAGE_HTML.as_bytes(),
        ),
        "/api/list" => {
            let body = {
                let guard = shared
                    .lock()
                    .map_err(|_| std::io::Error::other("state lock poisoned"))?;
                build_state_json(&guard)
            };
            write_text_response(&mut stream, "200 OK", &body)
        }
        "/api/refresh" => {
            let root = {
                let guard = shared
                    .lock()
                    .map_err(|_| std::io::Error::other("state lock poisoned"))?;
                guard.root.clone()
            };
            let wallpapers = scan_wallpapers(&root);
            let body = {
                let mut guard = shared
                    .lock()
                    .map_err(|_| std::io::Error::other("state lock poisoned"))?;
                if guard.root == root {
                    guard.wallpapers = wallpapers;
                }
                build_state_json(&guard)
            };
            write_text_response(&mut stream, "200 OK", &body)
        }
        "/api/set_output" => {
            let Some(output) = params.get("output") else {
                return write_text_response(&mut stream, "400 Bad Request", "Missing output.");
            };
            let mut guard = shared
                .lock()
                .map_err(|_| std::io::Error::other("state lock poisoned"))?;
            guard.settings.output = output.trim().to_string();
            if guard.settings.output.is_empty() {
                guard.settings.output = DEFAULT_OUTPUT.to_string();
            }
            let settings = guard.settings.clone();
            drop(guard);
            let _ = save_settings(&settings);
            write_text_response(
                &mut stream,
                "200 OK",
                &format!("Output saved: {}", settings.output),
            )
        }
        "/api/set_root" => {
            let Some(root) = params.get("root") else {
                return write_text_response(&mut stream, "400 Bad Request", "Missing root.");
            };
            let canonical = match canonicalize_directory(root) {
                Ok(path) => path,
                Err(err) => {
                    return write_text_response(
                        &mut stream,
                        "400 Bad Request",
                        &format!("Invalid root directory: {err}"),
                    );
                }
            };
            let wallpapers = scan_wallpapers(&canonical);
            let mut guard = shared
                .lock()
                .map_err(|_| std::io::Error::other("state lock poisoned"))?;
            guard.root = canonical.clone();
            guard.settings.scan_root = Some(canonical.to_string_lossy().to_string());
            guard.wallpapers = wallpapers;
            let settings = guard.settings.clone();
            drop(guard);
            let _ = save_settings(&settings);
            write_text_response(
                &mut stream,
                "200 OK",
                &format!("Root saved: {}", canonical.display()),
            )
        }
        "/api/apply" => {
            let Some(path_value) = params.get("path") else {
                return write_text_response(&mut stream, "400 Bad Request", "Missing path.");
            };
            let mut guard = shared
                .lock()
                .map_err(|_| std::io::Error::other("state lock poisoned"))?;
            let code = apply_and_save(&mut guard, path_value, true);
            if code == 0 {
                write_text_response(&mut stream, "200 OK", "Wallpaper applied.")
            } else {
                write_text_response(
                    &mut stream,
                    "500 Internal Server Error",
                    "Wallpaper apply failed. Check terminal logs.",
                )
            }
        }
        "/api/file" => {
            let Some(path_value) = params.get("path") else {
                return write_text_response(&mut stream, "400 Bad Request", "Missing path.");
            };
            let guard = shared
                .lock()
                .map_err(|_| std::io::Error::other("state lock poisoned"))?;
            serve_media_file(&mut stream, &guard, path_value)
        }
        _ => write_text_response(&mut stream, "404 Not Found", "Not found."),
    }
}

fn build_state_json(state: &AppState) -> String {
    let mut body = String::new();
    body.push('{');
    body.push_str("\"root\":\"");
    body.push_str(&json_escape(&state.root.to_string_lossy()));
    body.push_str("\",\"output\":\"");
    body.push_str(&json_escape(&state.settings.output));
    body.push_str("\",\"last\":");
    match &state.settings.last_wallpaper {
        Some(last) => {
            body.push('"');
            body.push_str(&json_escape(last));
            body.push('"');
        }
        None => body.push_str("null"),
    }
    body.push_str(",\"items\":[");
    for (index, item) in state.wallpapers.iter().enumerate() {
        if index > 0 {
            body.push(',');
        }
        body.push('{');
        body.push_str("\"path\":\"");
        body.push_str(&json_escape(&item.path));
        body.push_str("\",\"name\":\"");
        body.push_str(&json_escape(&item.name));
        body.push_str("\",\"kind\":\"");
        body.push_str(item.kind.as_str());
        body.push_str("\",\"thumb\":");
        if let Some(thumb) = &item.thumb {
            body.push('"');
            body.push_str(&json_escape(thumb));
            body.push('"');
        } else {
            body.push_str("null");
        }
        body.push_str("}");
    }
    body.push_str("]}");
    body
}

fn split_target(target: &str) -> (&str, &str) {
    match target.split_once('?') {
        Some((path, query)) => (path, query),
        None => (target, ""),
    }
}

fn parse_query(query: &str) -> HashMap<String, String> {
    let mut params = HashMap::new();
    for pair in query.split('&').filter(|segment| !segment.is_empty()) {
        let (raw_key, raw_value) = match pair.split_once('=') {
            Some((key, value)) => (key, value),
            None => (pair, ""),
        };
        let key = url_decode(raw_key);
        let value = url_decode(raw_value);
        params.insert(key, value);
    }
    params
}

fn serve_media_file(
    stream: &mut TcpStream,
    state: &AppState,
    raw_path: &str,
) -> std::io::Result<()> {
    let path = PathBuf::from(raw_path);
    let Some(path) = resolve_allowed_media_path(&state.root, &path) else {
        return write_text_response(stream, "403 Forbidden", "Path outside scan root.");
    };

    let mut file = match File::open(&path) {
        Ok(file) => file,
        Err(_) => return write_text_response(stream, "404 Not Found", "File not found."),
    };
    let metadata = match file.metadata() {
        Ok(metadata) if metadata.is_file() => metadata,
        Ok(_) => return write_text_response(stream, "404 Not Found", "File not found."),
        Err(err) => return Err(err),
    };
    let content_type = mime_from_path(&path);
    write_stream_response(stream, "200 OK", content_type, metadata.len(), &mut file)
}

fn mime_from_path(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "bmp" => "image/bmp",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "mp4" => "video/mp4",
        "mkv" => "video/x-matroska",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        "avi" => "video/x-msvideo",
        "m4v" => "video/x-m4v",
        _ => "application/octet-stream",
    }
}

fn write_text_response(stream: &mut TcpStream, status: &str, body: &str) -> std::io::Result<()> {
    write_bytes_response(stream, status, "text/plain; charset=utf-8", body.as_bytes())
}

fn write_bytes_response(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &[u8],
) -> std::io::Result<()> {
    let headers = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(headers.as_bytes())?;
    stream.write_all(body)?;
    Ok(())
}

fn write_stream_response(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    content_length: u64,
    body: &mut File,
) -> std::io::Result<()> {
    let headers = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {content_length}\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n"
    );
    stream.write_all(headers.as_bytes())?;
    io::copy(body, stream)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_query;

    #[test]
    fn parse_query_decodes_percent_and_plus() {
        let params = parse_query("path=%2Ftmp%2Fdemo+wallpaper.png&output=DP-3");
        assert_eq!(
            params.get("path").map(String::as_str),
            Some("/tmp/demo wallpaper.png")
        );
        assert_eq!(params.get("output").map(String::as_str), Some("DP-3"));
    }
}
