use rustx::prelude::*;
use std::{cell::RefCell, rc::Rc};

fn owner() -> (Runtime, Scope) {
    let out = Rc::new(RefCell::new(None));
    let copy = out.clone();
    let rt = Runtime::new(move |cx| {
        *copy.borrow_mut() = Some(cx);
        Node::text("")
    });
    let scope = out.borrow().clone().unwrap();
    (rt, scope)
}
fn ready() -> HttpClient {
    let h = HttpClient::default();
    h.connect(HostCapabilities::get_cancel()).unwrap();
    h
}
fn result() -> Result<HttpResponse, HostError> {
    Ok(HttpResponse {
        headers: vec![],
        status: 200,
        body: vec![],
    })
}

#[test]
fn handshake_rejects_skew_and_invalid_limits() {
    let h = HttpClient::default();
    let mut c = HostCapabilities::get_cancel();
    c.version += 1;
    assert!(matches!(
        h.connect(c),
        Err(HostError::VersionMismatch { .. })
    ));
    let mut c = HostCapabilities::get_cancel();
    c.max_requests = 0;
    assert_eq!(h.connect(c), Err(HostError::InvalidCapabilities));
    h.connect(HostCapabilities::get_cancel()).unwrap();
    assert_eq!(
        h.connect(HostCapabilities::get_cancel()),
        Err(HostError::AlreadyConnected)
    );
}
#[test]
fn missing_or_denied_host_never_enqueues_work() {
    let (_rt, s) = owner();
    let h = HttpClient::default();
    h.get(&s, "/users", |r| {
        assert_eq!(r.unwrap_err(), HostError::NotConnected)
    });
    for state in [
        CapabilityState::Unavailable,
        CapabilityState::Denied,
        CapabilityState::Failed,
    ] {
        let h = HttpClient::default();
        let mut c = HostCapabilities::get_cancel();
        c.http_get = state;
        h.connect(c).unwrap();
        h.get(&s, "/users", move |r| {
            assert_eq!(
                r.unwrap_err(),
                HostError::Capability {
                    service: "http_get",
                    state
                }
            )
        });
        assert_eq!((h.pending(), h.queued(), s.cleanup_count()), (0, 0, 0));
    }
    let h = HttpClient::default();
    let mut c = HostCapabilities::get_cancel();
    c.http_cancel = CapabilityState::Unavailable;
    h.connect(c).unwrap();
    h.get(&s, "/users", |r| {
        assert!(matches!(
            r,
            Err(HostError::Capability {
                service: "http_cancel",
                ..
            })
        ))
    });
}
#[test]
fn skewed_stale_and_oversized_completions_are_safe() {
    let (_rt, s) = owner();
    let h = ready();
    let calls = Rc::new(RefCell::new(0));
    let copy = calls.clone();
    let id = h.get(&s, "/users", move |r| {
        assert_eq!(r.unwrap_err(), HostError::TooLarge);
        *copy.borrow_mut() += 1;
    });
    h.drain();
    assert!(!h.complete(HOST_PROTOCOL_VERSION + 1, id, result()));
    assert_eq!(h.pending(), 1);
    assert!(h.complete(
        HOST_PROTOCOL_VERSION,
        id,
        Ok(HttpResponse {
            headers: vec![],
            status: 200,
            body: vec![0; MAX_HTTP_BYTES + 1]
        })
    ));
    assert!(!h.complete(HOST_PROTOCOL_VERSION, id, result()));
    assert_eq!(*calls.borrow(), 1);
    assert_eq!(s.cleanup_count(), 0);
}
#[test]
fn queue_and_scope_cleanup_stay_bounded_under_adversarial_churn() {
    let (rt, s) = owner();
    let h = ready();
    let first: Vec<_> = (0..MAX_HOST_REQUESTS)
        .map(|_| h.get(&s, "/users", |_| {}))
        .collect();
    h.get(&s, "/overflow", |r| {
        assert_eq!(r.unwrap_err(), HostError::Busy)
    });
    assert_eq!(h.pending(), MAX_HOST_REQUESTS);
    assert!(h
        .drain()
        .iter()
        .all(|c| matches!(c, HostCommand::Get(r) if r.version == HOST_PROTOCOL_VERSION)));
    for id in first {
        h.cancel(id);
    }
    for _ in 0..10_000 {
        let id = h.get(&s, "/users", |_| {});
        h.cancel(id);
    }
    assert_eq!(h.queued(), MAX_HOST_REQUESTS); // Only cancellations for dispatched work survive.
    for _ in 0..MAX_HOST_REQUESTS {
        h.get(&s, "/users", |_| {});
    }
    assert_eq!(h.queued(), MAX_HOST_COMMANDS);
    drop(rt);
    assert_eq!(h.pending(), 0);
    assert_eq!(h.drain().len(), MAX_HOST_REQUESTS);
    assert_eq!(s.cleanup_count(), 0);
}
#[test]
fn reentrant_callbacks_can_start_work_and_small_host_limits_apply() {
    let (_rt, s) = owner();
    let h = HttpClient::default();
    let mut caps = HostCapabilities::get_cancel();
    caps.max_requests = 1;
    caps.max_response_bytes = 8;
    caps.timeout_ms = 100;
    h.connect(caps).unwrap();
    let client = h.clone();
    let copy = s.clone();
    let id = h.get(&s, "/first", move |_| {
        client.get(&copy, "/second", |_| {});
    });
    let commands = h.drain();
    assert!(matches!(&commands[0], HostCommand::Get(r) if r.max_bytes == 8 && r.timeout_ms == 100));
    assert!(h.complete(HOST_PROTOCOL_VERSION, id, result()));
    assert_eq!(h.pending(), 1);
    assert_eq!(s.cleanup_count(), 1);
}
