# React parity audit — revision 1

16 November 2026; RustX source baseline `43c8b53`, plus RP-002 semantic teardown in this milestone. This is an initial source audit, not exhaustive browser certification. RP-001 remains implementing until element/property/event inventory and comparative baseline provisioning are complete. No percentage of React parity is claimed.

The official [version index](https://react.dev/versions) identifies React 19.3.0. The API inventory below targets that stable reference, checked on this date. Benchmark dependencies are not yet installed or locked; no React comparison has run. Public docs can change: a later inventory revision must identify additions rather than silently changing the baseline.

Status: **partial** = some implementation/tests exist but the complete outcome is unverified; **missing** = no matching public contract found in the inspected source; **legacy** = migration required. No row here means full platform equivalence. All React mechanisms map to Rust/canvas outcomes, not TSX source compatibility.

## Source anchors and inherited evidence

- [Runtime](../crates/rustx-core/src/lib.rs): Scope, Signal, Runtime, owners, reconciliation, effects/context; [runtime regressions](../crates/rustx/tests/runtime.rs).
- [Macro compiler](../crates/rustx-macros/src/lib.rs); [compile diagnostics](../crates/rustx/tests/compile.rs).
- [Resources](../crates/rustx-core/src/resources.rs), [services/tasks](../crates/rustx-core/src/services.rs), [errors](../crates/rustx-core/src/errors.rs), [router](../crates/rustx-core/src/router.rs); [service regressions](../crates/rustx/tests/stage3_services.rs).
- [Forms](../crates/rustx-core/src/forms.rs), [editing](../crates/rustx-core/src/editing.rs), [widgets](../crates/rustx-core/src/widgets.rs); [editing/form tests](../crates/rustx/tests/editing_forms.rs).
- [Style](../crates/rustx-core/src/style.rs), [drawing](../crates/rustx-core/src/drawing.rs), [engine](../crates/rustx-engine/src/lib.rs); [layout/typography contracts](../crates/rustx-engine/tests/stage2_contracts.rs).
- [Browser semantic/input adapter](../web/semantics.js), [host](../web/host.js), [services](../web/services.js); [browser regression suite](../scripts/test-browser.mjs).
- [Native host](../crates/rustx-engine/src/host.rs), [AccessKit projection](../crates/rustx-engine/src/accessibility.rs).
- [CLI tooling](../crates/rustx-cli/src/tooling/mod.rs), [sample applications](../examples/lab/src/applications.rustx).

[Acceptance](ACCEPTANCE.md), [support](stage4/SUPPORT.md) and [checkout replay](evidence/checkout-2026-11-16/README.md) retain exact inherited results. Browser CPU automation and native startup are not human AT/IME or full Windows interaction evidence. The first new result is the finite RP-002 adapter teardown contract, recorded separately.

## React public API inventory

Each grouped row explicitly names its entries; splitting a group into per-entry conformance cases remains RP-001 work. “Missing” does not mean the underlying concept is impossible in Rust; it means no accepted equivalent contract is present.

| Reference surface | RustX mapping / status | Required next evidence |
| --- | --- | --- |
| `useState`, `useReducer` | Signal get/set/update, partial; reducer can be authored in Rust | Reentrant update semantics, reducer example, atomic commit contract |
| `useContext`, `createContext` | Scope provide/context with TypeId and parent lookup, partial | RP-003 verifies nearest-provider shadowing, runtime isolation, disposal and signal contexts in Rust tests; reactive provider replacement and full target demos remain open |
| `useRef`, `useImperativeHandle` | NodeRef plus Rust handles, partial | Ref reset/rebinding and disposed imperative API tests |
| `useMemo`, `useCallback`, `memo` | Scope memo and mount-once closures, partial | Dependency diamonds, equality and reactive-prop invalidation; no rerender-hook simulation |
| `useEffect`, `useLayoutEffect`, `useInsertionEffect`, `useEffectEvent` | Scope effect and cleanup, partial | Engine post-layout phase absent; document non-reactive reads and style insertion equivalent; effect timing is not browser layout timing |
| `useId`, `useDebugValue`, `captureOwnerStack` | Runtime IDs, inspection and source locations, partial | Cross-root/hydration-safe stable IDs, user diagnostic labels and owner stack API |
| `useSyncExternalStore` | Missing subscription/snapshot contract | Coherent snapshots, subscription replacement, cleanup and tearing regression |
| `useTransition`, `startTransition`, `useDeferredValue` | Missing priority/deferred scheduling contract | Urgent input during heavy updates, starvation and atomic commits |
| `useOptimistic`, `useActionState`, DOM `useFormStatus`, `requestFormReset` | Forms, Mutation and reset state, partial | Pending action sequencing, optimistic rollback preserving newer edits, nested form status |
| `use`, `lazy`, `Suspense` | QueryState and error boundary are partial building blocks | Nested loading boundaries, refresh retention, real code splitting and resource reads |
| `Fragment` | Node fragment / markup fragments, partial | Independent composition/identity suite beyond current examples |
| `Profiler`, `StrictMode` | Inspector/profiling and deterministic checks, partial | Public scoped profiling; diagnostics for invalid lifecycle behavior without violating mount-once setup |
| `Activity`, `ViewTransition`, `addTransitionType` | Missing | Hide/resume cleanup/state policy, animation interruption, reduced motion |
| `act` | Runtime flush plus test helpers, partial | Bounded settle helper covering tasks/effects; failure on unresolved cycles |
| `cache`, `cacheSignal` | QueryCache is partial, not server request caching | Request isolation, invalidation lifetime, cancellation and identity |
| `createPortal` | Positioned overlays/modal widgets, partial | Reparented visual roots preserving logical events/context/focus |
| `flushSync` | Runtime flush, partial | Specify commit boundary and host visibility; effects can schedule further commits |
| `createRoot` / root render/unmount | Rust Runtime supports independent instances; browser host is singleton, partial | Multiple browser roots, full host teardown and remount |
| `hydrateRoot` | Missing | Server-state activation, mismatch/error recovery and pre-activation input preservation |
| `prefetchDNS`, `preconnect`, `preload`, `preloadModule`, `preinit`, `preinitModule` | Build-generated Wasm preload only, partial | Typed resource scheduling/deduplication/error and security policies |
| `browser` | Missing server/browser boundary API | Browser-only subtree handling during server content generation |
| `renderToReadableStream`, `renderToPipeableStream`, `resume`, `resumeToPipeableStream` | Missing | Streaming content/state, disconnect/cancel and deterministic takeover |
| `prerender`, `prerenderToNodeStream`, `resumeAndPrerender`, `resumeAndPrerenderToNodeStream` | Missing | Static semantic content export and resume protocol |
| `renderToString`, `renderToStaticMarkup` | Legacy React server APIs; RustX exporter missing | Explicit content/static export migration |
| `use client`, `use server`, Server Components / Server Functions | Missing application server boundary | Typed serialization, per-request auth/cache isolation; playground compiler is not an app server |
| `Children`, `cloneElement`, `Component`, `createElement`, `createRef`, `forwardRef`, `isValidElement`, `PureComponent` | Legacy; typed View/props/Node/NodeRef offer partial migration building blocks | Runnable porting recipes, class lifecycle/error-boundary mapping |
| Removed `createFactory`, legacy context/propTypes/string refs; DOM `findDOMNode`, `hydrate`, `render`, `unmountComponentAtNode`, stream legacy APIs | Legacy migration, no compatibility claim | Typed component/context/ref/root/content replacements |
| Compiler memoization/directives and hook lint rules | Rust types/macros/formatter, partial tooling outcome | Pin fair React compiler baseline; actionable Rust lifecycle diagnostics |

Official inventories: [hooks](https://react.dev/reference/react/hooks), [components](https://react.dev/reference/react/components), [APIs](https://react.dev/reference/react/apis), [legacy](https://react.dev/reference/react/legacy), [React DOM](https://react.dev/reference/react-dom), [DOM hooks](https://react.dev/reference/react-dom/hooks), [client](https://react.dev/reference/react-dom/client), [server](https://react.dev/reference/react-dom/server), [static](https://react.dev/reference/react-dom/static), [directives](https://react.dev/reference/rsc/directives). These links establish the comparison surface; the RustX judgments above are source-audit findings.

## Browser and ecosystem outcome inventory

The [DOM component index](https://react.dev/reference/react-dom/components) covers HTML, SVG and custom elements. This first audit groups categories; exhaustive individual tag, attribute, input type and event coverage is explicitly unfinished under RP-001. Never label that inventory complete from these groups.

| Outcome | Existing path / gap | Acceptance task |
| --- | --- | --- |
| Typed composition, defaults, generics, slots and keys | Macros/runtime tests; partial | Public library port without private APIs |
| Every form/input category | widgets/forms/editing; text/password/multiline and composed controls partial; date/color/file/native picker parity open | Controlled/uncontrolled, constraints, autofill, actual IME and password manager |
| Content elements, headings/lists/tables/links/disclosure | Widgets/semantics partial; full document structure missing | Reading order, selection, link context menu, nested content and browser find |
| Image, SVG, filters, masks, animation | Drawing/assets and CPU/GPU scene partial; no full SVG interpreter | Matched clips/transforms/filter/text and hit testing; no hidden SVG DOM renderer |
| Audio/video/captions/fullscreen/PiP/DRM | Missing public media capability | Real playback/permission/secure-origin and browser-owned exceptions |
| Custom elements, iframe/embedded third-party content | Isolated playground iframe only | Typed migration/interop with explicit visual DOM exceptions |
| Metadata/style/script/resource lifecycle | Static index/build loader only | Route title/meta, resource cancellation/deduplication; Rust owns visible layout |
| All event families and browser default actions | core events + semantics partial | Touch/pen/capture, composition, clipboard, native shortcuts and media events |
| Routing/loaders/auth/history | Router/HostServices partial | Nested loader cancellation, login expiry, scroll/focus restoration |
| Query caching/mutations/offline | QueryCache/Mutation/service worker partial | Per-user isolation, optimistic rollback, reconnect/TTL/migration |
| Responsive style, motion, locale/text | Taffy/text/typed styles partial | Bidi/IME, reduced motion/high contrast, rich text, real mobile keyboard |
| Accessibility browser/native | Semantics/AccessKit partial | Human VoiceOver/NVDA/TalkBack full task completion |
| Browser lifecycle | host/services partial | Multiple roots, teardown, hidden/resume, memory pressure, GPU loss |
| Find/translation/print/selection | Selectable Rust text and print text popup partial | Native find and translation; real printing, not export-only |
| Network/storage/files/clipboard | services partial | CORS/auth/quota/permission failures, multi-tab synchronization |
| Native distribution/services | native host + packaging partial | Clean Windows installation, multi-window, DPI, UIA, IME and lifecycle |
| Server/content/progressive enhancement | Missing | Indexable content, stable activation, streaming and no-Wasm fallback |
| Ecosystem/editor/migration | CLI/editor exists; library/migration contracts partial | Independent widget library, React region migration, state-preserving reload |

## Owner/runtime/task boundary findings

`Scope` and `Signal` use Weak handles to a single-threaded Rc/RefCell runtime. Owners contain TypeId context, signals, observers and cleanup callbacks. IDs increase within each runtime, not globally. `context()` clones the nearest matching value; sharing `Signal<T>` is possible, but replacing a plain value is not reactive notification.

`flush()` settles memos ahead of views, bounded at 1000 jobs, then runs an effect snapshot. Effect writes can require another flush; the cascade bound is 100 commits. This is not priority scheduling or an explicit transaction API. Cleanup callbacks live in a HashMap, so do not promise registration-order cleanup. `Signal::update` clones, invokes user code, then writes back: nested writes require an explicit contract before reducer/transaction equivalence is claimed.

`TaskQueue` caps pending tasks at 64, associates cleanup with Scope, and polls using a noop waker. It is not a general self-waking executor or background scheduler. First M1 task: specify and test context shadowing, independent runtime isolation and owner disposal using existing APIs before designing reactive providers/external stores.
