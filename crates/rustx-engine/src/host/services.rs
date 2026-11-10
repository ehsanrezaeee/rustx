//! Native platform services. Blocking OS dialogs and I/O run off the UI thread.
use rustx_core::{
    ResourceFailure, ResourceSource, ServiceClient, ServiceError, ServiceOperation, ServiceValue,
    MAX_SERVICE_BYTES, SERVICE_VERSION,
};
use std::{
    collections::HashMap,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc, Arc,
    },
    time::{Duration, Instant},
};
use winit::event_loop::EventLoopProxy;
type Reply = (u64, Result<ServiceValue, ServiceError>);
pub struct NativeServices {
    tx: mpsc::SyncSender<Reply>,
    rx: mpsc::Receiver<Reply>,
    workers: HashMap<u64, mpsc::SyncSender<ServiceOperation>>,
    watchers: Vec<u64>,
    history: Vec<String>,
    cursor: usize,
    active: Arc<AtomicUsize>,
}
struct WorkerPermit(Arc<AtomicUsize>);
impl Drop for WorkerPermit {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}
fn failed() -> ServiceError {
    ServiceError::Failed {
        message: "Native service failed".into(),
    }
}
impl Default for NativeServices {
    fn default() -> Self {
        let (tx, rx) = mpsc::sync_channel(32);
        Self {
            tx,
            rx,
            workers: HashMap::new(),
            watchers: vec![],
            history: vec!["rustx://native/#/".into()],
            cursor: 0,
            active: Arc::new(AtomicUsize::new(0)),
        }
    }
}
impl NativeServices {
    fn permit(&self) -> Option<WorkerPermit> {
        self.active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                (count < 32).then_some(count + 1)
            })
            .ok()
            .map(|_| WorkerPermit(self.active.clone()))
    }
    pub fn capabilities() -> rustx_core::ServiceCapabilities {
        use rustx_core::{CapabilityState as C, ServiceCapabilities};
        let installed = |program: &str| {
            std::env::var_os("PATH").is_some_and(|paths| {
                std::env::split_paths(&paths).any(|p| p.join(program).is_file())
            })
        };
        let state = |value| if value { C::Supported } else { C::Unavailable };
        #[cfg(target_os = "macos")]
        let (clipboard, files, print) = (
            installed("pbcopy") && installed("pbpaste"),
            installed("osascript"),
            installed("osascript"),
        );
        #[cfg(target_os = "windows")]
        let (clipboard, files, print) = {
            let supported = installed("powershell.exe");
            (supported, supported, supported)
        };
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        let (clipboard, files, print) = (
            installed("wl-copy") && installed("wl-paste"),
            installed("zenity"),
            installed("lp"),
        );
        ServiceCapabilities {
            version: SERVICE_VERSION,
            clipboard: state(clipboard),
            files: state(files),
            print: state(print),
            storage: state(
                std::env::var_os("LOCALAPPDATA")
                    .or_else(|| std::env::var_os("XDG_DATA_HOME"))
                    .or_else(|| std::env::var_os("HOME"))
                    .is_some(),
            ),
            navigation: C::Supported,
            websocket: C::Supported,
            stream: C::Supported,
            scheduling: C::Supported,
            max_bytes: MAX_SERVICE_BYTES,
            max_resources: 32,
        }
    }
    fn navigate(
        &mut self,
        url: &str,
        replace: bool,
        hash: bool,
    ) -> Result<ServiceValue, ServiceError> {
        if !url.starts_with('/') || url.starts_with("//") || url.len() > 8192 {
            return Err(ServiceError::Invalid);
        }
        let url = if hash {
            format!("rustx://native/#{url}")
        } else {
            format!("rustx://native{url}")
        };
        if replace {
            self.history[self.cursor] = url;
        } else {
            self.history.truncate(self.cursor + 1);
            self.history.push(url);
            self.cursor += 1;
            if self.history.len() > 128 {
                self.history.remove(0);
                self.cursor -= 1;
            }
        }
        Ok(ServiceValue::Location {
            url: self.history[self.cursor].clone(),
        })
    }
    pub fn pump(
        &mut self,
        client: &ServiceClient,
        wake: &EventLoopProxy<()>,
    ) -> Vec<ResourceFailure> {
        let mut failures = vec![];
        for (id, result) in self.rx.try_iter() {
            let terminal = result.is_err()
                || matches!(
                    &result,
                    Ok(ServiceValue::Unit
                        | ServiceValue::Chunk { done: true, .. }
                        | ServiceValue::SocketClosed { .. })
                );
            if terminal {
                self.workers.remove(&id);
            }
            let failure = result.as_ref().err().and_then(|error| {
                let code = match error {
                    ServiceError::Cancelled => return None,
                    ServiceError::Unavailable => "unavailable",
                    ServiceError::Denied => "denied",
                    ServiceError::Quota => "quota",
                    ServiceError::TooLarge => "too_large",
                    ServiceError::Busy => "busy",
                    ServiceError::Invalid => "invalid",
                    ServiceError::Failed { .. } => "failed",
                };
                Some(ResourceFailure {
                    source: ResourceSource::Service,
                    code: code.into(),
                    status: None,
                })
            });
            if client.complete(SERVICE_VERSION, id, result) {
                failures.extend(failure);
            }
        }
        for command in client.drain() {
            let id = command.id;
            match command.operation {
                ServiceOperation::Cancel { resource } => {
                    if let Some(worker) = self.workers.remove(&resource) {
                        let _ = worker.try_send(ServiceOperation::Cancel { resource });
                    }
                    self.watchers.retain(|i| *i != resource);
                }
                ServiceOperation::WatchLocation => {
                    self.watchers.push(id);
                    client.complete(
                        SERVICE_VERSION,
                        id,
                        Ok(ServiceValue::Location {
                            url: self.history[self.cursor].clone(),
                        }),
                    );
                }
                ServiceOperation::Navigate { url, replace, hash } => {
                    let value = match self.navigate(&url, replace, hash) {
                        Ok(value) => value,
                        Err(error) => {
                            client.complete(SERVICE_VERSION, id, Err(error));
                            continue;
                        }
                    };
                    for observer in &self.watchers {
                        client.complete(SERVICE_VERSION, *observer, Ok(value.clone()));
                    }
                    client.complete(SERVICE_VERSION, id, Ok(value));
                }
                ServiceOperation::History { delta } => {
                    self.cursor = (self.cursor as i64 + delta as i64)
                        .clamp(0, self.history.len() as i64 - 1)
                        as usize;
                    let value = ServiceValue::Location {
                        url: self.history[self.cursor].clone(),
                    };
                    for observer in &self.watchers {
                        client.complete(SERVICE_VERSION, *observer, Ok(value.clone()));
                    }
                    client.complete(SERVICE_VERSION, id, Ok(ServiceValue::Unit));
                }
                operation @ (ServiceOperation::StreamRead { .. }
                | ServiceOperation::SocketSend { .. }
                | ServiceOperation::SocketClose { .. }) => {
                    let resource = match &operation {
                        ServiceOperation::StreamRead { resource }
                        | ServiceOperation::SocketSend { resource, .. }
                        | ServiceOperation::SocketClose { resource, .. } => *resource,
                        _ => unreachable!(),
                    };
                    if let Some(worker) = self.workers.get(&resource) {
                        if worker.try_send(operation).is_err() {
                            self.workers.remove(&resource);
                            client.complete(SERVICE_VERSION, resource, Err(ServiceError::Busy));
                        }
                    } else {
                        client.complete(SERVICE_VERSION, resource, Err(ServiceError::Invalid));
                    }
                }
                operation @ (ServiceOperation::Timer { .. }
                | ServiceOperation::StreamOpen { .. }
                | ServiceOperation::SocketOpen { .. }) => {
                    if self.workers.len() >= 32 {
                        client.complete(SERVICE_VERSION, id, Err(ServiceError::Busy));
                        continue;
                    }
                    let Some(permit) = self.permit() else {
                        client.complete(SERVICE_VERSION, id, Err(ServiceError::Busy));
                        continue;
                    };
                    let (tx, rx) = mpsc::sync_channel(8);
                    self.workers.insert(id, tx);
                    let output = self.tx.clone();
                    let wake = wake.clone();
                    std::thread::spawn(move || {
                        let _permit = permit;
                        let send = |result| {
                            let success = output.send((id, result)).is_ok();
                            let _ = wake.send_event(());
                            success
                        };
                        let result = match operation {
                            ServiceOperation::Timer { delay_ms } => {
                                if delay_ms == 0 || delay_ms > 60_000 {
                                    Err(ServiceError::Invalid)
                                } else {
                                    match rx.recv_timeout(Duration::from_millis(delay_ms as u64)) {
                                        Err(mpsc::RecvTimeoutError::Timeout) => {
                                            send(Ok(ServiceValue::Unit));
                                            Ok(())
                                        }
                                        _ => Ok(()),
                                    }
                                }
                            }
                            ServiceOperation::StreamOpen {
                                url,
                                max_bytes,
                                timeout_ms,
                            } => stream(url, max_bytes, timeout_ms, rx, &send),
                            ServiceOperation::SocketOpen { url, protocols } => {
                                socket(url, protocols, rx, &send)
                            }
                            _ => unreachable!(),
                        };
                        if let Err(error) = result {
                            send(Err(error));
                        }
                    });
                }
                operation => {
                    let Some(permit) = self.permit() else {
                        client.complete(SERVICE_VERSION, id, Err(ServiceError::Busy));
                        continue;
                    };
                    let output = self.tx.clone();
                    let wake = wake.clone();
                    std::thread::spawn(move || {
                        let _permit = permit;
                        let result = execute(operation);
                        let _ = output.send((id, result));
                        let _ = wake.send_event(());
                    });
                }
            }
        }
        failures
    }
}
fn output(command: Command, input: Option<&[u8]>) -> Result<String, ServiceError> {
    output_with_cancel(command, input, false)
}
fn output_with_cancel(
    mut command: Command,
    input: Option<&[u8]>,
    cancelable: bool,
) -> Result<String, ServiceError> {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    if input.is_some() {
        command.stdin(Stdio::piped());
    }
    let mut child = command.spawn().map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => ServiceError::Unavailable,
        std::io::ErrorKind::PermissionDenied => ServiceError::Denied,
        _ => failed(),
    })?;
    let mut stderr = child.stderr.take().ok_or_else(failed)?;
    let error_output = std::thread::spawn(move || {
        let mut bytes = vec![];
        let _ = (&mut stderr).take(4096).read_to_end(&mut bytes);
        let _ = std::io::copy(&mut stderr, &mut std::io::sink());
        bytes
    });
    if let Some(input) = input {
        child
            .stdin
            .take()
            .ok_or_else(failed)?
            .write_all(input)
            .map_err(|_| failed())
            .or_else(|error| {
                let _ = child.kill();
                let _ = child.wait();
                Err(error)
            })?;
    }
    let mut bytes = vec![];
    let read = child
        .stdout
        .take()
        .ok_or_else(failed)?
        .take(MAX_SERVICE_BYTES as u64 + 1)
        .read_to_end(&mut bytes);
    if read.is_err() || bytes.len() > MAX_SERVICE_BYTES {
        let _ = child.kill();
        let _ = child.wait();
        let _ = error_output.join();
        return Err(ServiceError::TooLarge);
    }
    let status = child.wait().map_err(|_| failed())?;
    let error =
        String::from_utf8_lossy(&error_output.join().unwrap_or_default()).to_ascii_lowercase();
    if !status.success() {
        if cancelable
            && (error.contains("(-128)")
                || (cfg!(not(any(target_os = "macos", target_os = "windows")))
                    && status.code() == Some(1)))
        {
            return Err(ServiceError::Cancelled);
        }
        if error.contains("permission denied")
            || error.contains("not permitted")
            || error.contains("not authorized")
            || error.contains("(-1743)")
            || error.contains("access is denied")
        {
            return Err(ServiceError::Denied);
        }
        return Err(failed());
    }
    String::from_utf8(bytes).map_err(|_| ServiceError::Invalid)
}
pub(super) fn clipboard(read: bool, text: &str) -> Result<ServiceValue, ServiceError> {
    if text.len() > MAX_SERVICE_BYTES {
        return Err(ServiceError::TooLarge);
    }
    #[cfg(target_os = "macos")]
    let command = Command::new(if read { "pbpaste" } else { "pbcopy" });
    #[cfg(target_os = "windows")]
    let command = {
        let mut c = Command::new("powershell.exe");
        c.args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            if read {
                "$ErrorActionPreference='Stop'; [Console]::OutputEncoding=[System.Text.UTF8Encoding]::new($false); Get-Clipboard -Raw"
            } else {
                "$ErrorActionPreference='Stop'; [Console]::InputEncoding=[System.Text.UTF8Encoding]::new($false); Set-Clipboard -Value ([Console]::In.ReadToEnd())"
            },
        ]);
        c
    };
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let command = Command::new(if read { "wl-paste" } else { "wl-copy" });
    let result = output(command, if read { None } else { Some(text.as_bytes()) })?;
    Ok(if read {
        ServiceValue::Text { text: result }
    } else {
        ServiceValue::Unit
    })
}
fn choose_file(save: bool, name: &str) -> Result<PathBuf, ServiceError> {
    #[cfg(target_os = "macos")]
    let command = {
        let mut c = Command::new("osascript");
        c.args([
            "-e",
            if save {
                "POSIX path of (choose file name with prompt \"Save RustX export\")"
            } else {
                "POSIX path of (choose file with prompt \"Open a file for RustX\")"
            },
        ]);
        c
    };
    #[cfg(target_os = "windows")]
    let command = {
        let mut c = Command::new("powershell.exe");
        c.args(["-NoProfile","-STA","-Command",if save{"$ErrorActionPreference='Stop'; [Console]::OutputEncoding=[System.Text.UTF8Encoding]::new($false); Add-Type -AssemblyName System.Windows.Forms; $d=New-Object System.Windows.Forms.SaveFileDialog; if($d.ShowDialog() -eq 'OK'){[Console]::Write($d.FileName)}"}else{"$ErrorActionPreference='Stop'; [Console]::OutputEncoding=[System.Text.UTF8Encoding]::new($false); Add-Type -AssemblyName System.Windows.Forms; $d=New-Object System.Windows.Forms.OpenFileDialog; if($d.ShowDialog() -eq 'OK'){[Console]::Write($d.FileName)}"}]);
        c
    };
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let command = {
        let mut c = Command::new("zenity");
        c.arg("--file-selection");
        if save {
            c.args(["--save", "--confirm-overwrite", "--filename", name]);
        }
        c
    };
    let _ = name;
    let path = output_with_cancel(command, None, true)?;
    let path = path.trim();
    if path.is_empty() {
        Err(ServiceError::Cancelled)
    } else {
        Ok(PathBuf::from(path))
    }
}
fn storage_path(key: &str) -> Result<PathBuf, ServiceError> {
    if key.is_empty() || key.len() > 1024 {
        return Err(ServiceError::Invalid);
    }
    let base = std::env::var_os("LOCALAPPDATA")
        .or_else(|| std::env::var_os("XDG_DATA_HOME"))
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .ok_or(ServiceError::Unavailable)?;
    let directory = base.join("rustx");
    std::fs::create_dir_all(&directory).map_err(|_| ServiceError::Denied)?;
    if key.len() > 124 {
        return Err(ServiceError::TooLarge);
    }
    let encoded = key.bytes().map(|b| format!("{b:02x}")).collect::<String>();
    Ok(directory.join(format!("{encoded}.json")))
}
struct TemporaryPath(PathBuf);
impl Drop for TemporaryPath {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
fn storage_lock(path: &Path) -> Result<std::fs::File, ServiceError> {
    let directory = path.parent().ok_or(ServiceError::Invalid)?;
    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(directory.join(".lock"))
        .map_err(|_| ServiceError::Denied)?;
    file.lock().map_err(|_| failed())?;
    Ok(file)
}
fn read_bounded(file: std::fs::File, limit: usize) -> Result<Vec<u8>, ServiceError> {
    if !file.metadata().map_err(|_| failed())?.is_file() {
        return Err(ServiceError::Invalid);
    }
    let mut bytes = vec![];
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| failed())?;
    if bytes.len() > limit {
        return Err(ServiceError::TooLarge);
    }
    Ok(bytes)
}
fn storage_write(path: &Path, version: u32, value: &str) -> Result<(), ServiceError> {
    if version == 0 {
        return Err(ServiceError::Invalid);
    }
    if value.len() > MAX_SERVICE_BYTES {
        return Err(ServiceError::TooLarge);
    }
    let bytes = serde_json::to_vec(&serde_json::json!({"version":version,"value":value}))
        .map_err(|_| ServiceError::Invalid)?;
    let _lock = storage_lock(path)?;
    let directory = path.parent().ok_or(ServiceError::Invalid)?;
    let size = std::fs::read_dir(directory)
        .map_err(|_| ServiceError::Denied)?
        .filter_map(Result::ok)
        .filter(|entry| entry.path() != path)
        .filter_map(|entry| entry.metadata().ok())
        .map(|metadata| metadata.len())
        .sum::<u64>();
    if size + bytes.len() as u64 > 8 * MAX_SERVICE_BYTES as u64 {
        return Err(ServiceError::Quota);
    }
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| failed())?
        .as_nanos();
    let temporary =
        TemporaryPath(directory.join(format!(".rustx-{}-{nonce}.tmp", std::process::id())));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary.0)
        .map_err(|_| ServiceError::Denied)?;
    file.write_all(&bytes).map_err(|_| failed())?;
    file.sync_all().map_err(|_| failed())?;
    drop(file);
    std::fs::rename(&temporary.0, path).map_err(|_| ServiceError::Denied)?;
    Ok(())
}
fn execute(operation: ServiceOperation) -> Result<ServiceValue, ServiceError> {
    match operation {
        ServiceOperation::ClipboardRead => clipboard(true, ""),
        ServiceOperation::ClipboardWrite { text } => clipboard(false, &text),
        ServiceOperation::StorageGet { key } => {
            let path = storage_path(&key)?;
            let _lock = storage_lock(&path)?;
            match std::fs::File::open(path) {
                Ok(file) => {
                    let bytes = read_bounded(file, MAX_SERVICE_BYTES * 6 + 1024)?;
                    let value: serde_json::Value =
                        serde_json::from_slice(&bytes).map_err(|_| ServiceError::Invalid)?;
                    let version =
                        u32::try_from(value["version"].as_u64().ok_or(ServiceError::Invalid)?)
                            .map_err(|_| ServiceError::Invalid)?;
                    let value = value["value"].as_str().ok_or(ServiceError::Invalid)?;
                    if version == 0 {
                        return Err(ServiceError::Invalid);
                    }
                    if value.len() > MAX_SERVICE_BYTES {
                        return Err(ServiceError::TooLarge);
                    }
                    Ok(ServiceValue::Storage {
                        version,
                        value: value.to_owned(),
                    })
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(ServiceValue::Missing),
                Err(_) => Err(ServiceError::Denied),
            }
        }
        ServiceOperation::StorageSet {
            key,
            version,
            value,
        } => {
            let path = storage_path(&key)?;
            storage_write(&path, version, &value)?;
            Ok(ServiceValue::Unit)
        }
        ServiceOperation::StorageRemove { key } => {
            let path = storage_path(&key)?;
            let _lock = storage_lock(&path)?;
            match std::fs::remove_file(path) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => return Err(ServiceError::Denied),
            }
            Ok(ServiceValue::Unit)
        }
        ServiceOperation::FileOpen {
            accept: _,
            max_bytes,
        } => {
            if max_bytes == 0 || max_bytes > MAX_SERVICE_BYTES {
                return Err(ServiceError::Invalid);
            }
            let path = choose_file(false, "")?;
            let bytes = read_bounded(
                std::fs::File::open(&path).map_err(|_| ServiceError::Denied)?,
                max_bytes,
            )?;
            Ok(ServiceValue::File {
                name: path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
                mime: "application/octet-stream".into(),
                bytes,
            })
        }
        ServiceOperation::FileDownload {
            name,
            mime: _,
            bytes,
        } => {
            if bytes.len() > MAX_SERVICE_BYTES {
                return Err(ServiceError::TooLarge);
            }
            let path = choose_file(true, &name)?;
            std::fs::write(path, bytes).map_err(|_| ServiceError::Denied)?;
            Ok(ServiceValue::Unit)
        }
        ServiceOperation::Print { title, text } => print_text(&title, &text),
        _ => Err(ServiceError::Unavailable),
    }
}
fn print_text(title: &str, text: &str) -> Result<ServiceValue, ServiceError> {
    if text.len() > MAX_SERVICE_BYTES || title.len() > 256 {
        return Err(ServiceError::TooLarge);
    }
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| failed())?
        .as_nanos();
    let path = std::env::temp_dir().join(format!("rustx-print-{}-{nonce}.txt", std::process::id()));
    let _temporary = TemporaryPath(path.clone());
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|_| ServiceError::Denied)?;
    file.write_all(format!("{title}\n\n{text}").as_bytes())
        .map_err(|_| failed())?;
    drop(file);
    #[cfg(target_os = "macos")]
    let command = {
        let mut c = Command::new("osascript");
        c.args([
            "-e",
            "on run argv",
            "-e",
            "tell application \"TextEdit\"",
            "-e",
            "set printDocument to open POSIX file (item 1 of argv)",
            "-e",
            "print printDocument with print dialog",
            "-e",
            "close printDocument saving no",
            "-e",
            "end tell",
            "-e",
            "end run",
            "--",
        ]);
        c.arg(&path);
        c
    };
    #[cfg(target_os = "windows")]
    let command = {
        let mut c = Command::new("powershell.exe");
        c.args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "$ErrorActionPreference='Stop'; Start-Process -FilePath $env:RUSTX_PRINT_PATH -Verb Print -Wait",
        ]);
        c.env("RUSTX_PRINT_PATH", &path);
        c
    };
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let command = {
        let mut c = Command::new("lp");
        c.arg(&path);
        c
    };
    let result = output_with_cancel(command, None, true);
    let _ = std::fs::remove_file(path);
    result?;
    Ok(ServiceValue::Unit)
}
fn stream(
    url: String,
    max: usize,
    timeout: u32,
    commands: mpsc::Receiver<ServiceOperation>,
    send: &impl Fn(Result<ServiceValue, ServiceError>) -> bool,
) -> Result<(), ServiceError> {
    if max == 0 || max > MAX_SERVICE_BYTES || timeout == 0 || timeout > 10_000 {
        return Err(ServiceError::Invalid);
    }
    let url = if url.starts_with('/') {
        format!("http://127.0.0.1:3000{url}")
    } else {
        url
    };
    let response = match ureq::AgentBuilder::new()
        .timeout(Duration::from_millis(timeout as u64))
        .build()
        .get(&url)
        .call()
    {
        Ok(r) => r,
        Err(ureq::Error::Status(_, r)) => r,
        Err(_) => return Err(failed()),
    };
    let headers = response
        .headers_names()
        .into_iter()
        .filter_map(|name| response.header(&name).map(|v| (name, v.to_owned())))
        .collect();
    let status = response.status();
    let total = response
        .header("content-length")
        .and_then(|s| s.parse().ok());
    let mut reader = response.into_reader();
    let mut received = 0;
    let start = Instant::now();
    if !send(Ok(ServiceValue::Stream { status, headers })) {
        return Ok(());
    }
    loop {
        let remaining = Duration::from_millis(timeout as u64).saturating_sub(start.elapsed());
        let command = commands.recv_timeout(remaining).map_err(|_| failed())?;
        match command {
            ServiceOperation::Cancel { .. } => return Ok(()),
            ServiceOperation::StreamRead { .. } => {
                let mut bytes = vec![0; 65_536];
                let n = reader.read(&mut bytes).map_err(|_| failed())?;
                bytes.truncate(n);
                received += n;
                if received > max {
                    return Err(ServiceError::TooLarge);
                }
                if !send(Ok(ServiceValue::Chunk {
                    bytes,
                    done: n == 0,
                    received,
                    total,
                })) {
                    return Ok(());
                }
                if n == 0 {
                    return Ok(());
                }
            }
            _ => return Err(ServiceError::Invalid),
        }
    }
}
fn socket(
    url: String,
    protocols: Vec<String>,
    commands: mpsc::Receiver<ServiceOperation>,
    send: &impl Fn(Result<ServiceValue, ServiceError>) -> bool,
) -> Result<(), ServiceError> {
    use std::net::{TcpStream, ToSocketAddrs};
    use tungstenite::{
        client::IntoClientRequest,
        protocol::{frame::coding::CloseCode, CloseFrame, WebSocketConfig},
        stream::MaybeTlsStream,
        Message,
    };
    let url = if url.starts_with('/') {
        format!("ws://127.0.0.1:3000{url}")
    } else {
        url
    };
    if url.len() > 8192
        || protocols.len() > 8
        || protocols.iter().any(|s| {
            s.len() > 128
                || !s
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
        })
    {
        return Err(ServiceError::Invalid);
    }
    let mut request = url
        .as_str()
        .into_client_request()
        .map_err(|_| ServiceError::Invalid)?;
    let uri = request.uri();
    let host = uri.host().ok_or(ServiceError::Invalid)?;
    let port = uri
        .port_u16()
        .unwrap_or(if uri.scheme_str() == Some("wss") {
            443
        } else {
            80
        });
    let addresses = (host, port).to_socket_addrs().map_err(|_| failed())?;
    let mut stream = None;
    for address in addresses.take(4) {
        if let Ok(s) = TcpStream::connect_timeout(&address, Duration::from_secs(3)) {
            stream = Some(s);
            break;
        }
    }
    let stream = stream.ok_or_else(failed)?;
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .map_err(|_| failed())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(10)))
        .map_err(|_| failed())?;
    if !protocols.is_empty() {
        request.headers_mut().insert(
            "Sec-WebSocket-Protocol",
            protocols
                .join(", ")
                .parse()
                .map_err(|_| ServiceError::Invalid)?,
        );
    }
    let config = WebSocketConfig::default()
        .read_buffer_size(8192)
        .write_buffer_size(0)
        .max_write_buffer_size(262_144)
        .max_message_size(Some(65_536))
        .max_frame_size(Some(65_536));
    let (mut socket, _) = tungstenite::client_tls_with_config(request, stream, Some(config), None)
        .map_err(|_| failed())?;
    match socket.get_mut() {
        MaybeTlsStream::Plain(s) => {
            s.set_read_timeout(Some(Duration::from_millis(25)))
                .map_err(|_| failed())?;
        }
        MaybeTlsStream::Rustls(s) => {
            s.sock
                .set_read_timeout(Some(Duration::from_millis(25)))
                .map_err(|_| failed())?;
        }
        _ => {}
    }
    if !send(Ok(ServiceValue::SocketOpen)) {
        let _ = socket.close(None);
        return Ok(());
    }
    loop {
        loop {
            match commands.try_recv() {
                Ok(ServiceOperation::Cancel { .. }) | Err(mpsc::TryRecvError::Disconnected) => {
                    let _ = socket.close(None);
                    return Ok(());
                }
                Ok(ServiceOperation::SocketSend { bytes, text, .. }) => {
                    if bytes.len() > 65_536 {
                        return Err(ServiceError::TooLarge);
                    }
                    let message = if text {
                        Message::Text(
                            String::from_utf8(bytes)
                                .map_err(|_| ServiceError::Invalid)?
                                .into(),
                        )
                    } else {
                        Message::Binary(bytes.into())
                    };
                    socket.send(message).map_err(|_| failed())?;
                }
                Ok(ServiceOperation::SocketClose { code, reason, .. }) => {
                    if (code != 1000 && !(3000..=4999).contains(&code)) || reason.len() > 123 {
                        return Err(ServiceError::Invalid);
                    }
                    socket
                        .close(Some(CloseFrame {
                            code: CloseCode::from(code),
                            reason: reason.into(),
                        }))
                        .map_err(|_| failed())?;
                }
                Ok(_) => return Err(ServiceError::Invalid),
                Err(mpsc::TryRecvError::Empty) => break,
            }
        }
        match socket.read() {
            Ok(Message::Text(text)) => {
                if !send(Ok(ServiceValue::SocketMessage {
                    bytes: text.as_bytes().to_vec(),
                    text: true,
                })) {
                    return Ok(());
                }
            }
            Ok(Message::Binary(bytes)) => {
                if !send(Ok(ServiceValue::SocketMessage {
                    bytes: bytes.to_vec(),
                    text: false,
                })) {
                    return Ok(());
                }
            }
            Ok(Message::Close(frame)) => {
                let (code, reason) = frame.map_or((1000, String::new()), |f| {
                    (u16::from(f.code), f.reason.to_string())
                });
                send(Ok(ServiceValue::SocketClosed {
                    code,
                    reason,
                    clean: true,
                }));
                return Ok(());
            }
            Ok(_) => {}
            Err(tungstenite::Error::Io(error))
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) => {}
            Err(tungstenite::Error::ConnectionClosed | tungstenite::Error::AlreadyClosed) => {
                send(Ok(ServiceValue::SocketClosed {
                    code: 1000,
                    reason: String::new(),
                    clean: true,
                }));
                return Ok(());
            }
            Err(tungstenite::Error::Capacity(_)) => return Err(ServiceError::TooLarge),
            Err(_) => return Err(failed()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_navigation_preserves_history_and_hash_route_modes() {
        use rustx_core::{match_location, RouteMode, RouteState};
        for (hash, mode) in [(false, RouteMode::History), (true, RouteMode::Hash)] {
            let mut host = NativeServices::default();
            let routes = vec![("/people/:id".into(), "person")];
            let value = host
                .navigate("/people/42?tab=details#name", false, hash)
                .unwrap();
            let ServiceValue::Location { url } = value else {
                panic!("Expected location");
            };
            let RouteState::Found(found) = match_location(&url, mode, &routes) else {
                panic!("Native URL did not match route: {url}");
            };
            assert_eq!(found.route, "person");
            assert_eq!(found.params.get("id").map(String::as_str), Some("42"));
            assert_eq!(found.query, vec![("tab".into(), "details".into())]);
            assert_eq!(found.fragment, "name");
            assert_eq!(host.history.len(), 2);
            host.navigate("/people/43", true, hash).unwrap();
            assert_eq!(
                host.history.len(),
                2,
                "replace must retain the history length"
            );
            assert_eq!(host.cursor, 1);
            assert!(matches!(
                host.navigate("//external.test/", false, hash),
                Err(ServiceError::Invalid)
            ));
        }
    }
    #[test]
    #[ignore = "requires permission to bind loopback TCP"]
    fn actual_native_pull_stream_and_socket_echo() {
        use std::{cell::RefCell, net::TcpListener};
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut request = [0; 2048];
            socket.read(&mut request).unwrap();
            socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 12\r\nConnection: close\r\n\r\nnative bytes").unwrap();
        });
        let (tx, rx) = mpsc::sync_channel(8);
        let chunks = RefCell::new(vec![]);
        stream(format!("http://{address}/"), 1024, 3000, rx, &|result| {
            match result.unwrap() {
                ServiceValue::Stream { status, .. } => {
                    assert_eq!(status, 200);
                    tx.send(ServiceOperation::StreamRead { resource: 1 })
                        .unwrap();
                }
                ServiceValue::Chunk { bytes, done, .. } => {
                    chunks.borrow_mut().extend(bytes);
                    if !done {
                        tx.send(ServiceOperation::StreamRead { resource: 1 })
                            .unwrap();
                    }
                }
                other => panic!("{other:?}"),
            }
            true
        })
        .unwrap();
        server.join().unwrap();
        assert_eq!(&*chunks.borrow(), b"native bytes");
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (socket, _) = listener.accept().unwrap();
            let mut socket = tungstenite::accept(socket).unwrap();
            let message = socket.read().unwrap();
            socket.send(message).unwrap();
            let close = socket.read().unwrap();
            assert!(close.is_close());
            let _ = socket.flush();
        });
        let (tx, rx) = mpsc::sync_channel(8);
        let echoed = RefCell::new(false);
        socket(format!("ws://{address}/"), vec![], rx, &|result| {
            match result.unwrap() {
                ServiceValue::SocketOpen => {
                    tx.send(ServiceOperation::SocketSend {
                        resource: 1,
                        bytes: "سلام native".as_bytes().to_vec(),
                        text: true,
                    })
                    .unwrap();
                }
                ServiceValue::SocketMessage { bytes, text } => {
                    assert!(text);
                    assert_eq!(bytes, "سلام native".as_bytes());
                    *echoed.borrow_mut() = true;
                    tx.send(ServiceOperation::SocketClose {
                        resource: 1,
                        code: 1000,
                        reason: "done".into(),
                    })
                    .unwrap();
                }
                ServiceValue::SocketClosed { code, .. } => assert_eq!(code, 1000),
                other => panic!("{other:?}"),
            }
            true
        })
        .unwrap();
        server.join().unwrap();
        assert!(*echoed.borrow());
    }
}
