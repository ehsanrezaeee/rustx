//! Versioned, scope-owned GET/cancel boundary. This is not the Stage 3 HTTP API.
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
        }
    }
}
#[derive(Clone, Debug, serde::Serialize)]
pub struct HttpRequest {
    pub version: u32,
    pub id: u64,
    pub url: String,
    pub max_bytes: usize,
    pub timeout_ms: u32,
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
#[derive(Clone, Debug)]
pub struct HttpResponse {
    pub status: u16,
    pub body: Vec<u8>,
}
#[derive(Clone, Debug, serde::Serialize)]
#[serde(tag = "kind")]
pub enum HostCommand {
    Get(HttpRequest),
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
            .retain(|c| !matches!(c, HostCommand::Get(r) if r.id == id));
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
        let url = url.into();
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
                for (service, state) in [
                    ("http_get", caps.http_get),
                    ("http_cancel", caps.http_cancel),
                ] {
                    if state != CapabilityState::Supported {
                        return Err(HostError::Capability { service, state });
                    }
                }
                if url.is_empty() || url.len() > MAX_HTTP_URL_BYTES {
                    return Err(HostError::InvalidRequest);
                }
                if b.pending.len() >= caps.max_requests || b.queue.len() >= MAX_HOST_COMMANDS {
                    return Err(HostError::Busy);
                }
                Ok((caps.max_response_bytes, caps.timeout_ms))
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
            b.queue.push(HostCommand::Get(HttpRequest {
                version: HOST_PROTOCOL_VERSION,
                id,
                url,
                max_bytes,
                timeout_ms,
            }));
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
            b.queue
                .retain(|c| !matches!(c, HostCommand::Get(r) if r.id == id));
            (b.pending.remove(&id), limit)
        };
        if let Some((s, c, cb)) = cb {
            s.remove_cleanup(c);
            if s.is_alive() {
                let result = result.and_then(|r| {
                    if r.body.len() > limit {
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
