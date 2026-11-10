//! Typed route matching independent of browser DOM and history implementations.
use crate::{Scope, ServiceClient, ServiceError, ServiceOperation, ServiceValue, Signal};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RouteMode {
    History,
    Hash,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouteMatch<T> {
    pub route: T,
    pub params: BTreeMap<String, String>,
    pub query: Vec<(String, String)>,
    pub fragment: String,
    pub path: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RouteState<T> {
    Found(RouteMatch<T>),
    NotFound(String),
    Error(ServiceError),
}
#[derive(Clone)]
pub struct Router<T: Clone + 'static> {
    pub current: Signal<RouteState<T>>,
    scope: Scope,
    services: ServiceClient,
    mode: RouteMode,
    guard: Rc<RefCell<Option<Rc<dyn Fn(&str) -> bool>>>>,
}
impl<T: Clone + 'static> Router<T> {
    pub fn new(
        scope: &Scope,
        services: ServiceClient,
        mode: RouteMode,
        routes: Vec<(String, T)>,
    ) -> Result<Self, ServiceError> {
        if routes.len() > 128
            || routes
                .iter()
                .any(|(p, _)| !p.starts_with('/') || p.len() > 1024)
        {
            return Err(ServiceError::Invalid);
        }
        let current = scope.signal(RouteState::NotFound(String::new()));
        let out = current.clone();
        services.subscribe(scope, ServiceOperation::WatchLocation, move |result| {
            let state = match result {
                Ok(ServiceValue::Location { url }) => match_location(&url, mode, &routes),
                Err(error) => RouteState::Error(error),
                _ => RouteState::Error(ServiceError::Invalid),
            };
            let _ = out.set(state);
        })?;
        Ok(Self {
            current,
            scope: scope.clone(),
            services,
            mode,
            guard: Rc::new(RefCell::new(None)),
        })
    }
    /// Guards run synchronously before pushState; browser Back cannot always be cancelled.
    pub fn set_guard(&self, guard: impl Fn(&str) -> bool + 'static) {
        *self.guard.borrow_mut() = Some(Rc::new(guard));
    }
    pub fn navigate(&self, url: impl Into<String>, replace: bool) -> Result<u64, ServiceError> {
        let url = url.into();
        if !url.starts_with('/') || url.starts_with("//") || url.len() > 8192 {
            return Err(ServiceError::Invalid);
        }
        if self.guard.borrow().as_ref().is_some_and(|g| !g(&url)) {
            return Err(ServiceError::Cancelled);
        }
        let out = self.current.clone();
        self.services.call(
            &self.scope,
            ServiceOperation::Navigate {
                url,
                replace,
                hash: self.mode == RouteMode::Hash,
            },
            move |result| {
                if let Err(error) = result {
                    let _ = out.set(RouteState::Error(error));
                }
            },
        )
    }
    pub fn back(&self) -> Result<u64, ServiceError> {
        self.services
            .call(&self.scope, ServiceOperation::History { delta: -1 }, |_| {})
    }
    pub fn forward(&self) -> Result<u64, ServiceError> {
        self.services
            .call(&self.scope, ServiceOperation::History { delta: 1 }, |_| {})
    }
}
fn decode(value: &str, plus: bool) -> Option<String> {
    let mut out = vec![];
    let mut bytes = value.bytes();
    while let Some(b) = bytes.next() {
        if b == b'%' {
            let a = (bytes.next()? as char).to_digit(16)?;
            let c = (bytes.next()? as char).to_digit(16)?;
            out.push((a * 16 + c) as u8);
        } else {
            out.push(if plus && b == b'+' { b' ' } else { b });
        }
    }
    String::from_utf8(out).ok()
}
pub fn match_location<T: Clone>(
    url: &str,
    mode: RouteMode,
    routes: &[(String, T)],
) -> RouteState<T> {
    let relative = if mode == RouteMode::Hash {
        url.split_once('#').map_or("/", |(_, r)| r)
    } else {
        if let Some((_, rest)) = url.split_once("://") {
            rest.find('/').map_or("/", |i| &rest[i..])
        } else {
            url
        }
    };
    let (base, fragment) = relative.split_once('#').unwrap_or((relative, ""));
    let (path, query) = base.split_once('?').unwrap_or((base, ""));
    let path = if path.is_empty() { "/" } else { path };
    if path.len() > 8192 {
        return RouteState::NotFound(path.into());
    }
    let segments = path
        .trim_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .map(|s| decode(s, false))
        .collect::<Option<Vec<_>>>();
    let Some(segments) = segments else {
        return RouteState::NotFound(path.into());
    };
    let query = query
        .split('&')
        .filter(|s| !s.is_empty())
        .map(|s| {
            let (k, v) = s.split_once('=').unwrap_or((s, ""));
            Some((decode(k, true)?, decode(v, true)?))
        })
        .collect::<Option<Vec<_>>>();
    let Some(query) = query else {
        return RouteState::NotFound(path.into());
    };
    for (pattern, route) in routes {
        let expected = pattern
            .trim_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();
        let mut params = BTreeMap::new();
        let mut matched = true;
        for (i, p) in expected.iter().enumerate() {
            if let Some(name) = p.strip_prefix('*') {
                if i + 1 != expected.len() {
                    matched = false;
                } else {
                    params.insert(name.into(), segments[i.min(segments.len())..].join("/"));
                }
                break;
            }
            let Some(actual) = segments.get(i) else {
                matched = false;
                break;
            };
            if let Some(name) = p.strip_prefix(':') {
                params.insert(name.into(), actual.clone());
            } else if p != actual {
                matched = false;
                break;
            }
        }
        if matched
            && (expected.len() == segments.len()
                || expected.last().is_some_and(|p| p.starts_with('*')))
        {
            return RouteState::Found(RouteMatch {
                route: route.clone(),
                params,
                query,
                fragment: decode(fragment, false).unwrap_or_default(),
                path: path.into(),
            });
        }
    }
    RouteState::NotFound(path.into())
}
