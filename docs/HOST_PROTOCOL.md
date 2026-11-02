# Stage 1 host protocol, version 1

The portable `HttpClient` has no ambient host. Connect once with `HostCapabilities` before mounting code that requests HTTP. The browser boot supplies `HostNetwork.capabilities()` to `init_app`; the native lab supplies the same versioned GET/cancel contract before component setup. Reconnecting requires a new client/session.

The report has `version`, `http_get`, `http_cancel`, `max_requests`, `max_response_bytes`, and `timeout_ms`. Capability states are `supported`, `unavailable`, `denied` and `failed`. Unknown report fields, incompatible versions, zero limits and limits above the framework maxima are rejected. Unavailable/denied/failed GET or cancellation produces a typed error before queuing work. Capability declarations are API discipline, not a sandbox for malicious dependencies.

Every `Get` and `Cancel` envelope carries version 1 and an exact positive JavaScript integer ID. Completion carries the version separately through the generated Wasm binding. Wrong-version completion is ignored without consuming the pending request; stale/canceled/disposed IDs cannot invoke callbacks. IDs are monotonic within the client, never reused after scope removal. Creating a new session requires disposing the old host adapter.

Limits:

| Resource | Maximum |
| --- | --- |
| Live Rust requests / active JS transports | 16 |
| Rust queued commands / JS command batch | 32 |
| UTF-8 URL bytes | 8,192 |
| Response body bytes per request | 1,048,576 |
| Request timeout | 10,000 ms |
| Native result channel | 16 results |

A host may advertise smaller positive request/body/timeout limits. Rust checks admission before allocating a pending callback/cleanup. Canceling an undispatched GET removes it; canceling dispatched work queues a versioned cancel. At most 16 dispatched cancellations and 16 newly queued GETs can exist together. Repeated start/cancel cycles without draining do not grow the queue. Queue saturation returns `HostError::Busy`; disposal always removes pending callbacks. Body size is checked by both adapter and Rust completion handling.

The JS adapter validates envelope versions, safe IDs, batch and per-request bounds, rejects duplicate active IDs, and invokes browser Fetch with the browser global receiver. It releases transport bookkeeping before invoking completion, permitting reentrant requests. Disposing the adapter aborts active transports and suppresses later callbacks.

Native workers use a bounded result channel and an active-worker limit. Cancellation rejects late results; blocking native I/O can continue until its timeout. This limitation is explicit and is not equivalent to browser AbortController cancellation.

Only GET and cancellation are implemented. POST, headers, uploads, streaming, WebSockets and other host services are outside this Stage 1 slice; no stub claims support for them. They remain Stage 3 work. Tests: `cargo test --workspace --locked`, `npm run test:host`, `npm run test:wasm`, `npm run test:http`, and the actual browser suite.
