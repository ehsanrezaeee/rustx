use std::{
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    time::Duration,
};
pub struct Request {
    pub method: String,
    pub path: String,
    pub headers: std::collections::BTreeMap<String, String>,
    pub body: Vec<u8>,
}
pub fn read_request(stream: &mut TcpStream, limit: usize) -> Result<Request, String> {
    stream.set_nonblocking(false).map_err(|e| e.to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;
    let mut bytes = vec![];
    let end = loop {
        let mut chunk = [0; 4096];
        let n = stream.read(&mut chunk).map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("Incomplete request".into());
        }
        bytes.extend_from_slice(&chunk[..n]);
        if let Some(n) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
            if n > 16384 {
                return Err("Header limit".into());
            }
            break n + 4;
        }
        if bytes.len() > 16384 {
            return Err("Header limit".into());
        }
    };
    let head = std::str::from_utf8(&bytes[..end]).map_err(|_| "Invalid headers")?;
    let mut lines = head.lines();
    let parts = lines
        .next()
        .ok_or("Missing request line")?
        .split_whitespace()
        .collect::<Vec<_>>();
    if parts.len() != 3 || parts[2] != "HTTP/1.1" {
        return Err("Invalid HTTP request".into());
    }
    let method = parts[0].to_owned();
    let path = parts[1].to_owned();
    let mut headers = std::collections::BTreeMap::new();
    for line in lines.filter(|s| !s.is_empty()) {
        let (k, v) = line.split_once(':').ok_or("Invalid header")?;
        let key = k.to_ascii_lowercase();
        if headers.insert(key, v.trim().to_owned()).is_some() {
            return Err("Duplicate headers are not accepted".into());
        }
    }
    if headers.contains_key("transfer-encoding") {
        return Err("Chunked requests are not accepted".into());
    }
    let len = headers
        .get("content-length")
        .map(|s| s.parse::<usize>().map_err(|_| "Invalid length"))
        .transpose()?
        .unwrap_or(0);
    if len > limit {
        return Err("Request size limit".into());
    }
    while bytes.len() - end < len {
        let mut chunk = [0; 4096];
        let n = stream.read(&mut chunk).map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("Incomplete body".into());
        }
        bytes.extend_from_slice(&chunk[..n]);
    }
    Ok(Request {
        method,
        path,
        headers,
        body: bytes[end..end + len].to_vec(),
    })
}
fn resolve(root: &Path, url: &str) -> Option<PathBuf> {
    let path = url.split('?').next()?;
    if !path.starts_with('/')
        || path.contains('%')
        || path.contains('\\')
        || path.split('/').any(|p| p == "..")
    {
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
fn websocket(stream: TcpStream) {
    use tungstenite::{protocol::WebSocketConfig, Message};
    let config = WebSocketConfig::default()
        .read_buffer_size(8192)
        .write_buffer_size(0)
        .max_message_size(Some(65_536))
        .max_frame_size(Some(65_536))
        .max_write_buffer_size(262_144);
    let Ok(mut socket) = tungstenite::accept_with_config(stream, Some(config)) else {
        return;
    };
    loop {
        match socket.read() {
            Ok(message @ (Message::Text(_) | Message::Binary(_))) => {
                if socket.send(message).is_err() {
                    break;
                }
            }
            Ok(Message::Close(_)) => {
                let _ = socket.flush();
                break;
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }
}
fn respond(mut stream: TcpStream, root: &Path) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(10)));
    let mut peek = [0; 8192];
    let Ok(n) = stream.peek(&mut peek) else {
        return;
    };
    let first = String::from_utf8_lossy(&peek[..n]);
    if first
        .lines()
        .next()
        .is_some_and(|s| s.split_whitespace().nth(1) == Some("/api/socket"))
    {
        websocket(stream);
        return;
    }
    let mut data = vec![];
    let header_end = loop {
        let mut chunk = [0; 8192];
        let Ok(n) = stream.read(&mut chunk) else {
            return;
        };
        if n == 0 {
            return;
        }
        data.extend_from_slice(&chunk[..n]);
        if let Some(i) = data.windows(4).position(|s| s == b"\r\n\r\n") {
            break i + 4;
        }
        if data.len() > 32_768 {
            return;
        }
    };
    let request = String::from_utf8_lossy(&data[..header_end]).into_owned();
    let mut words = request.lines().next().unwrap_or("").split_whitespace();
    let method = words.next().unwrap_or("");
    let url = words.next().unwrap_or("");
    let headers = request
        .lines()
        .skip(1)
        .filter_map(|s| s.split_once(':'))
        .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_owned()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let length = headers
        .get("content-length")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);
    if length > 1_048_576 {
        return;
    }
    while data.len() - header_end < length {
        let mut chunk = [0; 8192];
        let Ok(n) = stream.read(&mut chunk) else {
            return;
        };
        if n == 0 {
            return;
        }
        data.extend_from_slice(&chunk[..n]);
    }
    let payload = &data[header_end..header_end + length];
    if url == "/api/stream" && method == "GET" {
        let _=stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/plain;charset=UTF-8\r\nTransfer-Encoding: chunked\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n");
        for chunk in [
            "RustX streamed ",
            "data with ",
            "bounded pull backpressure.\n",
        ] {
            if write!(stream, "{:x}\r\n{chunk}\r\n", chunk.len()).is_err() {
                return;
            }
            let _ = stream.flush();
            std::thread::sleep(Duration::from_millis(25));
        }
        let _ = stream.write_all(b"0\r\n\r\n");
        return;
    }
    let (status, mime, mut body) = if url == "/api/echo"
        && matches!(
            method,
            "GET" | "HEAD" | "POST" | "PUT" | "PATCH" | "DELETE" | "OPTIONS"
        ) {
        let body = serde_json::to_vec(&serde_json::json!({
            "method": method,
            "body": String::from_utf8_lossy(payload),
            "bytes": payload.len(),
            "content_type": headers.get("content-type"),
        }))
        .unwrap_or_default();
        ("200 OK", "application/json", body)
    } else if !matches!(method, "GET" | "HEAD") {
        (
            "405 Method Not Allowed",
            "text/plain",
            b"Unsupported fixture method".to_vec(),
        )
    } else if url == "/api/empty" {
        ("200 OK", "application/json", b"[]".to_vec())
    } else if url == "/api/timeout" {
        std::thread::sleep(Duration::from_millis(1200));
        (
            "200 OK",
            "application/json",
            include_bytes!(concat!(env!("OUT_DIR"), "/users.json")).to_vec(),
        )
    } else if url.starts_with("/api/users") {
        if url.contains("delay=1") {
            std::thread::sleep(Duration::from_millis(1200));
        }
        (
            "200 OK",
            "application/json",
            include_bytes!(concat!(env!("OUT_DIR"), "/users.json")).to_vec(),
        )
    } else if url == "/api/error" {
        (
            "503 Service Unavailable",
            "application/json",
            br#"{"error":"Intentional test failure"}"#.to_vec(),
        )
    } else if url == "/api/malformed" {
        ("200 OK", "application/json", b"{broken-json".to_vec())
    } else if url == "/api/large" {
        ("200 OK", "application/octet-stream", vec![b'x'; 1_048_577])
    } else if let Some(file) = resolve(root, url) {
        let mime = match file.extension().and_then(|s| s.to_str()).unwrap_or("") {
            "html" => "text/html; charset=utf-8",
            "js" | "mjs" => "text/javascript",
            "css" => "text/css",
            "svg" => "image/svg+xml",
            "wasm" => "application/wasm",
            "json" => "application/json",
            _ => "application/octet-stream",
        };
        match fs::read(file) {
            Ok(data) => ("200 OK", mime, data),
            Err(_) => ("500 Internal Server Error", "text/plain", vec![]),
        }
    } else if !url.starts_with("/api/")
        && !url.contains(['%', '\\'])
        && !url.split('/').any(|p| p == "..")
        && Path::new(url.split('?').next().unwrap_or(""))
            .extension()
            .is_none()
    {
        match fs::read(root.join("index.html")) {
            Ok(data) => ("200 OK", "text/html; charset=utf-8", data),
            Err(_) => ("404 Not Found", "text/plain", b"Not found".to_vec()),
        }
    } else {
        ("404 Not Found", "text/plain", b"Not found".to_vec())
    };
    let quality = |name: &str| {
        let mut exact = None;
        let mut wildcard = 0_f32;
        for entry in headers
            .get("accept-encoding")
            .into_iter()
            .flat_map(|value| value.split(','))
        {
            let mut parts = entry.trim().split(';');
            let encoding = parts.next().unwrap_or("").trim();
            let mut weight = 1_f32;
            for parameter in parts {
                if let Some(value) = parameter.trim().strip_prefix("q=") {
                    weight = value
                        .parse::<f32>()
                        .ok()
                        .filter(|q| q.is_finite() && (0_f32..=1.).contains(q))
                        .unwrap_or(0.);
                }
            }
            if encoding.eq_ignore_ascii_case(name) {
                exact = Some(weight);
            }
            if encoding == "*" {
                wildcard = weight;
            }
        }
        exact.unwrap_or(wildcard)
    };
    let mut candidates = [
        (quality("br"), "br", "Content-Encoding: br\r\n"),
        (quality("gzip"), "gz", "Content-Encoding: gzip\r\n"),
    ];
    candidates.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut encoding = "";
    if status == "200 OK" && !url.starts_with("/api/") {
        for (quality, suffix, header) in candidates {
            if quality <= 0. {
                continue;
            }
            if let Some(file) = resolve(
                root,
                &format!("{}.{suffix}", url.split('?').next().unwrap_or("")),
            ) {
                if let Ok(bytes) = fs::read(file) {
                    body = bytes;
                    encoding = header;
                    break;
                }
            }
        }
    }
    let digest = crate::tooling::hash(&body);
    let immutable = status == "200 OK"
        && !mime.starts_with("text/html")
        && url
            .split('.')
            .any(|part| part.len() == 12 && part.bytes().all(|b| b.is_ascii_hexdigit()));
    let cache = if immutable {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    };
    let mut csp = concat!(
        "default-src 'none'; script-src 'self' 'wasm-unsafe-eval'; ",
        "style-src 'self' 'unsafe-inline'; img-src 'self' data:; ",
        "connect-src 'self' https://jsonplaceholder.typicode.com; ",
        "worker-src 'self' blob:; base-uri 'none'; object-src 'none'; ",
        "frame-ancestors 'none'"
    )
    .to_owned();
    if let Ok(config) = fs::read(root.join("runtime-config.json")) {
        if let Ok(config) = serde_json::from_slice::<serde_json::Value>(&config) {
            if let Some(origin) = config["compiler"]
                .as_str()
                .filter(|s| s.starts_with("http://") || s.starts_with("https://"))
                .filter(|s| {
                    s.bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b".:-/".contains(&b))
                })
            {
                csp = csp.replace(
                    "https://jsonplaceholder.typicode.com",
                    &format!("https://jsonplaceholder.typicode.com {origin}"),
                );
                csp.push_str(&format!("; frame-src {origin}"));
            }
        }
    }
    let not_modified = headers
        .get("if-none-match")
        .is_some_and(|value| value == &format!("\"{digest}\""));
    let status = if not_modified {
        "304 Not Modified"
    } else {
        status
    };
    let header=format!("HTTP/1.1 {status}\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nCache-Control: {cache}\r\nETag: \"{digest}\"\r\nVary: Accept-Encoding\r\n{encoding}Content-Security-Policy: {csp}\r\nX-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\nConnection: close\r\n\r\n",if not_modified{0}else{body.len()});
    let _ = stream.write_all(header.as_bytes());
    if method != "HEAD" && !not_modified {
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
