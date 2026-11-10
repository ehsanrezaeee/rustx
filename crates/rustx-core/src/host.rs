//! Versioned, scope-owned HTTP boundary. Completion callbacks run outside borrows.
use crate::{Scope, MAX_HOST_ID};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

pub const HOST_PROTOCOL_VERSION: u32 = 1;
pub const MAX_HOST_REQUESTS: usize = 16;
pub const MAX_HOST_COMMANDS: usize = 32;
pub const MAX_HTTP_BYTES: usize = 1_048_576;
pub const HTTP_TIMEOUT_MS: u32 = 10_000;
pub const MAX_HTTP_URL_BYTES: usize = 8192;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityState {
    Supported,
    Unavailable,
    Denied,
    Failed,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostCapabilities {
    pub version: u32,
    pub http_get: CapabilityState,
    pub http_cancel: CapabilityState,
    pub max_requests: usize,
    pub max_response_bytes: usize,
    pub timeout_ms: u32,
    #[serde(default)]
    pub http_methods: bool,
}
impl HostCapabilities {
    pub fn get_cancel() -> Self {
        Self {
            version: HOST_PROTOCOL_VERSION,
            http_get: CapabilityState::Supported,
            http_cancel: CapabilityState::Supported,
            max_requests: MAX_HOST_REQUESTS,
            max_response_bytes: MAX_HTTP_BYTES,
            timeout_ms: HTTP_TIMEOUT_MS,
            http_methods: true,
        }
    }
}
#[derive(Clone, serde::Serialize)]
pub struct HttpRequest {
    pub version: u32,
    pub id: u64,
    pub url: String,
    pub max_bytes: usize,
    pub timeout_ms: u32,
    pub method: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub credentials: Credentials,
}
impl std::fmt::Debug for HttpRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpRequest")
            .field("id", &self.id)
            .field("method", &self.method)
            .field("body_bytes", &self.body.len())
            .finish_non_exhaustive()
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Credentials {
    Omit,
    #[default]
    SameOrigin,
    Include,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HostError {
    Unsupported(&'static str),
    Capability {
        service: &'static str,
        state: CapabilityState,
    },
    NotConnected,
    VersionMismatch {
        expected: u32,
        actual: u32,
    },
    InvalidCapabilities,
    AlreadyConnected,
    Busy,
    InvalidRequest,
    Cancelled,
    Timeout,
    TooLarge,
    Transport(String),
    Http(u16),
    Decode(String),
}
#[derive(Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}
impl std::fmt::Debug for HttpResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpResponse")
            .field("status", &self.status)
            .field("body_bytes", &self.body.len())
            .finish_non_exhaustive()
    }
}
impl HttpResponse {
    pub fn error_for_status(self) -> Result<Self, HostError> {
        if (200..300).contains(&self.status) {
            Ok(self)
        } else {
            Err(HostError::Http(self.status))
        }
    }
    pub fn bytes(self) -> Vec<u8> {
        self.body
    }
    pub fn text(self) -> Result<String, HostError> {
        String::from_utf8(self.body).map_err(|_| HostError::Decode("Response is not UTF-8".into()))
    }
    pub fn json<T: serde::de::DeserializeOwned>(self) -> Result<T, HostError> {
        serde_json::from_slice(&self.body).map_err(|e| {
            HostError::Decode(format!(
                "Invalid JSON at line {}, column {}",
                e.line(),
                e.column()
            ))
        })
    }
}
#[derive(Clone, Debug, serde::Serialize)]
#[serde(tag = "kind")]
pub enum HostCommand {
    Get(HttpRequest),
    Request(HttpRequest),
    Cancel { version: u32, id: u64 },
}
type Callback = Box<dyn FnOnce(Result<HttpResponse, HostError>)>;
struct Requests {
    next: u64,
    capabilities: Option<HostCapabilities>,
    queue: Vec<HostCommand>,
    pending: HashMap<u64, (Scope, u64, Callback)>,
}
impl Requests {
    fn cancel_command(&mut self, id: u64) {
        // Undispatched work needs no host cancellation. This coalescing also
        // keeps repeated start/cancel cycles bounded without pumping the host.
        let before = self.queue.len();
        self.queue
            .retain(|c| !matches!(c, HostCommand::Get(r) | HostCommand::Request(r) if r.id == id));
        if self.queue.len() == before {
            debug_assert!(self.queue.len() < MAX_HOST_COMMANDS);
            self.queue.push(HostCommand::Cancel {
                version: HOST_PROTOCOL_VERSION,
                id,
            });
        }
    }
}
#[derive(Clone)]
pub struct HttpClient(Rc<RefCell<Requests>>);
impl Default for HttpClient {
    fn default() -> Self {
        Self(Rc::new(RefCell::new(Requests {
            next: 1,
            capabilities: None,
            queue: vec![],
            pending: HashMap::new(),
        })))
    }
}
impl HttpClient {
    /// One handshake per client/session; replace the client to reconnect.
    pub fn connect(&self, capabilities: HostCapabilities) -> Result<(), HostError> {
        if capabilities.version != HOST_PROTOCOL_VERSION {
            return Err(HostError::VersionMismatch {
                expected: HOST_PROTOCOL_VERSION,
                actual: capabilities.version,
            });
        }
        if !(1..=MAX_HOST_REQUESTS).contains(&capabilities.max_requests)
            || !(1..=MAX_HTTP_BYTES).contains(&capabilities.max_response_bytes)
            || !(1..=HTTP_TIMEOUT_MS).contains(&capabilities.timeout_ms)
        {
            return Err(HostError::InvalidCapabilities);
        }
        let mut b = self.0.borrow_mut();
        if b.capabilities.is_some() {
            return Err(HostError::AlreadyConnected);
        }
        b.capabilities = Some(capabilities);
        Ok(())
    }
    pub fn capabilities(&self) -> Option<HostCapabilities> {
        self.0.borrow().capabilities.clone()
    }
    pub fn get(
        &self,
        scope: &Scope,
        url: impl Into<String>,
        f: impl FnOnce(Result<HttpResponse, HostError>) + 'static,
    ) -> u64 {
        self.start(scope, self.request("GET", url), true, f)
    }
    pub fn request(&self, method: impl Into<String>, url: impl Into<String>) -> RequestBuilder {
        RequestBuilder {
            client: self.clone(),
            method: method.into().to_ascii_uppercase(),
            url: url.into(),
            headers: vec![],
            body: vec![],
            timeout_ms: HTTP_TIMEOUT_MS,
            max_bytes: MAX_HTTP_BYTES,
            credentials: Credentials::SameOrigin,
            invalid: false,
        }
    }
    pub fn start(
        &self,
        scope: &Scope,
        request: RequestBuilder,
        legacy_get: bool,
        f: impl FnOnce(Result<HttpResponse, HostError>) + 'static,
    ) -> u64 {
        let url = request.url.clone();
        let id = {
            let mut b = self.0.borrow_mut();
            let id = b.next;
            assert!(id <= MAX_HOST_ID, "host ID space exhausted");
            b.next = b.next.checked_add(1).expect("request ID exhausted");
            let admission = (|| {
                if !scope.is_alive() {
                    return Err(HostError::Cancelled);
                }
                let caps = b.capabilities.as_ref().ok_or(HostError::NotConnected)?;
                if !legacy_get && !caps.http_methods {
                    return Err(HostError::Unsupported("HTTP methods"));
                }
                for (service, state) in [
                    ("http_get", caps.http_get),
                    ("http_cancel", caps.http_cancel),
                ] {
                    if state != CapabilityState::Supported {
                        return Err(HostError::Capability { service, state });
                    }
                }
                if request.invalid
                    || url.is_empty()
                    || url.len() > MAX_HTTP_URL_BYTES
                    || !matches!(
                        request.method.as_str(),
                        "GET" | "HEAD" | "POST" | "PUT" | "PATCH" | "DELETE" | "OPTIONS"
                    )
                    || request.body.len() > MAX_HTTP_BYTES
                    || (matches!(request.method.as_str(), "GET" | "HEAD")
                        && !request.body.is_empty())
                {
                    return Err(HostError::InvalidRequest);
                }
                if b.pending.len() >= caps.max_requests || b.queue.len() >= MAX_HOST_COMMANDS {
                    return Err(HostError::Busy);
                }
                Ok((
                    caps.max_response_bytes.min(request.max_bytes),
                    caps.timeout_ms.min(request.timeout_ms),
                ))
            })();
            let (max_bytes, timeout_ms) = match admission {
                Ok(limits) => limits,
                Err(error) => {
                    drop(b);
                    f(Err(error));
                    return id;
                }
            };
            b.pending.insert(id, (scope.clone(), 0, Box::new(f)));
            let req = HttpRequest {
                version: HOST_PROTOCOL_VERSION,
                id,
                url,
                max_bytes,
                timeout_ms,
                method: request.method,
                headers: request.headers,
                body: request.body,
                credentials: request.credentials,
            };
            b.queue.push(if legacy_get {
                HostCommand::Get(req)
            } else {
                HostCommand::Request(req)
            });
            id
        };
        let weak = Rc::downgrade(&self.0);
        let cleanup = scope.on_cleanup(move || {
            if let Some(i) = weak.upgrade() {
                let cb = {
                    let mut b = i.borrow_mut();
                    let c = b.pending.remove(&id);
                    if c.is_some() {
                        b.cancel_command(id);
                    }
                    c
                };
                drop(cb);
            }
        });
        if let Some(v) = self.0.borrow_mut().pending.get_mut(&id) {
            v.1 = cleanup;
        }
        id
    }
    pub fn cancel(&self, id: u64) {
        let cb = {
            let mut b = self.0.borrow_mut();
            let cb = b.pending.remove(&id);
            if cb.is_some() {
                b.cancel_command(id);
            }
            cb
        };
        if let Some((s, c, cb)) = cb {
            s.remove_cleanup(c);
            if s.is_alive() {
                cb(Err(HostError::Cancelled));
            }
        }
    }
    /// Reject mismatched envelopes without consuming the valid outstanding ID.
    pub fn complete(&self, version: u32, id: u64, result: Result<HttpResponse, HostError>) -> bool {
        if version != HOST_PROTOCOL_VERSION {
            return false;
        }
        let (cb, limit) = {
            let mut b = self.0.borrow_mut();
            let limit = b.capabilities.as_ref().map_or(0, |c| c.max_response_bytes);
            b.queue.retain(
                |c| !matches!(c, HostCommand::Get(r) | HostCommand::Request(r) if r.id == id),
            );
            (b.pending.remove(&id), limit)
        };
        if let Some((s, c, cb)) = cb {
            s.remove_cleanup(c);
            if s.is_alive() {
                let result = result.and_then(|r| {
                    if r.body.len() > limit
                        || r.headers.len() > 64
                        || r.headers
                            .iter()
                            .map(|(k, v)| k.len() + v.len())
                            .sum::<usize>()
                            > 32_768
                    {
                        Err(HostError::TooLarge)
                    } else {
                        Ok(r)
                    }
                });
                cb(result);
                return true;
            }
        }
        false
    }
    pub fn drain(&self) -> Vec<HostCommand> {
        std::mem::take(&mut self.0.borrow_mut().queue)
    }
    pub fn pending(&self) -> usize {
        self.0.borrow().pending.len()
    }
    pub fn queued(&self) -> usize {
        self.0.borrow().queue.len()
    }
}

/// Builder diagnostics intentionally omit URL queries, headers and payloads.
#[derive(Clone)]
pub struct RequestBuilder {
    client: HttpClient,
    method: String,
    url: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    timeout_ms: u32,
    max_bytes: usize,
    credentials: Credentials,
    invalid: bool,
}
impl RequestBuilder {
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        let (name, value) = (name.into(), value.into());
        if self.headers.len() >= 32
            || name.is_empty()
            || name.len() > 256
            || value.len() > 8192
            || !name
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&c))
            || value.contains(['\r', '\n', '\0'])
        {
            self.invalid = true;
        } else {
            self.headers.push((name, value));
        }
        self
    }
    pub fn body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = body.into();
        self
    }
    pub fn text(self, text: impl Into<String>) -> Self {
        self.header("content-type", "text/plain;charset=UTF-8")
            .body(text.into().into_bytes())
    }
    pub fn json<T: serde::Serialize>(mut self, value: &T) -> Self {
        match serde_json::to_vec(value) {
            Ok(v) => self.body = v,
            Err(_) => self.invalid = true,
        }
        self.header("content-type", "application/json")
    }
    pub fn timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout_ms = timeout.as_millis().clamp(1, HTTP_TIMEOUT_MS as u128) as u32;
        self
    }
    pub fn response_limit(mut self, limit: usize) -> Self {
        if !(1..=MAX_HTTP_BYTES).contains(&limit) {
            self.invalid = true;
        } else {
            self.max_bytes = limit;
        }
        self
    }
    pub fn credentials(mut self, policy: Credentials) -> Self {
        self.credentials = policy;
        self
    }
    pub fn send_callback(
        self,
        scope: &Scope,
        f: impl FnOnce(Result<HttpResponse, HostError>) + 'static,
    ) -> u64 {
        let client = self.client.clone();
        let limit = self.max_bytes;
        client.start(scope, self, false, move |r| {
            f(r.and_then(|r| {
                if r.body.len() > limit {
                    Err(HostError::TooLarge)
                } else {
                    Ok(r)
                }
            }))
        })
    }
    /// Dropping an unresolved future synchronously cancels its host request.
    pub fn send(self, scope: &Scope) -> HttpFuture {
        let state = Rc::new(RefCell::new(FutureState {
            result: None,
            waker: None,
        }));
        let out = state.clone();
        let client = self.client.clone();
        let id = self.send_callback(scope, move |result| {
            let wake = {
                let mut s = out.borrow_mut();
                s.result = Some(result);
                s.waker.take()
            };
            if let Some(w) = wake {
                w.wake();
            }
        });
        HttpFuture {
            client,
            id,
            state,
            completed: false,
        }
    }
    /// Multipart encoding uses a unique per-request boundary and rejects header injection.
    /// Upload progress is deliberately absent: browser Fetch cannot report it uniformly.
    pub fn multipart(mut self, parts: Vec<MultipartPart>) -> Self {
        let base = format!("rustx-{:016x}", self.client.0.borrow().next);
        let boundary = (0..32)
            .map(|suffix| format!("{base}-{suffix:02x}"))
            .find(|candidate| {
                parts.iter().all(|part| {
                    !part
                        .bytes
                        .windows(candidate.len())
                        .any(|bytes| bytes == candidate.as_bytes())
                })
            });
        let Some(boundary) = boundary else {
            self.invalid = true;
            return self;
        };
        let mut body = vec![];
        for part in parts {
            if part.name.is_empty()
                || part.name.contains(['\r', '\n', '"', '\\'])
                || part
                    .filename
                    .as_ref()
                    .is_some_and(|s| s.contains(['\r', '\n', '"', '\\']))
                || part.mime.contains(['\r', '\n'])
            {
                self.invalid = true;
                break;
            }
            let file = part
                .filename
                .map_or(String::new(), |n| format!("; filename=\"{n}\""));
            body.extend_from_slice(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"{}\"{file}\r\nContent-Type: {}\r\n\r\n", part.name, part.mime).as_bytes());
            body.extend(part.bytes);
            body.extend_from_slice(b"\r\n");
            if body.len() > MAX_HTTP_BYTES {
                self.invalid = true;
                break;
            }
        }
        body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
        self.body = body;
        self.header(
            "content-type",
            format!("multipart/form-data; boundary={boundary}"),
        )
    }
}
pub struct MultipartPart {
    pub name: String,
    pub filename: Option<String>,
    pub mime: String,
    pub bytes: Vec<u8>,
}
struct FutureState {
    result: Option<Result<HttpResponse, HostError>>,
    waker: Option<std::task::Waker>,
}
pub struct HttpFuture {
    client: HttpClient,
    id: u64,
    state: Rc<RefCell<FutureState>>,
    completed: bool,
}
impl std::future::Future for HttpFuture {
    type Output = Result<HttpResponse, HostError>;
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let result = self.state.borrow_mut().result.take();
        if let Some(r) = result {
            self.completed = true;
            std::task::Poll::Ready(r)
        } else {
            self.state.borrow_mut().waker = Some(cx.waker().clone());
            std::task::Poll::Pending
        }
    }
}
impl Drop for HttpFuture {
    fn drop(&mut self) {
        if !self.completed {
            self.client.cancel(self.id);
        }
    }
}
