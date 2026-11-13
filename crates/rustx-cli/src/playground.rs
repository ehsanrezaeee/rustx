//! Isolated compilation. Visitor code is compiled to Wasm, never executed natively.
use crate::tooling::{self, hash, write};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
static STOP: AtomicBool = AtomicBool::new(false);
#[cfg(unix)]
extern "C" fn stop(_: i32) {
    STOP.store(true, Ordering::Relaxed);
}
const MAX_SOURCE: usize = 32768;
const MAX_JOBS: usize = 8;
const MAX_DIAGNOSTICS: u64 = 65536;
const TTL: Duration = Duration::from_secs(300);
#[derive(Clone)]
struct Cache {
    root: PathBuf,
    sysroot: PathBuf,
    rustc: PathBuf,
    bindings: PathBuf,
    externs: Vec<(String, String)>,
}
struct Job {
    created: Instant,
    path: PathBuf,
    state: Value,
    cancel: Arc<AtomicBool>,
}
struct State {
    jobs: Mutex<BTreeMap<String, Job>>,
    active: AtomicUsize,
    cache: Cache,
    root: PathBuf,
    origin: String,
    app_origin: String,
}
fn random() -> Result<String, String> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).map_err(|e| e.to_string())?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
fn prepare(framework: &Path) -> Result<Cache, String> {
    if !cfg!(target_os = "macos") && !cfg!(target_os = "linux") {
        return Err(
            "No verified compiler isolation provider for this OS; compilation is disabled".into(),
        );
    }
    let version = tooling::output("rustc", &["--version"], framework)?;
    if !version.starts_with("rustc 1.90.0 ") {
        return Err("Compiler toolchain must be Rust 1.90.0".into());
    }
    let bindings_version = tooling::output("wasm-bindgen", &["--version"], framework)?;
    if bindings_version.trim() != "wasm-bindgen 0.2.100" {
        return Err("Compiler bindings must be wasm-bindgen 0.2.100".into());
    }
    let sysroot =
        PathBuf::from(tooling::output("rustc", &["--print", "sysroot"], framework)?.trim())
            .canonicalize()
            .map_err(|e| e.to_string())?;
    let rustc = sysroot.join("bin/rustc");
    let bindings = std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .map(|p| p.join("wasm-bindgen"))
        .find(|p| p.is_file())
        .ok_or("wasm-bindgen not found")?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let artifacts = tooling::output(
        "cargo",
        &[
            "build",
            "-p",
            "rustx",
            "-p",
            "rustx-web",
            "--lib",
            "--target",
            "wasm32-unknown-unknown",
            "--release",
            "--locked",
            "--message-format=json",
        ],
        framework,
    )?;
    let mut libraries = Vec::new();
    let mut externs = vec![];
    for line in artifacts.lines() {
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if v["reason"] != "compiler-artifact" {
            continue;
        }
        if let Some(files) = v["filenames"].as_array() {
            for file in files {
                let path = PathBuf::from(file.as_str().unwrap_or(""));
                if matches!(
                    path.extension().and_then(|s| s.to_str()),
                    Some("rlib" | "rmeta" | "dylib" | "so" | "dll")
                ) {
                    if [
                        "rustx",
                        "rustx_core",
                        "rustx_web",
                        "rustx_engine",
                        "serde_json",
                        "wasm_bindgen",
                        "wasm_bindgen_futures",
                    ]
                    .contains(&v["target"]["name"].as_str().unwrap_or(""))
                        && path.extension().is_some_and(|e| e == "rlib")
                    {
                        externs.push((
                            v["target"]["name"].as_str().unwrap().to_owned(),
                            path.file_name().unwrap().to_string_lossy().into_owned(),
                        ));
                    }
                    libraries.push(path);
                }
            }
        }
    }
    if !externs.iter().any(|(name, _)| name == "rustx_web") {
        return Err("Trusted Wasm cache is incomplete".into());
    }
    let mut identity = String::new();
    for path in &libraries {
        identity.push_str(&hash(fs::read(path).map_err(|e| e.to_string())?));
    }
    let root = framework
        .join(".rustx/playground/cache")
        .join(&hash(identity)[..20]);
    fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    for file in libraries {
        let destination = root.join(file.file_name().unwrap());
        if !destination.exists() {
            fs::copy(&file, &destination).map_err(|e| e.to_string())?;
        }
    }
    let cache = Cache {
        root,
        sysroot,
        rustc,
        bindings,
        externs,
    };
    let probe = framework.join(".rustx/playground/probe");
    fs::create_dir_all(&probe).map_err(|e| e.to_string())?;
    let mut command = isolated(&cache, &probe, &cache.rustc)?;
    command.arg("--version");
    let result = command
        .output()
        .map_err(|e| format!("Isolation unavailable: {e}"))?;
    if !result.status.success() {
        return Err(format!(
            "Isolation probe failed; no unsandboxed fallback (status {:?}): {}",
            result.status,
            String::from_utf8_lossy(&result.stderr)
        ));
    }
    write(&probe.join("valid.rs"), "pub fn probe() -> usize { 42 }")?;
    let mut valid = isolated(&cache, &probe, &cache.rustc)?;
    valid
        .args([
            "--crate-type=lib",
            "--emit=metadata",
            "--target=wasm32-unknown-unknown",
        ])
        .arg(probe.join("valid.rs"))
        .arg("-o")
        .arg(probe.join("valid.rmeta"));
    let checked = valid.output().map_err(|e| e.to_string())?;
    if !checked.status.success() {
        return Err(format!(
            "Isolated compiler self-test failed: {}",
            String::from_utf8_lossy(&checked.stderr)
        ));
    }
    // A successful version probe alone is insufficient: verify a denied read with real rustc.
    write(
        &probe.join("denied.rs"),
        "const _: &str = include_str!(\"/etc/passwd\"); fn main() {}",
    )?;
    let mut denied = isolated(&cache, &probe, &cache.rustc)?;
    denied
        .args(["--crate-type=lib", "--emit=metadata"])
        .arg(probe.join("denied.rs"))
        .arg("-o")
        .arg(probe.join("denied.rmeta"));
    if denied.output().map_err(|e| e.to_string())?.status.success() {
        return Err("Isolation denied-read self-test failed; compilation disabled".into());
    }
    Ok(cache)
}
fn isolated(cache: &Cache, job: &Path, program: &Path) -> Result<Command, String> {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    let mut c: Command;
    #[cfg(target_os = "macos")]
    {
        let profile = format!(
            concat!(
                "(version 1)(deny default)",
                "(allow process-fork)(allow process-info*)(allow sysctl-read)",
                "(allow mach-lookup)(allow file-read-metadata)",
                "(allow file-read* (literal \"/\") (subpath \"/System\") ",
                "(subpath \"/usr/lib\") (subpath \"/Library/Apple\") ",
                "(subpath \"/private/var/db/dyld\") (literal \"/dev/urandom\") ",
                "(literal \"/dev/random\") (literal \"/dev/null\") ",
                "(subpath {:?}) (subpath {:?}) (subpath {:?}) (literal {:?}))",
                "(allow file-write* (subpath {:?}) (literal \"/dev/null\"))",
                "(allow process-exec (literal \"/bin/sh\") (subpath {:?}) (literal {:?}))"
            ),
            cache.sysroot.to_str().unwrap(),
            cache.root.to_str().unwrap(),
            job.to_str().unwrap(),
            cache.bindings.to_str().unwrap(),
            job.to_str().unwrap(),
            cache.sysroot.to_str().unwrap(),
            cache.bindings.to_str().unwrap(),
        );
        write(&job.join("sandbox.sb"), profile)?;
        c = Command::new("/usr/bin/sandbox-exec");
        c.arg("-f").arg(job.join("sandbox.sb"));
    }
    #[cfg(target_os = "linux")]
    {
        c = Command::new("bwrap");
        c.args([
            "--unshare-all",
            "--die-with-parent",
            "--new-session",
            "--clearenv",
            "--ro-bind",
            "/usr",
            "/usr",
            "--ro-bind-try",
            "/lib",
            "/lib",
            "--ro-bind-try",
            "/lib64",
            "/lib64",
            "--ro-bind-try",
            "/bin",
            "/bin",
            "--proc",
            "/proc",
            "--dev",
            "/dev",
        ])
        .arg("--ro-bind")
        .arg(&cache.sysroot)
        .arg(&cache.sysroot)
        .arg("--ro-bind")
        .arg(&cache.root)
        .arg(&cache.root)
        .arg("--ro-bind")
        .arg(&cache.bindings)
        .arg(&cache.bindings)
        .arg("--bind")
        .arg(job)
        .arg(job);
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = (cache, job, program);
        return Err("Compiler isolation unavailable".into());
    }
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        c.arg(program);
        c.env_clear()
            .env("PATH", cache.sysroot.join("bin"))
            .env("TMPDIR", job)
            .env("RUST_BACKTRACE", "0")
            .env("CARGO_MANIFEST_DIR", job)
            .env("CARGO_PKG_NAME", "rustx_preview")
            .env("CARGO_PKG_VERSION", "0.1.0")
            .current_dir(job)
            .stdin(Stdio::null());
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            unsafe {
                c.pre_exec(|| {
                    #[cfg(target_os = "linux")]
                    {
                        let bounds = libc::rlimit {
                            rlim_cur: 1_610_612_736,
                            rlim_max: 1_610_612_736,
                        };
                        if libc::setrlimit(libc::RLIMIT_AS, &bounds) != 0 {
                            return Err(std::io::Error::last_os_error());
                        }
                    }
                    for (resource, limit) in [
                        (libc::RLIMIT_CPU, 60),
                        (libc::RLIMIT_FSIZE, 33_554_432),
                        (libc::RLIMIT_NOFILE, 128),
                    ] {
                        let bounds = libc::rlimit {
                            rlim_cur: limit,
                            rlim_max: limit,
                        };
                        if libc::setrlimit(resource, &bounds) != 0 {
                            return Err(std::io::Error::last_os_error());
                        }
                    }
                    Ok(())
                });
            }
            c.process_group(0);
        }
        Ok(c)
    }
}
fn memory_exceeded(pid: u32) -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    {
        let mut pids = vec![pid as i32];
        let mut total = 0u64;
        let mut index = 0;
        while index < pids.len() && index < 64 {
            let current = pids[index];
            index += 1;
            let mut info = std::mem::MaybeUninit::<libc::rusage_info_v2>::zeroed();
            // SAFETY: libproc receives a correctly sized writable v2 record and bounded PID array.
            let result = unsafe {
                libc::proc_pid_rusage(current, libc::RUSAGE_INFO_V2, info.as_mut_ptr().cast())
            };
            if result == 0 {
                total += unsafe { info.assume_init() }.ri_resident_size;
            } else if current == pid as i32
                && std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH)
            {
                return Err("Memory accounting unavailable; compiler stopped".into());
            }
            let mut children = [0i32; 32];
            let count = unsafe {
                libc::proc_listchildpids(
                    current,
                    children.as_mut_ptr().cast(),
                    std::mem::size_of_val(&children) as i32,
                )
            };
            if count > 0 {
                for child in children.into_iter() {
                    if child > 0 && !pids.contains(&child) {
                        pids.push(child);
                    }
                }
            }
        }
        Ok(total > 1_610_612_736)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = pid;
        Ok(false)
    }
}
fn bounded_process(
    mut command: Command,
    job: &Path,
    cancel: &AtomicBool,
    started: Instant,
) -> Result<(bool, String), String> {
    let diagnostics = job.join("diagnostics.jsonl");
    let file = fs::File::create(&diagnostics).map_err(|e| e.to_string())?;
    command
        .stderr(Stdio::from(file.try_clone().map_err(|e| e.to_string())?))
        .stdout(Stdio::from(file));
    let mut child = command.spawn().map_err(|e| e.to_string())?;
    loop {
        if cancel.load(Ordering::Acquire)
            || started.elapsed() > Duration::from_secs(30)
            || fs::metadata(&diagnostics).is_ok_and(|m| m.len() > MAX_DIAGNOSTICS)
            || memory_exceeded(child.id()).unwrap_or(true)
        {
            #[cfg(unix)]
            unsafe {
                libc::kill(-(child.id() as i32), libc::SIGKILL);
            } // SAFETY: child owns a new process group, so this cannot target the server/user shell.
            let _ = child.kill();
            let _ = child.wait();
            return Err(if cancel.load(Ordering::Acquire) {
                "cancelled"
            } else {
                "limit_exceeded"
            }
            .into());
        }
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            let mut bytes = vec![];
            fs::File::open(&diagnostics)
                .map_err(|e| e.to_string())?
                .take(MAX_DIAGNOSTICS)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            let text = String::from_utf8_lossy(&bytes).into_owned();
            return Ok((
                status.success(),
                if !status.success() && text.is_empty() {
                    format!("Compiler process ended: {status}")
                } else {
                    text
                },
            ));
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}
fn compile(state: &State, path: &Path, source: &str, cancel: &AtomicBool) -> Result<Value, String> {
    let deadline = Instant::now();
    write(&path.join("main.rustx"), source)?;
    write(
        &path.join("wrapper.rs"),
        include_str!("../templates/preview-wrapper.rs"),
    )?;
    let mut rustc = isolated(&state.cache, path, &state.cache.rustc)?;
    rustc
        .args([
            "--edition=2021",
            "--crate-name",
            "rustx_preview",
            "--crate-type",
            "cdylib",
            "--target",
            "wasm32-unknown-unknown",
            "--error-format=json",
            "-C",
            "opt-level=s",
            "-C",
            "debuginfo=0",
            "-C",
            "lto=thin",
            "-C",
            "codegen-units=1",
            "-C",
            "link-arg=--max-memory=268435456",
        ])
        .arg("--sysroot")
        .arg(&state.cache.sysroot)
        .arg("-L")
        .arg(format!("dependency={}", state.cache.root.display()))
        .arg("--remap-path-prefix")
        .arg(format!("{}=/playground", path.display()))
        .arg("--remap-path-prefix")
        .arg(format!("{}=/dependencies", state.cache.root.display()))
        .arg("--remap-path-prefix")
        .arg(format!("{}=/toolchain", state.cache.sysroot.display()));
    for (name, file) in &state.cache.externs {
        rustc
            .arg("--extern")
            .arg(format!("{name}={}", state.cache.root.join(file).display()));
    }
    rustc
        .arg(path.join("wrapper.rs"))
        .arg("-o")
        .arg(path.join("rustx_preview.wasm"));
    let (success, errors) = bounded_process(rustc, path, cancel, deadline)?;
    if !success {
        fn sanitize(value: &mut Value) {
            match value {
                Value::Object(map) => {
                    map.remove("rendered");
                    map.remove("expansion");
                    if let Some(file) = map.get_mut("file_name") {
                        *file = json!(if file.as_str().unwrap_or("").ends_with("main.rustx") {
                            "main.rustx"
                        } else {
                            "framework"
                        });
                    }
                    for child in map.values_mut() {
                        sanitize(child);
                    }
                }
                Value::Array(items) => {
                    for child in items {
                        sanitize(child);
                    }
                }
                _ => {}
            }
        }
        let mut diagnostics = vec![];
        for line in errors.lines() {
            if let Ok(mut item) = serde_json::from_str::<Value>(line) {
                item.as_object_mut().map(|m| m.remove("rendered"));
                if let Some(spans) = item["spans"].as_array_mut() {
                    for span in spans {
                        span["file_name"] = json!(if span["file_name"]
                            .as_str()
                            .unwrap_or("")
                            .ends_with("main.rustx")
                        {
                            "main.rustx"
                        } else {
                            "framework"
                        });
                    }
                }
                sanitize(&mut item);
                diagnostics.push(item);
            }
        }
        return Ok(
            json!({"status":"compile_error","diagnostics":diagnostics,"message":if errors.is_empty(){"Compiler exited without diagnostics"}else{"Rust compilation failed"}}),
        );
    }
    fs::create_dir_all(path.join("pkg")).map_err(|e| e.to_string())?;
    let mut binding = isolated(&state.cache, path, &state.cache.bindings)?;
    binding
        .arg(path.join("rustx_preview.wasm"))
        .args(["--target", "web", "--out-dir"])
        .arg(path.join("pkg"));
    let (ok, message) = bounded_process(binding, path, cancel, deadline)?;
    if !ok {
        return Err(format!(
            "binding_failed: {}",
            message.chars().take(1000).collect::<String>()
        ));
    }
    let js = fs::read_to_string(path.join("pkg/rustx_preview.js")).map_err(|e| e.to_string())?;
    if path.join("pkg/snippets").exists()
        || js
            .lines()
            .any(|line| line.trim_start().starts_with("import "))
    {
        return Err(
            "Visitor JavaScript/module imports are unavailable; use the scoped RustX APIs".into(),
        );
    }
    let _ = fs::remove_file(path.join("rustx_preview.wasm"));
    let mut bytes = 0u64;
    for file in fs::read_dir(path.join("pkg")).map_err(|e| e.to_string())? {
        let file = file.map_err(|e| e.to_string())?;
        if !file.file_type().map_err(|e| e.to_string())?.is_file() {
            return Err("Unexpected compiler artifact".into());
        }
        bytes += file.metadata().map_err(|e| e.to_string())?.len();
    }
    if bytes > 32 * 1024 * 1024 {
        return Err("Output limit exceeded".into());
    }
    Ok(
        json!({"status":"ready","wasm_sha256":hash(fs::read(path.join("pkg/rustx_preview_bg.wasm")).map_err(|e|e.to_string())?),"bytes":bytes,"diagnostics":[]}),
    )
}
fn response(
    stream: &mut TcpStream,
    status: &str,
    mime: &str,
    body: &[u8],
    origin: Option<&str>,
    extra: &str,
) {
    let cors = origin.map_or(String::new(), |o| {
        format!("Access-Control-Allow-Origin: {o}\r\nVary: Origin\r\n")
    });
    let head=format!("HTTP/1.1 {status}\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\n{cors}{extra}Connection: close\r\n\r\n",body.len());
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body);
}
fn json_response(stream: &mut TcpStream, status: &str, value: Value, origin: Option<&str>) {
    response(
        stream,
        status,
        "application/json",
        &serde_json::to_vec(&value).unwrap(),
        origin,
        "",
    );
}
fn handle(mut stream: TcpStream, state: Arc<State>) {
    let request = match crate::server::read_request(&mut stream, MAX_SOURCE + 4096) {
        Ok(r) => r,
        Err(e) => {
            json_response(&mut stream, "400 Bad Request", json!({"error":e}), None);
            return;
        }
    };
    let request_origin = request.headers.get("origin").map(String::as_str);
    let trusted = request_origin.is_none() || request_origin == Some(state.app_origin.as_str());
    let cors = if trusted {
        Some(state.app_origin.as_str())
    } else {
        None
    };
    if request.method == "OPTIONS" {
        if !trusted {
            json_response(
                &mut stream,
                "403 Forbidden",
                json!({"error":"Origin denied"}),
                None,
            );
        } else {
            response(&mut stream,"204 No Content","text/plain",b"",cors,"Access-Control-Allow-Methods: GET, POST, DELETE, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\n");
        }
        return;
    }
    if request.path == "/health" && request.method == "GET" {
        json_response(
            &mut stream,
            "200 OK",
            json!({"available":true,"isolation":if cfg!(target_os="macos"){"seatbelt"}else{"bubblewrap"},"source_limit":MAX_SOURCE,"concurrent":2,"retained_jobs":8,"timeout_ms":30000,"wasm_memory":268435456}),
            Some("*"),
        );
        return;
    }
    if request.path == "/api/users" && request.method == "GET" {
        response(
            &mut stream,
            "200 OK",
            "application/json",
            include_bytes!(concat!(env!("OUT_DIR"), "/users.json")),
            Some("*"),
            "",
        );
        return;
    }
    if request.path == "/compile" && request.method == "POST" {
        if !trusted
            || !request
                .headers
                .get("content-type")
                .is_some_and(|v| v.starts_with("application/json"))
        {
            json_response(
                &mut stream,
                "403 Forbidden",
                json!({"error":"Only the configured application origin and JSON source are accepted"}),
                cors,
            );
            return;
        }
        let value: Value = match serde_json::from_slice(&request.body) {
            Ok(v) => v,
            Err(_) => {
                json_response(
                    &mut stream,
                    "400 Bad Request",
                    json!({"error":"Invalid JSON"}),
                    cors,
                );
                return;
            }
        };
        let source = match value["source"].as_str() {
            Some(s) if s.len() <= MAX_SOURCE => s.to_owned(),
            _ => {
                json_response(
                    &mut stream,
                    "413 Content Too Large",
                    json!({"error":"Source exceeds 32 KiB"}),
                    cors,
                );
                return;
            }
        };
        let mut jobs = state.jobs.lock().unwrap();
        let expired = jobs
            .iter()
            .filter(|(_, j)| j.created.elapsed() > TTL)
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        for id in expired {
            if let Some(j) = jobs.remove(&id) {
                j.cancel.store(true, Ordering::Release);
                let _ = fs::remove_dir_all(j.path);
            }
        }
        if jobs.len() >= MAX_JOBS {
            let oldest = jobs
                .iter()
                .filter(|(_, j)| j.state["status"] != "compiling")
                .min_by_key(|(_, j)| j.created)
                .map(|(id, _)| id.clone());
            if let Some(id) = oldest {
                if let Some(job) = jobs.remove(&id) {
                    let _ = fs::remove_dir_all(job.path);
                }
            }
        }
        if jobs.len() >= MAX_JOBS || state.active.load(Ordering::Acquire) >= 2 {
            json_response(
                &mut stream,
                "429 Too Many Requests",
                json!({"error":"Compiler is busy; retry after current jobs finish or expire"}),
                cors,
            );
            return;
        }
        let id = match random() {
            Ok(id) => id,
            Err(_) => {
                json_response(
                    &mut stream,
                    "503 Service Unavailable",
                    json!({"error":"Random source unavailable"}),
                    cors,
                );
                return;
            }
        };
        let path = state.root.join(&id);
        if fs::create_dir_all(&path).is_err() {
            return;
        }
        let cancel = Arc::new(AtomicBool::new(false));
        jobs.insert(
            id.clone(),
            Job {
                created: Instant::now(),
                path: path.clone(),
                state: json!({"status":"compiling"}),
                cancel: cancel.clone(),
            },
        );
        state.active.fetch_add(1, Ordering::AcqRel);
        drop(jobs);
        json_response(
            &mut stream,
            "202 Accepted",
            json!({"id":id,"status":"compiling"}),
            cors,
        );
        let copy = state.clone();
        std::thread::spawn(move || {
            let result = compile(&copy, &path, &source, &cancel).unwrap_or_else(
                |e| json!({"status":if e=="cancelled"{"cancelled"}else{"error"},"message":e}),
            );
            let mut jobs = copy.jobs.lock().unwrap();
            if let Some(job) = jobs.get_mut(&id) {
                job.state = result;
                if job.state["status"] == "ready" {
                    job.state["preview"] = json!(format!("{}/jobs/{id}/preview.html", copy.origin));
                }
            }
            copy.active.fetch_sub(1, Ordering::AcqRel);
        });
        return;
    }
    let parts = request.path.split('/').collect::<Vec<_>>();
    if parts.len() >= 3
        && parts[1] == "jobs"
        && parts[2].len() == 32
        && parts[2].bytes().all(|b| b.is_ascii_hexdigit())
    {
        let job = {
            let jobs = state.jobs.lock().unwrap();
            jobs.get(parts[2])
                .map(|job| (job.state.clone(), job.path.clone(), job.cancel.clone()))
        };
        let Some((job_state, job_path, job_cancel)) = job else {
            json_response(
                &mut stream,
                "404 Not Found",
                json!({"error":"Job not found or expired"}),
                cors,
            );
            return;
        };
        if parts.len() == 3 && request.method == "GET" {
            if !trusted {
                json_response(
                    &mut stream,
                    "403 Forbidden",
                    json!({"error":"Origin denied"}),
                    None,
                );
            } else {
                json_response(&mut stream, "200 OK", job_state.clone(), cors);
            }
            return;
        }
        if parts.len() == 3 && request.method == "DELETE" {
            if !trusted {
                json_response(
                    &mut stream,
                    "403 Forbidden",
                    json!({"error":"Origin denied"}),
                    None,
                );
            } else {
                job_cancel.store(true, Ordering::Release);
                json_response(&mut stream, "200 OK", json!({"status":"cancelled"}), cors);
            }
            return;
        }
        if request.method == "GET" && job_state["status"] == "ready" {
            let csp=format!("Content-Security-Policy: default-src 'none'; script-src {} 'wasm-unsafe-eval'; style-src 'unsafe-inline'; connect-src {}; worker-src blob:; base-uri 'none'; object-src 'none'; frame-ancestors {}\r\n",state.origin,state.origin,state.app_origin);
            if parts.len() == 4 && parts[3] == "preview.html" {
                let html = include_str!(concat!(env!("OUT_DIR"), "/preview.html"))
                    .replace("__ORIGIN__", &state.origin)
                    .replace("__JOB__", parts[2]);
                response(
                    &mut stream,
                    "200 OK",
                    "text/html;charset=utf-8",
                    html.as_bytes(),
                    Some("*"),
                    &csp,
                );
                return;
            }
            if parts.len() >= 5
                && parts[3] == "pkg"
                && !parts[4..]
                    .iter()
                    .any(|p| p.is_empty() || p.contains(['%', '\\']) || *p == ".." || *p == ".")
            {
                let file = job_path.join("pkg").join(parts[4..].join("/"));
                if let Ok(bytes) = fs::read(&file) {
                    let mime = if file.extension().is_some_and(|e| e == "wasm") {
                        "application/wasm"
                    } else {
                        "text/javascript"
                    };
                    response(&mut stream, "200 OK", mime, &bytes, Some("*"), &csp);
                    return;
                }
            }
        }
    }
    if request.method == "GET"
        && ["/preview.js", "/worker.js", "/semantics.js", "/services.js"]
            .contains(&request.path.split('?').next().unwrap_or(""))
    {
        let bytes = match request.path.split('?').next().unwrap_or("") {
            "/preview.js" => include_str!(concat!(env!("OUT_DIR"), "/preview.js")),
            "/worker.js" => include_str!(concat!(env!("OUT_DIR"), "/worker.js")),
            "/semantics.js" => include_str!(concat!(env!("OUT_DIR"), "/semantics.js")),
            _ => include_str!(concat!(env!("OUT_DIR"), "/services.js")),
        };
        response(
            &mut stream,
            "200 OK",
            "text/javascript",
            bytes.as_bytes(),
            Some("*"),
            "",
        );
        return;
    }
    json_response(
        &mut stream,
        "404 Not Found",
        json!({"error":"Not found"}),
        cors,
    );
}
pub fn serve(framework: &Path, port: u16, app_origin: &str) -> Result<(), String> {
    let origin = format!("http://127.0.0.1:{port}");
    if !app_origin.starts_with("http://127.0.0.1:")
        || !app_origin[17..].bytes().all(|b| b.is_ascii_digit())
    {
        return Err(
            "Local compiler requires an explicit http://127.0.0.1:PORT application origin".into(),
        );
    }
    let listener = TcpListener::bind(("127.0.0.1", port)).map_err(|e| e.to_string())?;
    let cache = prepare(framework)?;
    let root = framework
        .join(".rustx/playground/jobs")
        .join(port.to_string());
    if root.exists() {
        fs::remove_dir_all(&root).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let state = Arc::new(State {
        jobs: Mutex::new(BTreeMap::new()),
        active: AtomicUsize::new(0),
        cache,
        root,
        origin: origin.clone(),
        app_origin: app_origin.to_owned(),
    });
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGTERM, stop as libc::sighandler_t);
        libc::signal(libc::SIGINT, stop as libc::sighandler_t);
    } // SAFETY: the handler only stores an atomic flag.
    println!("RustX isolated compiler serving {origin}; application origin {app_origin}");
    let connections = Arc::new(AtomicUsize::new(0));
    let mut cleaned = Instant::now();
    while !STOP.load(Ordering::Relaxed) {
        if cleaned.elapsed() > Duration::from_secs(1) {
            let mut jobs = state.jobs.lock().unwrap();
            let expired = jobs
                .iter()
                .filter(|(_, j)| j.created.elapsed() > TTL)
                .map(|(id, _)| id.clone())
                .collect::<Vec<_>>();
            for id in expired {
                if let Some(job) = jobs.remove(&id) {
                    job.cancel.store(true, Ordering::Release);
                    let _ = fs::remove_dir_all(job.path);
                }
            }
            cleaned = Instant::now();
        }
        match listener.accept() {
            Ok((stream, _)) => {
                if connections
                    .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                        (n < 16).then_some(n + 1)
                    })
                    .is_err()
                {
                    continue;
                }
                let s = state.clone();
                let counter = connections.clone();
                std::thread::spawn(move || {
                    handle(stream, s);
                    counter.fetch_sub(1, Ordering::AcqRel);
                });
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(25))
            }
            Err(e) => return Err(e.to_string()),
        }
    }
    for job in state.jobs.lock().unwrap().values() {
        job.cancel.store(true, Ordering::Release);
    }
    let deadline = Instant::now();
    while state.active.load(Ordering::Acquire) > 0 && deadline.elapsed() < Duration::from_secs(2) {
        std::thread::sleep(Duration::from_millis(20));
    }
    Ok(())
}
