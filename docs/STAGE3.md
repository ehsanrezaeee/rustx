# Stage 3 — Usable applications

Updated 9 November 2026. Implementation work is on `stage-3`, as requested. The source, lockfiles, test scripts and rebuilt browser application are delivered together. [ACCEPTANCE.md](ACCEPTANCE.md) records executed checks and remaining target gates; source implementation does not establish Windows, real IME or screen-reader success.

## Run

```sh
bash scripts/build-web.sh
cargo run -p rustx-cli --locked -- serve dist 3000
```

Open `http://127.0.0.1:3000/?renderer=cpu#/applications`. The Applications page is a shared Rust dashboard with a Persian/English profile form, password field, multiline editor, validation, dialog, controlled preferences, saved drafts, import/export and connected service operations. Other routes are `#/`, `#/people`, `#/lab` and `#/engine`. The People page adds Rust search, selected details, copy and printable export. The drawing board adds an edit mode: select an object, enable editing, then drag or use arrow keys. Normal mode continues to pan the camera.

```sh
RUSTX_DEMO=applications cargo run -p rustx-lab --bin rustx-lab --locked
```

The native executable uses the same app with native services. Run the fixture server on port 3000 for native relative HTTP/WebSocket URLs. `RUSTX_RENDERER=cpu` forces software rendering. Windows build instructions remain in [PLATFORM_CHECKS.md](PLATFORM_CHECKS.md).

## Events and focus

`Node::on_event` and `on_event_capture` register typed `EventKind` handlers. The runtime clones a committed ancestor path and dispatches capture, target and bubble synchronously. `prevent_default`, propagation stopping and pointer capture/release affect that dispatch; changing a signal does not change an event's path midway through dispatch. Pointer ownership is released on up/cancel or node disposal. Browser default cancellation occurs during the originating browser event.

```rust
Node::widget(Widget::Button)
    .on_pointer_down(|event| event.capture_pointer())
    .on_click(|_| { /* application action */ })
```

Controls share disabled/read-only rules and semantic state. Tab follows committed order. Modal overlays trap focus and restore the opener; Escape belongs to the overlay handler. Tabs, menus and radio groups support arrow navigation; sliders support arrows, Home/End and Page keys. `ScrollState`/`scroll_view` consume nested wheel/key scrolling until their boundary, then let it chain outward. The browser preserves native shortcuts and single activation across semantic controls and canvas input.

Canvas picking is a pointer-up default action: cancel it synchronously with `on_pointer_up` and `prevent_default`. A later click handler cannot undo an already delivered pick. Pointer events distinguish mouse, touch, pen and unknown input; `data.x/y` are viewport logical coordinates and `local_x/local_y` use each current target's committed Rust box.

`FileDrop` carries `EventData.files: Vec<DroppedFileMetadata>` with `name`, optional `mime` and optional byte `size`. It carries no contents or absolute paths and never reads dropped-file bytes. Explicit file selection remains the content-import API. Browser drops target their projected Rust node synchronously; canvas/native drops use the hit target, then focus, then the application root. Browser file navigation is always prevented. At most 16 metadata entries are admitted; names are at most 1024 UTF-8 bytes without path separators/control characters, MIME values at most 256 printable ASCII bytes, and known sizes at most the safe integer limit. Oversized/invalid metadata is rejected rather than truncated. Native MIME/size can be unavailable. Debug projections redact filenames.

`Resize` reports logical `width`, `height` and platform `scale` at the application root. `Scroll` reports the actual committed `delta_y` at its Rust scroll owner (or application root for viewport scrolling); listen with `on_event(EventKind::Scroll, ...)`. Initial positions, cancelled wheels, clamped boundaries and idle frames produce no scroll notification. `ResourceError` carries a bounded `ResourceFailure { source, code, status }`: source is HTTP/service/renderer and code is a stable identifier without URLs, paths, payloads or host messages. Adapters report accepted transport/timeout/size/service failures and renderer recovery. Intentional cancellation and stale completions do not raise another error event. HTTP status responses, including 503, remain responses until the application's explicit status policy is applied.

## Finite widget library

All blueprint widget names are accepted by `rustx!` and `Widget`: layout/content, action/navigation, form, feedback/overlay, collection and drawing controls. They lower to the existing Rust primitives, with explicit roles and `ControlState` rather than HTML visual layout.

Use the stateful public helpers for composed behavior: `tabs`/`TabPanel`, `radio_group`, `select_with_id`, `combobox_with_id`, `OverlayState` with `dialog`/`popover`, `tooltip`, `ToastQueue`/`toast_region`, `TableState`/`data_table` and `ScrollState`/`scroll_view`. The select and searchable combobox popups are retained Rust nodes. Table sorting/selection is scoped state; the existing fixed-height `VirtualListState` remains the bounded large-collection API. A raw widget node exposes controlled props/events for custom composition; helpers supply stateful patterns.

`ControlState` carries role-specific values, checked/selected/expanded, range/step, disabled/read-only/required/invalid, labels/descriptions/controls, language/direction, heading level, live announcements and table sorting. Transparent browser controls receive those properties and Rust rectangles; browser measurement does not determine visible geometry.

## Text editing and forms

`Editor` retains UTF-8 selection, maps browser UTF-16 offsets, respects grapheme boundaries, owns bounded undo/redo, and groups composition updates into one undo step. It provides selection, copy/cut/paste, insertion/deletion, movement, composition commit/cancel and explicit password/read-only configuration. `Engine` uses shared shaped clusters for visual bidi caret movement; both renderers paint selection, caret and preedit decorations. Browser beforeinput/input/selection/composition events update the same model. Native key and IME events use that model too.

