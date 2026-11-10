//! Small bounded query cache: explicit clocks, no automatic mutation retries.
use crate::{HostError, HttpClient, HttpResponse, RequestBuilder, Scope, Signal};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};
#[derive(Clone, Debug, PartialEq)]
pub enum QueryState<T> {
    Idle,
    Loading,
    Success(T),
    Error(HostError),
}
struct Entry<T: 'static> {
    url: String,
    generation: u64,
    updated: u64,
    state: QueryState<T>,
    observers: HashMap<u64, Signal<QueryState<T>>>,
    pending: Option<(HttpClient, u64)>,
}
struct CacheInner<T: 'static> {
    entries: HashMap<String, Entry<T>>,
    next: u64,
    capacity: usize,
}
#[derive(Clone)]
pub struct QueryCache<T: 'static> {
    owner: Scope,
    inner: Rc<RefCell<CacheInner<T>>>,
}
impl<T: Clone + serde::de::DeserializeOwned + 'static> QueryCache<T> {
    pub fn new(owner: &Scope, capacity: usize) -> Self {
        Self {
            owner: owner.clone(),
            inner: Rc::new(RefCell::new(CacheInner {
                entries: HashMap::new(),
                next: 1,
                capacity: capacity.clamp(1, 64),
            })),
        }
    }
    /// Concurrent consumers of one key share one request. `now_ms` is a monotonic host clock.
    pub fn query(
        &self,
        scope: &Scope,
        key: impl Into<String>,
        url: impl Into<String>,
        now_ms: u64,
        stale_ms: u64,
        http: &HttpClient,
    ) -> Result<Signal<QueryState<T>>, HostError> {
        let (key, url) = (key.into(), url.into());
        if key.is_empty() || key.len() > 1024 {
            return Err(HostError::InvalidRequest);
        }
        let (state, id, load) = {
            let mut inner = self.inner.borrow_mut();
            if !inner.entries.contains_key(&key) && inner.entries.len() >= inner.capacity {
                let evict = inner
                    .entries
                    .iter()
                    .filter(|(_, e)| e.observers.is_empty() && e.pending.is_none())
                    .min_by_key(|(_, e)| e.updated)
                    .map(|(k, _)| k.clone());
                if let Some(k) = evict {
                    inner.entries.remove(&k);
                } else {
                    return Err(HostError::Busy);
                }
            }
            let id = inner.next;
            inner.next += 1;
            let entry = inner.entries.entry(key.clone()).or_insert_with(|| Entry {
                url: url.clone(),
                generation: 0,
                updated: 0,
                state: QueryState::Idle,
                observers: HashMap::new(),
                pending: None,
            });
            if entry.url != url {
                return Err(HostError::InvalidRequest);
            }
            if entry.observers.len() >= 64 {
                return Err(HostError::Busy);
            }
            let load = entry.pending.is_none()
                && (!matches!(entry.state, QueryState::Success(_))
                    || now_ms.saturating_sub(entry.updated) >= stale_ms);
            let state = scope.signal(entry.state.clone());
            entry.observers.insert(id, state.clone());
            (state, id, load)
        };
        let weak = Rc::downgrade(&self.inner);
        let cleanup_key = key.clone();
        scope.on_cleanup(move || {
            if let Some(inner) = weak.upgrade() {
                let cancel = {
                    let mut inner = inner.borrow_mut();
                    if let Some(entry) = inner.entries.get_mut(&cleanup_key) {
                        entry.observers.remove(&id);
                        if entry.observers.is_empty() {
                            entry.generation += 1;
                            let pending = entry.pending.take();
                            if pending.is_some() {
                                entry.state = QueryState::Idle;
                            }
                            pending
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                };
                if let Some((http, id)) = cancel {
                    http.cancel(id);
                }
            }
        });
        if load {
            self.refetch(&key, now_ms, http)?;
        }
        Ok(state)
    }
    pub fn refetch(&self, key: &str, now_ms: u64, http: &HttpClient) -> Result<(), HostError> {
        let (url, generation, cancel, signals) = {
            let mut inner = self.inner.borrow_mut();
            let entry = inner
                .entries
                .get_mut(key)
                .ok_or(HostError::InvalidRequest)?;
            entry.generation += 1;
            entry.state = QueryState::Loading;
            (
                entry.url.clone(),
                entry.generation,
                entry.pending.take(),
                entry.observers.values().cloned().collect::<Vec<_>>(),
            )
        };
        if let Some((client, id)) = cancel {
            client.cancel(id);
        }
        for s in signals {
            let _ = s.set(QueryState::Loading);
        }
        let weak = Rc::downgrade(&self.inner);
        let cache_key = key.to_owned();
        let id = http
            .request("GET", url)
            .send_callback(&self.owner, move |result| {
                let result = result
                    .and_then(HttpResponse::error_for_status)
                    .and_then(HttpResponse::json::<T>);
                if let Some(inner) = weak.upgrade() {
                    let update = {
                        let mut inner = inner.borrow_mut();
                        if let Some(entry) = inner.entries.get_mut(&cache_key) {
                            if entry.generation != generation {
                                return;
                            }
                            entry.pending = None;
                            entry.updated = now_ms;
                            entry.state = match result {
                                Ok(v) => QueryState::Success(v),
                                Err(e) => QueryState::Error(e),
                            };
                            Some((
                                entry.state.clone(),
                                entry.observers.values().cloned().collect::<Vec<_>>(),
                            ))
                        } else {
                            None
                        }
                    };
                    if let Some((state, signals)) = update {
                        for s in signals {
                            let _ = s.set(state.clone());
                        }
                    }
                }
            });
        let mut inner = self.inner.borrow_mut();
        if let Some(entry) = inner.entries.get_mut(key) {
            if entry.generation == generation && matches!(entry.state, QueryState::Loading) {
                entry.pending = Some((http.clone(), id));
            }
        }
        Ok(())
    }
    pub fn invalidate(&self, key: &str) -> bool {
        let update = {
            let mut inner = self.inner.borrow_mut();
            let Some(entry) = inner.entries.get_mut(key) else {
                return false;
            };
            entry.generation += 1;
            entry.state = QueryState::Idle;
            (
                entry.pending.take(),
                entry.observers.values().cloned().collect::<Vec<_>>(),
            )
        };
        if let Some((http, id)) = update.0 {
            http.cancel(id);
        }
        for s in update.1 {
            let _ = s.set(QueryState::Idle);
        }
        true
    }
    pub fn len(&self) -> usize {
        self.inner.borrow().entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
impl<T: 'static> Drop for CacheInner<T> {
    fn drop(&mut self) {
        for entry in self.entries.values_mut() {
            if let Some((http, id)) = entry.pending.take() {
                http.cancel(id);
            }
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum MutationState<T> {
    Idle,
    Pending,
    Success(T),
    Error(HostError),
}
#[derive(Clone)]
pub struct Mutation<T: 'static> {
    pub state: Signal<MutationState<T>>,
    generation: Rc<Cell<u64>>,
    pending: Rc<RefCell<Option<(HttpClient, u64)>>>,
}
impl<T: Clone + serde::de::DeserializeOwned + 'static> Mutation<T> {
    pub fn new(scope: &Scope) -> Self {
        Self {
            state: scope.signal(MutationState::Idle),
            generation: Rc::new(Cell::new(0)),
            pending: Rc::new(RefCell::new(None)),
        }
    }
    /// Mutations run once. Starting another cancels the former and rejects late completion.
    pub fn run(&self, scope: &Scope, request: RequestBuilder, http: &HttpClient) {
        self.cancel();
        let generation = self.generation.get() + 1;
        self.generation.set(generation);
        let _ = self.state.set(MutationState::Pending);
        let version = self.generation.clone();
        let state = self.state.clone();
        let pending = self.pending.clone();
        let id = request.send_callback(scope, move |result| {
            if version.get() != generation {
                return;
            }
            pending.borrow_mut().take();
            let result = result
                .and_then(HttpResponse::error_for_status)
                .and_then(HttpResponse::json::<T>);
            let _ = state.set(match result {
                Ok(v) => MutationState::Success(v),
                Err(e) => MutationState::Error(e),
            });
        });
        if matches!(self.state.peek(), Ok(MutationState::Pending)) {
            *self.pending.borrow_mut() = Some((http.clone(), id));
        }
    }
    pub fn cancel(&self) {
        self.generation.set(self.generation.get() + 1);
        let pending = self.pending.borrow_mut().take();
        if let Some((http, id)) = pending {
            http.cancel(id);
        }
        let _ = self.state.set(MutationState::Idle);
    }
}
/// Versioned persistence values are migrated in Rust before being exposed to application state.
pub fn migrate_record<T: serde::de::DeserializeOwned>(
    version: u32,
    current: u32,
    value: &str,
    migrate: impl FnOnce(u32, &str) -> Result<String, crate::ServiceError>,
) -> Result<T, crate::ServiceError> {
    if version > current || version == 0 {
        return Err(crate::ServiceError::Invalid);
    }
    let value = if version == current {
        value.to_owned()
    } else {
        migrate(version, value)?
    };
    serde_json::from_str(&value).map_err(|_| crate::ServiceError::Invalid)
}
