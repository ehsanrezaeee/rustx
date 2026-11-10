//! Portable, bounded capability calls. Hosts report permission and availability explicitly.
use crate::{CapabilityState, Scope, MAX_HOST_ID};
use std::{
    cell::RefCell,
    collections::{BTreeSet, HashMap},
    rc::Rc,
};
pub const SERVICE_VERSION: u32 = 1;
pub const MAX_SERVICE_BYTES: usize = 1_048_576;
pub const MAX_SERVICE_CALLS: usize = 32;
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceCapabilities {
    pub version: u32,
    pub clipboard: CapabilityState,
    pub files: CapabilityState,
    pub storage: CapabilityState,
    pub navigation: CapabilityState,
    pub websocket: CapabilityState,
    pub stream: CapabilityState,
    pub print: CapabilityState,
    pub scheduling: CapabilityState,
    pub max_bytes: usize,
    pub max_resources: usize,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ServiceError {
    Unavailable,
    Denied,
    Cancelled,
    Quota,
    TooLarge,
    Busy,
    Invalid,
    Failed { message: String },
}
#[derive(Clone, serde::Serialize)]
pub struct ServiceCommand {
    pub version: u32,
    pub id: u64,
    #[serde(flatten)]
    pub operation: ServiceOperation,
}
impl std::fmt::Debug for ServiceCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServiceCommand")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}
#[derive(Clone, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ServiceOperation {
    Timer {
        delay_ms: u32,
    },
    Cancel {
        resource: u64,
    },
    ClipboardRead,
    ClipboardWrite {
        text: String,
    },
    FileOpen {
        accept: String,
        max_bytes: usize,
    },
    FileDownload {
        name: String,
        mime: String,
        bytes: Vec<u8>,
    },
    StorageGet {
        key: String,
    },
    StorageSet {
        key: String,
        #[serde(rename = "schema_version")]
        version: u32,
        value: String,
    },
    StorageRemove {
        key: String,
    },
    WatchLocation,
    Navigate {
        url: String,
        replace: bool,
        hash: bool,
    },
    History {
        delta: i32,
    },
    StreamOpen {
        url: String,
        max_bytes: usize,
        timeout_ms: u32,
    },
    StreamRead {
        resource: u64,
    },
    SocketOpen {
        url: String,
        protocols: Vec<String>,
    },
    SocketSend {
        resource: u64,
        bytes: Vec<u8>,
        text: bool,
    },
    SocketClose {
        resource: u64,
        code: u16,
        reason: String,
    },
    Print {
        title: String,
        text: String,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServiceValue {
    Unit,
    Text {
        text: String,
    },
    Storage {
        version: u32,
        value: String,
    },
    Missing,
    File {
        name: String,
        mime: String,
        bytes: Vec<u8>,
    },
    Location {
        url: String,
    },
    Stream {
        status: u16,
        headers: Vec<(String, String)>,
    },
    Chunk {
        bytes: Vec<u8>,
        done: bool,
        received: usize,
        total: Option<usize>,
    },
    SocketOpen,
    SocketMessage {
        bytes: Vec<u8>,
        text: bool,
    },
    SocketClosed {
        code: u16,
        reason: String,
        clean: bool,
    },
}
#[derive(serde::Deserialize)]
struct Envelope {
    value: Option<ServiceValue>,
    error: Option<ServiceError>,
}
type Callback = Rc<dyn Fn(Result<ServiceValue, ServiceError>)>;
struct Pending {
    scope: Scope,
    cleanup: u64,
    callback: Callback,
    persistent: bool,
}
struct Inner {
    next: u64,
    queue: Vec<ServiceCommand>,
    pending: HashMap<u64, Pending>,
    capabilities: Option<ServiceCapabilities>,
    cancellations: BTreeSet<u64>,
}
#[derive(Clone)]
pub struct ServiceClient(Rc<RefCell<Inner>>);
impl Default for ServiceClient {
    fn default() -> Self {
        Self(Rc::new(RefCell::new(Inner {
            next: 1,
            queue: vec![],
            pending: HashMap::new(),
            capabilities: None,
            cancellations: BTreeSet::new(),
        })))
    }
}
impl ServiceClient {
    pub fn connect(&self, capabilities: ServiceCapabilities) -> Result<(), ServiceError> {
        if capabilities.version != SERVICE_VERSION
            || capabilities.max_bytes == 0
            || capabilities.max_bytes > MAX_SERVICE_BYTES
            || capabilities.max_resources == 0
            || capabilities.max_resources > MAX_SERVICE_CALLS
        {
            return Err(ServiceError::Invalid);
        }
        let mut inner = self.0.borrow_mut();
        if inner.capabilities.is_some() {
            return Err(ServiceError::Invalid);
        }
        inner.capabilities = Some(capabilities);
        Ok(())
    }
    pub fn capabilities(&self) -> Option<ServiceCapabilities> {
        self.0.borrow().capabilities.clone()
    }
    pub fn call(
        &self,
        scope: &Scope,
        operation: ServiceOperation,
        callback: impl Fn(Result<ServiceValue, ServiceError>) + 'static,
    ) -> Result<u64, ServiceError> {
        self.start(scope, operation, Rc::new(callback), false)
    }
    /// Streams and sockets remain owned by the scope until explicitly cancelled.
    pub fn subscribe(
        &self,
        scope: &Scope,
        operation: ServiceOperation,
        callback: impl Fn(Result<ServiceValue, ServiceError>) + 'static,
    ) -> Result<u64, ServiceError> {
        self.start(scope, operation, Rc::new(callback), true)
    }
    fn start(
        &self,
        scope: &Scope,
        operation: ServiceOperation,
        callback: Callback,
        persistent: bool,
    ) -> Result<u64, ServiceError> {
        if !scope.is_alive() {
            return Err(ServiceError::Cancelled);
        }
        let id = {
            let mut inner = self.0.borrow_mut();
            if let Some(caps) = &inner.capabilities {
                let state =
                    match &operation {
                        ServiceOperation::Timer { .. } => caps.scheduling,
                        ServiceOperation::ClipboardRead
                        | ServiceOperation::ClipboardWrite { .. } => caps.clipboard,
                        ServiceOperation::FileOpen { .. }
                        | ServiceOperation::FileDownload { .. } => caps.files,
                        ServiceOperation::StorageGet { .. }
                        | ServiceOperation::StorageSet { .. }
                        | ServiceOperation::StorageRemove { .. } => caps.storage,
                        ServiceOperation::WatchLocation
                        | ServiceOperation::Navigate { .. }
                        | ServiceOperation::History { .. } => caps.navigation,
                        ServiceOperation::StreamOpen { .. }
                        | ServiceOperation::StreamRead { .. } => caps.stream,
                        ServiceOperation::SocketOpen { .. }
                        | ServiceOperation::SocketSend { .. }
                        | ServiceOperation::SocketClose { .. } => caps.websocket,
                        ServiceOperation::Print { .. } => caps.print,
                        ServiceOperation::Cancel { .. } => CapabilityState::Supported,
                    };
                match state {
                    CapabilityState::Supported => {}
                    CapabilityState::Denied => return Err(ServiceError::Denied),
                    CapabilityState::Unavailable => return Err(ServiceError::Unavailable),
                    CapabilityState::Failed => {
                        return Err(ServiceError::Failed {
                            message: "Host service unavailable after failure".into(),
                        })
                    }
                }
                if inner.pending.len() + inner.cancellations.len() >= caps.max_resources {
                    return Err(ServiceError::Busy);
                }
            }
            if inner.pending.len() + inner.cancellations.len() >= MAX_SERVICE_CALLS
                || inner.queue.len() >= MAX_SERVICE_CALLS
            {
                return Err(ServiceError::Busy);
            }
            let id = inner.next;
            if id > MAX_HOST_ID {
                return Err(ServiceError::Busy);
            }
            let encoded = serde_json::to_vec(&operation).map_err(|_| ServiceError::Invalid)?;
            // JSON arrays encode binary bytes with up to four bytes each.
            if encoded.len() > MAX_SERVICE_BYTES * 4 + 8192 {
                return Err(ServiceError::TooLarge);
            }
            inner.next += 1;
            inner.pending.insert(
                id,
                Pending {
                    scope: scope.clone(),
                    cleanup: 0,
                    callback,
                    persistent,
                },
            );
            inner.queue.push(ServiceCommand {
                version: SERVICE_VERSION,
                id,
                operation,
            });
            id
        };
        let weak = Rc::downgrade(&self.0);
        let cleanup = scope.on_cleanup(move || {
            if let Some(inner) = weak.upgrade() {
                ServiceClient(inner).cancel(id);
            }
        });
        if let Some(p) = self.0.borrow_mut().pending.get_mut(&id) {
            p.cleanup = cleanup;
        }
        Ok(id)
    }
    pub fn command(&self, operation: ServiceOperation) -> Result<(), ServiceError> {
        if let ServiceOperation::Cancel { resource } = &operation {
            self.cancel(*resource);
            return Ok(());
        }
        match &operation {
            ServiceOperation::Cancel { .. } | ServiceOperation::StreamRead { .. } => {}
            ServiceOperation::SocketSend { bytes, .. } if bytes.len() <= 65_536 => {}
            ServiceOperation::SocketClose { reason, code, .. }
                if reason.len() <= 123 && (*code == 1000 || (3000..=4999).contains(code)) => {}
            _ => return Err(ServiceError::Invalid),
        }
        let mut inner = self.0.borrow_mut();
        let resource = match &operation {
            ServiceOperation::StreamRead { resource }
            | ServiceOperation::SocketSend { resource, .. }
            | ServiceOperation::SocketClose { resource, .. } => *resource,
            _ => return Err(ServiceError::Invalid),
        };
        if !inner.pending.contains_key(&resource) {
            return Err(ServiceError::Invalid);
        }
        if inner.queue.len() >= MAX_SERVICE_CALLS {
            return Err(ServiceError::Busy);
        }
        inner.queue.push(ServiceCommand {
            version: SERVICE_VERSION,
            id: 0,
            operation,
        });
        Ok(())
    }
    pub fn cancel(&self, id: u64) {
        let pending = {
            let mut inner = self.0.borrow_mut();
            let pending = inner.pending.remove(&id);
            if pending.is_some() {
                let queued = inner.queue.iter().any(|c| c.id == id);
                inner.queue.retain(|c| c.id != id && !matches!(&c.operation,ServiceOperation::StreamRead{resource}|ServiceOperation::SocketSend{resource,..}|ServiceOperation::SocketClose{resource,..} if *resource==id));
                if !queued {
                    inner.cancellations.insert(id);
                }
            }
            pending
        };
        if let Some(p) = pending {
            p.scope.remove_cleanup(p.cleanup);
        }
    }
    pub fn complete_json(&self, version: u32, id: u64, json: &str) -> bool {
        if version != SERVICE_VERSION || json.len() > MAX_SERVICE_BYTES * 4 + 8192 {
            return false;
        }
        let Ok(envelope) = serde_json::from_str::<Envelope>(json) else {
            return false;
        };
        let result = match (envelope.value, envelope.error) {
            (Some(v), None) => Ok(v),
            (None, Some(e)) => Err(e),
            _ => return false,
        };
        self.complete(version, id, result)
    }
    pub fn complete(
        &self,
        version: u32,
        id: u64,
        result: Result<ServiceValue, ServiceError>,
    ) -> bool {
        if version != SERVICE_VERSION {
            return false;
        }
        let result = result.and_then(|value| {
            let valid = match &value {
                ServiceValue::Text { text } => text.len() <= MAX_SERVICE_BYTES,
                ServiceValue::Storage { value, .. } => value.len() <= MAX_SERVICE_BYTES,
                ServiceValue::File { bytes, .. } => bytes.len() <= MAX_SERVICE_BYTES,
                ServiceValue::Chunk { bytes, .. } | ServiceValue::SocketMessage { bytes, .. } => {
                    bytes.len() <= 65_536
                }
                ServiceValue::Stream { headers, .. } => {
                    headers.len() <= 64
                        && headers
                            .iter()
                            .map(|(k, v)| k.len() + v.len())
                            .sum::<usize>()
                            <= 32_768
                }
                _ => true,
            };
            if valid {
                Ok(value)
            } else {
                Err(ServiceError::TooLarge)
            }
        });
        let (callback, removed) = {
            let mut inner = self.0.borrow_mut();
            let Some(p) = inner.pending.get(&id) else {
                return false;
            };
            if !p.scope.is_alive() {
                return false;
            }
            let callback = p.callback.clone();
            let terminal = !p.persistent
                || result.is_err()
                || matches!(
                    &result,
                    Ok(ServiceValue::Chunk { done: true, .. } | ServiceValue::SocketClosed { .. })
                );
            let removed = if terminal {
                inner.pending.remove(&id)
            } else {
                None
            };
            (callback, removed)
        };
        if let Some(p) = removed {
            if p.persistent && result.is_err() {
                self.0.borrow_mut().cancellations.insert(id);
            }
            p.scope.remove_cleanup(p.cleanup);
        }
        callback(result);
        true
    }
    pub fn drain(&self) -> Vec<ServiceCommand> {
        let mut inner = self.0.borrow_mut();
        let mut commands = std::mem::take(&mut inner.cancellations)
            .into_iter()
            .map(|resource| ServiceCommand {
                version: SERVICE_VERSION,
                id: 0,
                operation: ServiceOperation::Cancel { resource },
            })
            .collect::<Vec<_>>();
        commands.append(&mut inner.queue);
        commands
    }
    pub fn pending(&self) -> usize {
        self.0.borrow().pending.len()
    }
    pub fn clipboard_write(
        &self,
        scope: &Scope,
        text: impl Into<String>,
        f: impl Fn(Result<ServiceValue, ServiceError>) + 'static,
    ) -> Result<u64, ServiceError> {
        self.call(
            scope,
            ServiceOperation::ClipboardWrite { text: text.into() },
            f,
        )
    }
    pub fn sleep(&self, scope: &Scope, delay: std::time::Duration) -> ServiceFuture {
        let state = Rc::new(RefCell::new(ServiceFutureState {
            result: None,
            waker: None,
        }));
        let out = state.clone();
        let id = self.call(
            scope,
            ServiceOperation::Timer {
                delay_ms: delay.as_millis().clamp(1, 60_000) as u32,
            },
            move |result| {
                let waker = {
                    let mut state = out.borrow_mut();
                    state.result = Some(result);
                    state.waker.take()
                };
                if let Some(w) = waker {
                    w.wake();
                }
            },
        );
        let id = match id {
            Ok(id) => Some(id),
            Err(error) => {
                state.borrow_mut().result = Some(Err(error));
                None
            }
        };
        ServiceFuture {
            client: self.clone(),
            id,
            state,
        }
    }
    pub fn read_stream(&self, id: u64) -> Result<(), ServiceError> {
        self.command(ServiceOperation::StreamRead { resource: id })
    }
    pub fn socket_send(&self, id: u64, bytes: Vec<u8>, text: bool) -> Result<(), ServiceError> {
        if bytes.len() > 65_536 {
            return Err(ServiceError::TooLarge);
        }
        self.command(ServiceOperation::SocketSend {
            resource: id,
            bytes,
            text,
        })
    }
}
struct ServiceFutureState {
    result: Option<Result<ServiceValue, ServiceError>>,
    waker: Option<std::task::Waker>,
}
pub struct ServiceFuture {
    client: ServiceClient,
    id: Option<u64>,
    state: Rc<RefCell<ServiceFutureState>>,
}
impl std::future::Future for ServiceFuture {
    type Output = Result<ServiceValue, ServiceError>;
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let result = self.state.borrow_mut().result.take();
        if let Some(result) = result {
            self.id = None;
            std::task::Poll::Ready(result)
        } else {
            self.state.borrow_mut().waker = Some(cx.waker().clone());
            std::task::Poll::Pending
        }
    }
}
impl Drop for ServiceFuture {
    fn drop(&mut self) {
        if let Some(id) = self.id.take() {
            self.client.cancel(id);
        }
    }
}

/// A scope-owned local future queue, polled after events and service completions.
/// Pending tasks do not request animation frames. Hosts pump this queue when waking.
#[derive(Clone, Default)]
pub struct TaskQueue(Rc<RefCell<TaskInner>>);
#[derive(Default)]
struct TaskInner {
    next: u64,
    tasks: HashMap<
        u64,
        (
            Scope,
            u64,
            std::pin::Pin<Box<dyn std::future::Future<Output = ()>>>,
        ),
    >,
}
impl TaskQueue {
    pub fn spawn(
        &self,
        scope: &Scope,
        future: impl std::future::Future<Output = ()> + 'static,
    ) -> Result<u64, ServiceError> {
        if !scope.is_alive() {
            return Err(ServiceError::Cancelled);
        }
        let id = {
            let mut inner = self.0.borrow_mut();
            if inner.tasks.len() >= 64 {
                return Err(ServiceError::Busy);
            }
            inner.next += 1;
            inner.next
        };
        let weak = Rc::downgrade(&self.0);
        let cleanup = scope.on_cleanup(move || {
            if let Some(inner) = weak.upgrade() {
                inner.borrow_mut().tasks.remove(&id);
            }
        });
        self.0
            .borrow_mut()
            .tasks
            .insert(id, (scope.clone(), cleanup, Box::pin(future)));
        self.poll();
        Ok(id)
    }
    pub fn poll(&self) {
        let ids = self.0.borrow().tasks.keys().copied().collect::<Vec<_>>();
        for id in ids {
            let task = self.0.borrow_mut().tasks.remove(&id);
            if let Some((scope, cleanup, mut future)) = task {
                let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
                if scope.is_alive()
                    && future.as_mut().poll(&mut cx).is_pending()
                    && scope.is_alive()
                {
                    self.0
                        .borrow_mut()
                        .tasks
                        .insert(id, (scope, cleanup, future));
                } else {
                    scope.remove_cleanup(cleanup);
                }
            }
        }
    }
    pub fn pending(&self) -> usize {
        self.0.borrow().tasks.len()
    }
}