`Editor::move_word` moves between Unicode word starts. Focused editors handle Ctrl+Left/Right and Ctrl+Home/End; Up/Down use shared shaped lines, retain the preferred horizontal position and support Shift selection across wrapped/blank lines. Existing Home/End move to whole-text boundaries. Alt/Meta arrow behavior remains platform-reserved; native macOS Option-word behavior is not claimed.

Ordinary text can opt into selection with `Node::text("Copy this text").selectable(true)` or `<Paragraph selectable={true}>`. It keeps its Rust text geometry and retained read-only selection model. The browser projects one transparent read-only multiline control for platform selection/copy; the visible text and highlights remain Rust-painted. The People details view exercises this API. Default text stays outside the Tab order; selectable text participates in keyboard selection and copy. Selection is per node; cross-node document selection and rich text editing are outside this API.

Passwords are masked in snapshots, semantic values and inspector output. Actual values are available only through explicit model reads and the editing bridge, and are excluded from the profile draft/export. Autofill, password-manager and real IME/mobile behavior require target-specific verification; transparent native input presence alone is insufficient evidence.

```rust
let form = FormState::new(&cx);
let name = form.field("name", "").validator(required("Enter your name"));
let ticket = name.begin_validation()?;
// A later edit/reset makes this ticket stale.
name.finish_validation(ticket, None);
```

Forms track dirty/touched, field/form errors, validation/submission state and controlledness. Reset restores initial values and invalidates pending generations. Async validation and submission tickets bind to the field/form owner and generation. Submitting with invalid fields or pending validation fails. Changing controlledness reports an error. Sensitive form fields redact diagnostics and serialization.

## HTTP, resources and services

Legacy `HttpClient::get(&scope, url, callback)` remains compatible. General requests support GET/HEAD/POST/PUT/PATCH/DELETE/OPTIONS, bounded headers and bodies, credentials policy, multipart, response headers, text/JSON/bytes decoding, explicit status checks and per-request timeout/response limits.

```rust
let response = http.request("POST", "/api/echo")
    .json(&payload)
    .send(&cx)
    .await?
    .error_for_status()?;
let text = response.text()?;
```

`TaskQueue::spawn` owns futures in a scope; hosts poll on events/completions without continuous rendering. Dropping an unresolved HTTP future cancels its request. Scope disposal and query generations reject late results. Status responses remain responses until `error_for_status`. Transport uncertainty, timeout, cancellation, size and decode failures are separate. Diagnostics omit payloads and authorization headers. Upload progress is not fabricated.

`QueryCache<T>` has bounded key/observer capacity, explicit monotonic clock/stale time, concurrent request deduplication, invalidation and refetch. `Mutation<T>` cancels superseded work and never automatically retries a mutation. Native and browser adapters share request semantics; native networking does not implement browser CORS/cookie policy.

`ServiceClient` provides typed, scope-owned operations and distinct unavailable/denied/quota/invalid/size/cancel/failure results. Browser adapters cover IndexedDB, navigation/history/hash, clipboard, explicit file selection/download, printable text export, pull Fetch streams and WebSocket lifecycle. Native adapters cover platform clipboard/file dialogs, versioned local records, in-app history, streaming and WebSocket connections. Native capabilities depend on actual OS facilities and must be exercised separately. Scoped one-shot timers and `ServiceClient::sleep` provide scheduling without perpetual rendering. See [STAGE3_SERVICES.md](STAGE3_SERVICES.md) for the exact protocol, native dependencies and service limits.

Streams emit only after `read_stream(resource)` and apply bounded chunk/total sizes. WebSocket messages and outgoing queued bytes are bounded; close/disposal releases connections. Reconnect is an application policy. Services requiring activation are pumped from the input event. File, clipboard or print denial is an error, not a successful no-op.

`Router<T>` matches typed route values, parameters, Unicode percent-encoded segments/query/fragment and not-found routes. Navigation guards run synchronously before push/replace; browsers cannot guarantee cancellation of Back. Hash mode works without server rewrites; the development server supports extensionless SPA paths. `migrate_record` applies explicit versioned Rust migrations before decoding persisted state.

`AssetManifest` validates up to 256 logical IDs and versioned URLs. `load` fetches a scope-owned bounded payload through `HttpClient`, with revision-based cache invalidation. `rustx_engine::assets::AssetRegistry` installs shared PNG images/fonts and removes images. PNG decoding expands supported palette/grayscale/RGB/16-bit inputs into RGBA8, rejects animated PNG, limits dimensions to 4096 and decoded pixels to 8 Mi pixels/32 MiB, then applies the renderer's image budget. Font data remains engine-owned; replacing/removing an installed font requires a fresh engine. Revisions support cache invalidation; they are not an integrity check. Generic byte assets are returned to the application rather than installed in the renderer.

## Resource limits and platform gates

Editing values are at most 1 MiB; undo retains at most 128 entries/8 MiB. HTTP remains at most 16 active requests, 32 queued commands, 1 MiB response/body, 8 KiB URL and 10-second timeout. Service calls/resources are bounded at 32, bytes at 1 MiB, stream chunks/WebSocket messages at 64 KiB and socket outgoing buffering at 256 KiB. Query caches are at most 64 keys and 64 observers per key. Rendering and virtualization limits from [STAGE2.md](STAGE2.md) still apply.

Browser accessibility projection and the native AccessKit adapter are implementation paths. Required NVDA/Windows browser, VoiceOver/Safari and NVDA/native Windows form tasks remain manual gates until actually run. Real Windows execution, real OS IME, Safari/mobile, full native service permission/error matrices, renderer conformance and fixed-hardware performance acceptance are not established by deterministic tests or compilation. Stage 4 editor/tooling/release and the real source playground remain separate requirements.
