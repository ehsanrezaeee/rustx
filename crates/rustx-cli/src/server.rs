use std::{
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    time::Duration,
};
fn resolve(root: &Path, url: &str) -> Option<PathBuf> {
    let path = url.split('?').next()?;
    if !path.starts_with('/') || path.contains('%') || path.contains('\\') {
        return None;
    }
    let relative = path.trim_start_matches('/');
    let file = root
        .join(if relative.is_empty() {
            "index.html"
        } else {
            relative
        })
        .canonicalize()
        .ok()?;
    (file.starts_with(root) && file.is_file()).then_some(file)
}
fn respond(mut stream: TcpStream, root: &Path) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let mut data = [0; 8192];
    let Ok(n) = stream.read(&mut data) else {
        return;
    };
    let request = String::from_utf8_lossy(&data[..n]);
    let mut words = request.lines().next().unwrap_or("").split_whitespace();
    let method = words.next().unwrap_or("");
    let url = words.next().unwrap_or("");
    let (status, mime, body) = if !matches!(method, "GET" | "HEAD") {
        (
            "405 Method Not Allowed",
            "text/plain",
            b"GET or HEAD only".to_vec(),
        )
    } else if url.starts_with("/api/users") {
        if url.contains("delay=1") {
            std::thread::sleep(Duration::from_millis(1200));
        }
        (
            "200 OK",
            "application/json",
            include_bytes!("../../../fixtures/users.json").to_vec(),
        )
    } else if url == "/api/error" {
        (
            "503 Service Unavailable",
            "application/json",
            br#"{"error":"Intentional test failure"}"#.to_vec(),
        )
    } else if let Some(file) = resolve(root, url) {
        let mime = match file.extension().and_then(|s| s.to_str()).unwrap_or("") {
            "html" => "text/html; charset=utf-8",
            "js" => "text/javascript",
            "wasm" => "application/wasm",
            "json" => "application/json",
            _ => "application/octet-stream",
        };
        match fs::read(file) {
            Ok(data) => ("200 OK", mime, data),
            Err(_) => ("500 Internal Server Error", "text/plain", vec![]),
        }
    } else {
        ("404 Not Found", "text/plain", b"Not found".to_vec())
    };
    let header=format!("HTTP/1.1 {status}\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nCache-Control: no-cache\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n",body.len());
    let _ = stream.write_all(header.as_bytes());
    if method != "HEAD" {
        let _ = stream.write_all(&body);
    }
}
pub fn serve(root: &Path, port: &str) -> Result<(), String> {
    let root = root
        .canonicalize()
        .map_err(|e| format!("Build the bundle first: {e}"))?;
    let port: u16 = port.parse().map_err(|_| "Invalid port".to_owned())?;
    let listener = TcpListener::bind(("127.0.0.1", port)).map_err(|e| e.to_string())?;
    println!(
        "RustX serving {} at http://127.0.0.1:{port}",
        root.display()
    );
    for stream in listener.incoming() {
        let root = root.clone();
        if let Ok(stream) = stream {
            std::thread::spawn(move || respond(stream, &root));
        }
    }
    Ok(())
}
