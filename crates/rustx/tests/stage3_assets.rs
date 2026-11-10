use rustx::prelude::*;
use std::{cell::RefCell, rc::Rc};

fn entry() -> AssetEntry {
    AssetEntry {
        id: "icons/logo".into(),
        url: "/assets/logo.png?theme=dark".into(),
        revision: "sha256-ab12".into(),
        kind: AssetKind::PngImage,
        max_bytes: 4,
    }
}

#[test]
fn asset_manifest_rejects_ambiguous_names_and_versions_fetch_urls() {
    let manifest = AssetManifest::new(vec![entry()]).unwrap();
    assert_eq!(manifest.len(), 1);
    assert_eq!(
        manifest.request_url("icons/logo").unwrap(),
        "/assets/logo.png?theme=dark&rustx_revision=sha256-ab12"
    );
    assert_eq!(manifest.request_url("missing"), Err(AssetError::UnknownId));
    assert!(matches!(
        AssetManifest::new(vec![entry(), entry()]),
        Err(AssetError::DuplicateId)
    ));
    for id in [
        "../logo",
        "/logo",
        "icons//logo",
        "icons/./logo",
        "bad name",
    ] {
        let mut asset = entry();
        asset.id = id.into();
        assert!(matches!(
            AssetManifest::new(vec![asset]),
            Err(AssetError::InvalidManifest)
        ));
    }
    let mut asset = entry();
    asset.revision = "v1&override=true".into();
    assert!(matches!(
        AssetManifest::new(vec![asset]),
        Err(AssetError::InvalidManifest)
    ));
    let mut asset = entry();
    asset.max_bytes = MAX_HTTP_BYTES + 1;
    assert!(matches!(
        AssetManifest::new(vec![asset]),
        Err(AssetError::InvalidManifest)
    ));
}

#[test]
fn asset_load_preserves_identity_bounds_payload_and_cancels_on_disposal() {
    let saved = Rc::new(RefCell::new(None));
    let out = saved.clone();
    let runtime = Runtime::new(move |cx| {
        *out.borrow_mut() = Some(cx);
        Node::text("")
    });
    let scope = saved.borrow().clone().unwrap();
    let http = HttpClient::default();
    http.connect(HostCapabilities::get_cancel()).unwrap();
    let manifest = AssetManifest::new(vec![entry()]).unwrap();
    let result = Rc::new(RefCell::new(None));
    let out = result.clone();
    let id = manifest
        .load(&http, &scope, "icons/logo", move |r| {
            *out.borrow_mut() = Some(r)
        })
        .unwrap();
    assert!(
        matches!(&http.drain()[0], HostCommand::Request(r) if r.max_bytes == 4 && r.url.ends_with("rustx_revision=sha256-ab12"))
    );
    http.complete(
        HOST_PROTOCOL_VERSION,
        id,
        Ok(HttpResponse {
            status: 200,
            headers: vec![],
            body: vec![1, 2],
        }),
    );
    let data = result.borrow_mut().take().unwrap().unwrap();
    assert_eq!(data.entry.id, "icons/logo");
    assert_eq!(data.entry.revision, "sha256-ab12");
    assert_eq!(data.bytes, [1, 2]);
    let out = result.clone();
    let id = manifest
        .load(&http, &scope, "icons/logo", move |r| {
            *out.borrow_mut() = Some(r)
        })
        .unwrap();
    http.drain();
    http.complete(
        HOST_PROTOCOL_VERSION,
        id,
        Ok(HttpResponse {
            status: 200,
            headers: vec![],
            body: vec![0; 5],
        }),
    );
    assert!(matches!(
        result.borrow().as_ref(),
        Some(Err(AssetError::Transport(HostError::TooLarge)))
    ));
    let id = manifest
        .load(&http, &scope, "icons/logo", |_| {
            panic!("disposed load completed")
        })
        .unwrap();
    http.drain();
    drop(runtime);
    assert!(
        matches!(&http.drain()[0], HostCommand::Cancel { id: cancelled, .. } if *cancelled == id)
    );
    assert!(!http.complete(
        HOST_PROTOCOL_VERSION,
        id,
        Ok(HttpResponse {
            status: 200,
            headers: vec![],
            body: vec![]
        })
    ));
}
