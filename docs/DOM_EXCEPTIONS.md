# Browser DOM exception register — RP-004

16 November 2026. Browser host ownership is specified in ADR-015. Rust owns ordinary application state, layout, text and canvas pixels. The DOM below provides bounded browser capabilities; removing it must not remove accessible content or input behavior.

| ID / source | Capability and owner/lifetime | Bound / accounting category | Remaining evidence |
| --- | --- | --- | --- |
| DOM-01 `web/index.html`, `web/host.js` | Document and singleton canvas/semantic/fatal shell; page lifetime | `shell`: 7 elements in reference document; surface replaced on recovery/disposal | Simultaneous roots in one Wasm module are unsupported; independent documents own separate hosts |
| DOM-02 `web/semantics.js` | Transparent semantic/input projection; host owns elements and listener controllers | `semantic`: scales with Rust accessible content; exact eight-route baselines below | No pooling; physical IME/AT acceptance remains open |
| DOM-03 `web/host.js` | Emergency reload UI when Rust cannot run | `emergency`: two children in fatal container; ordinary production status badge removed | Actual blocked-Wasm startup tested; not every fatal failure mode |
| DOM-04 `web/services.js` | File input, download anchor/object URL, printable popup; operation lifetime | `service`: tagged transient inputs/anchors; existing service admission bounds apply | Main-document audit excludes popup contents; real permission/print UI remains a separate gate |
| DOM-05 `web/playground.js`, `playground/preview.html` | Opaque compiled app iframe; preview adapter lifetime | `preview`: at most one frame; host disposal removes it and its message/click listeners | 100 actual-DOM adapter cycles use a stub receiver; no new compiled-preview/inner-host certification |
| DOM-06 `web/devtools.js`, inspector files | Status, highlight, optional launcher/error overlay and separate inspector page | `tooling`: two elements in default dev host; zero after dispose and in production | Inspector popup owns a separate document; not included in main-document counts |
| DOM-07 generated bundle head/worker | Metadata, resource hints, script/style loading; document lifetime | `resource`: 6 elements in measured dev shell | Production hashed resources differ; no general typed metadata API yet |
| DOM-08 future semantic/server/print exports | Full content projection remains unimplemented | No approved budget until design | Must share Rust content and avoid duplicate announcements; browser parity blocker |

## Frozen reference budgets

Run a development build at 1280×720, DPR 1, CPU renderer. `fixtures/dom-budget-chromium.json` and `fixtures/dom-budget-webkit.json` freeze exact category counts and mutation ceilings. They agree on current semantic elements:

| Route | Semantic elements |
| --- | ---: |
| `/` | 36 |
| `/people` | 21 |
| `/lab` | 26 |
| `/engine` | 99 |
| `/applications` | 67 |
| `/playground` (no compiler configured) | 18 |
| `/capabilities` | 50 |
| `/store` | 13 |

Each has shell 7, resource 6 and tooling 2; service, preview, emergency and unregistered counts are zero in these settled workloads. After root disposal only shell 7 and resource 6 remain. This is not a universal maximum for applications, a matched React benchmark, or proof that optional service/preview branches allocate nothing.

One `/store` publication has zero added/removed elements. Attribute/text/child-list mutation ceilings use the recorded baseline plus 10% rounded up and 10 records where nonzero, to allow bounded browser bookkeeping variance. Addition/removal ceilings have no slack. Initial creation records and 100-cycle results are retained separately. No semantic diff/pooling optimization was made to obtain these numbers.

`?dom-audit=1` on a devtools build exposes `window.rustx.dom()`: main-document live element counts, cumulative MutationObserver record counts by category, and any semantic elements whose own computed opacity is nonzero. The observer is page-owned across host restarts. It drains records into numeric totals and retains no removed nodes or text. Attribute records count writes, not distinct value changes; textContent updates can appear as child-list records. Transient service/emergency nodes are counted even if detached before delivery. Unknown elements are reported as `unregistered`; this is diagnostic accounting, not a security boundary or a complete visual-paint audit. Iframe/popup interiors, GPU/native resources and browser heap memory are outside this counter.

## Ownership and enforcement

`window.rustx.dispose()` is dev-only, idempotent and immediately releases host registrations, semantic nodes, network admission entries, service resources and tool/preview DOM. Pending GPU initialization, if present, is awaited before `restart()` mounts a replacement. Concurrent restarts share one promise; captured old controls/development methods cannot call the new runtime. Rust app disposal is exported by both the lab and the shared scaffold macro. The page shell and foreign semantic-root children remain document-owned.

Persisted `pagehide` pauses painting and keeps Rust/semantic state; persisted `pageshow` invalidates/resizes/resumes. Non-persisted `pagehide` disposes. Synthetic lifecycle checks pass, but actual back navigation in both tested engines created a fresh document: BFCache restoration/eligibility is **unverified**, including live services and embedded tools. No unload hook or blanket BFCache support claim is introduced.

After `bash scripts/build-web.sh`, serve `dist` on port 3000 and run:

```sh
npm run test:dom:budget
npm run test:host:ownership
RUSTX_BROWSER=webkit npm run test:dom:budget
RUSTX_BROWSER=webkit npm run test:host:ownership
# Build production, serve it on 3011, then:
RUSTX_URL=http://127.0.0.1:3011 npm run test:dom:production
```

Budget checks reject unexplained count changes, ordinary unregistered DOM, visible semantic projection elements and excessive mutation records. They execute 100 route cycles and 100 full Rust root dispose/restart cycles, not mocked application trees. The ownership runner also tests pending HTTP cancellation and isolated preview-adapter cleanup. Linux Chromium CI invokes these checks and the production test; its new run result must be read from CI, not inferred from local macOS success. Only intentionally re-baseline with `RUSTX_RECORD_DOM=1` after reviewing a changed workload, updating the register and retaining old evidence. Never raise bounds merely to hide a regression or disable accessibility to meet them.

Actual evidence and verification limits: [RP-004](evidence/react-parity/RP-004/README.md). RP-002's foreign-child/listener lifecycle checks remain binding. Physical screen-reader, OS IME, mobile and multi-root acceptance remain separate work.
