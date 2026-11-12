use rustx::prelude::*;
use std::{cell::RefCell, rc::Rc, time::Duration};
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
fn http() -> HttpClient {
    let http = HttpClient::default();
    http.connect(HostCapabilities::get_cancel()).unwrap();
    http
}
fn response(body: &str) -> Result<HttpResponse, HostError> {
    Ok(HttpResponse {
        status: 200,
        headers: vec![],
        body: body.as_bytes().to_vec(),
    })
}
#[test]
fn request_policy_future_drop_and_scoped_tasks() {
    let (rt, scope) = owner();
    let h = http();
    let tasks = TaskQueue::default();
    let out = scope.signal(None);
    let copy = out.clone();
    let request = h
        .request("POST", "/api/echo")
        .header("x-test", "safe")
        .json(&vec![1, 2])
        .timeout(Duration::from_millis(120))
        .response_limit(40)
        .send(&scope);
    tasks
        .spawn(&scope, async move {
            let _ = copy.set(Some(request.await.unwrap().json::<Vec<i32>>().unwrap()));
        })
        .unwrap();
    assert!(
        matches!(&h.drain()[0],HostCommand::Request(r) if r.method=="POST"&&r.timeout_ms==120&&r.max_bytes==40&&r.credentials==Credentials::SameOrigin)
    );
    h.complete(HOST_PROTOCOL_VERSION, 1, response("[3,4]"));
    tasks.poll();
    assert_eq!(out.get(), Some(vec![3, 4]));
    assert_eq!(tasks.pending(), 0);
    let dropped = h.request("GET", "/never").send(&scope);
    h.drain();
    drop(dropped);
    assert!(matches!(h.drain()[0], HostCommand::Cancel { .. }));
    let future = h.request("GET", "/unmount").send(&scope);
    tasks
        .spawn(&scope, async move {
            let _ = future.await;
        })
        .unwrap();
    h.drain();
    drop(rt);
    assert_eq!(h.pending(), 0);
    assert_eq!(tasks.pending(), 0);
    assert!(scope.cleanup_count() == 0);
}
#[test]
fn invalid_headers_methods_status_decode_and_diagnostics() {
    let (_rt, scope) = owner();
    let h = http();
    let out = Rc::new(RefCell::new(None));
    let copy = out.clone();
    h.request("TRACE", "/secret?token=private")
        .header("authorization", "secret\r\ninjected: yes")
        .send_callback(&scope, move |r| *copy.borrow_mut() = Some(r.unwrap_err()));
    assert_eq!(*out.borrow(), Some(HostError::InvalidRequest));
    assert_eq!(h.pending(), 0);
    assert!(h.drain().is_empty());
    let r = HttpResponse {
        status: 503,
        headers: vec![],
        body: b"secret-password".to_vec(),
    };
    assert!(!format!("{r:?}").contains("secret"));
    assert_eq!(r.error_for_status().unwrap_err(), HostError::Http(503));
    assert!(matches!(
        HttpResponse {
            status: 200,
            headers: vec![],
            body: vec![0xff]
        }
        .text(),
        Err(HostError::Decode(_))
    ));
}
#[test]
fn services_reject_stale_skew_and_bound_scope_resources() {
    let (rt, scope) = owner();
    let client = ServiceClient::default();
    let log = Rc::new(RefCell::new(vec![]));
    let copy = log.clone();
    let id = client
        .subscribe(
            &scope,
            ServiceOperation::SocketOpen {
                url: "/api/socket".into(),
                protocols: vec![],
            },
            move |r| copy.borrow_mut().push(r),
        )
        .unwrap();
    client.drain();
    assert!(!client.complete_json(2, id, r#"{"value":{"type":"socket_open"}}"#));
    assert!(client.complete_json(1, id, r#"{"value":{"type":"socket_open"}}"#));
    assert_eq!(client.pending(), 1);
    client.complete(
        1,
        id,
        Ok(ServiceValue::SocketMessage {
            bytes: vec![0; 65_537],
            text: false,
        }),
    );
    assert_eq!(client.pending(), 0);
    assert_eq!(log.borrow().last(), Some(&Err(ServiceError::TooLarge)));
    // Terminal protocol errors must release the host's persistent resource before
    // its slot can be reused. Pump that cancellation before filling all 32 slots.
    let cancelled = client.drain();
    assert_eq!(cancelled.len(), 1);
    assert!(
        matches!(cancelled[0].operation, ServiceOperation::Cancel { resource } if resource == id)
    );
    for _ in 0..32 {
        client
            .subscribe(&scope, ServiceOperation::WatchLocation, |_| {})
            .unwrap();
    }
    assert_eq!(
        client.subscribe(&scope, ServiceOperation::WatchLocation, |_| {}),
        Err(ServiceError::Busy)
    );
    assert_eq!(client.drain().len(), 32);
    drop(rt);
    assert_eq!(client.pending(), 0);
    assert_eq!(client.drain().len(), 32);
    assert!(!client.complete(1, id, Ok(ServiceValue::SocketOpen)));
}
#[test]
fn query_dedup_stale_supersede_and_invalidation() {
    let (_rt, scope) = owner();
    let h = http();
    let cache = QueryCache::<Vec<i32>>::new(&scope, 2);
    let a = cache.query(&scope, "numbers", "/n", 100, 1000, &h).unwrap();
    let b = cache.query(&scope, "numbers", "/n", 100, 1000, &h).unwrap();
    assert_eq!(h.pending(), 1);
    assert_eq!(h.drain().len(), 1);
    h.complete(1, 1, response("[1]"));
    assert_eq!(a.get(), QueryState::Success(vec![1]));
    assert_eq!(b.get(), QueryState::Success(vec![1]));
    cache.query(&scope, "numbers", "/n", 200, 1000, &h).unwrap();
    assert_eq!(h.pending(), 0);
    cache.refetch("numbers", 1200, &h).unwrap();
    h.drain();
    cache.refetch("numbers", 1300, &h).unwrap();
    h.drain();
    assert!(!h.complete(1, 2, response("[2]")));
    h.complete(1, 3, response("[3]"));
    assert_eq!(a.get(), QueryState::Success(vec![3]));
    assert!(cache.invalidate("numbers"));
    assert_eq!(a.get(), QueryState::Idle);
    assert!(cache
        .query(&scope, "numbers", "/other", 1400, 1000, &h)
        .is_err());
}
#[test]
fn typed_routes_percent_queries_hash_and_not_found() {
    #[derive(Clone, Debug, PartialEq, Eq)]
    enum Page {
        Home,
        Person,
        Files,
    }
    let routes = vec![
        ("/".into(), Page::Home),
        ("/people/:id".into(), Page::Person),
        ("/files/*path".into(), Page::Files),
    ];
    let RouteState::Found(route) = match_location(
        "https://example.test/#/people/%D8%B9%D9%84%DB%8C?q=hello+world&q=2",
        RouteMode::Hash,
        &routes,
    ) else {
        panic!()
    };
    assert_eq!(route.route, Page::Person);
    assert_eq!(route.params["id"], "علی");
    assert_eq!(
        route.query,
        vec![("q".into(), "hello world".into()), ("q".into(), "2".into())]
    );
    assert!(matches!(
        match_location("/people/%FF", RouteMode::History, &routes),
        RouteState::NotFound(_)
    ));
    let RouteState::Found(route) = match_location("/files/a/b", RouteMode::History, &routes) else {
        panic!()
    };
    assert_eq!(route.params["path"], "a/b");
}
#[test]
fn storage_schema_and_secret_command_redaction() {
    let value = migrate_record::<Vec<i32>>(1, 2, "1", |version, v| {
        assert_eq!(version, 1);
        Ok(format!("[{v}]"))
    })
    .unwrap();
    assert_eq!(value, vec![1]);
    assert_eq!(
        migrate_record::<Vec<i32>>(3, 2, "[]", |_, _| panic!()),
        Err(ServiceError::Invalid)
    );
    let command = ServiceCommand {
        version: 1,
        id: 2,
        operation: ServiceOperation::StorageSet {
            key: "private".into(),
            version: 2,
            value: "secret".into(),
        },
    };
    let json = serde_json::to_value(&command).unwrap();
    assert_eq!(json["version"], 1);
    assert_eq!(json["schema_version"], 2);
    assert!(!format!("{command:?}").contains("secret"));
}

#[test]
fn scheduling_future_cancels_and_capability_denial_stays_distinct() {
    use std::future::Future;
    let (_rt, scope) = owner();
    let client = ServiceClient::default();
    client
        .connect(ServiceCapabilities {
            version: 1,
            clipboard: CapabilityState::Denied,
            files: CapabilityState::Unavailable,
            storage: CapabilityState::Supported,
            navigation: CapabilityState::Supported,
            websocket: CapabilityState::Supported,
            stream: CapabilityState::Supported,
            print: CapabilityState::Supported,
            scheduling: CapabilityState::Supported,
            max_bytes: MAX_SERVICE_BYTES,
            max_resources: 32,
        })
        .unwrap();
    assert_eq!(
        client.clipboard_write(&scope, "private", |_| panic!()),
        Err(ServiceError::Denied)
    );
    let mut future = Box::pin(client.sleep(&scope, Duration::from_millis(20)));
    let command = client.drain().pop().unwrap();
    assert!(matches!(
        command.operation,
        ServiceOperation::Timer { delay_ms: 20 }
    ));
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    assert!(future.as_mut().poll(&mut cx).is_pending());
    assert!(client.complete(1, command.id, Ok(ServiceValue::Unit)));
    assert_eq!(
        future.as_mut().poll(&mut cx),
        std::task::Poll::Ready(Ok(ServiceValue::Unit))
    );
    let future = client.sleep(&scope, Duration::from_secs(1));
    client.drain();
    drop(future);
    assert!(matches!(
        client.drain()[0].operation,
        ServiceOperation::Cancel { .. }
    ));
}
