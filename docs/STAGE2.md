# Stage 2 — Shared visual engine

Updated 2 November 2026. This document describes the implementation and its public API. The authoritative requirement states and executed evidence remain in [ACCEPTANCE.md](ACCEPTANCE.md), against RX-10–RX-18 in [BLUEPRINT.md](BLUEPRINT.md).

The user explicitly requested Stage 2 work on the existing `stage-1-completion` branch. That authorizes progressing the engine while retaining the outstanding Stage 1 Windows, real OS IME and human screen-reader gates. It does not accept those gates or certify Stage 2 on unavailable targets. No new downloadable archive is required for this continuation.

## Run the reference scene

```sh
bash scripts/build-web.sh
cargo run -p rustx-cli --locked -- serve dist 3000
```

Open `http://127.0.0.1:3000/?renderer=cpu` and choose **Visual engine**. Remove `?renderer=cpu` to request WebGPU with CPU fallback. The example source is `examples/lab/src/visual.rustx`, using public Rust APIs rather than special JavaScript application logic.

The page contains a responsive dashboard, a 10,000-object drawing board, and a 100,000-record fixed-height data explorer. The dashboard changes palette through a scoped theme context. Camera controls, pointer pick/drag and arrow-key pan alter Rust camera state. The data explorer creates a window of keyed row scopes, with spacers for the remaining records; selecting a record remains ordinary scoped application state.

## Styles, theme and layout

`Style` now includes flex alignment/justification, flex shrink, min/max width, text alignment/direction, theme color tokens, and bounded viewport overrides. Existing logical pixel sizes, padding/gap, flex/grid/block, wrap, clips, gradients, border radius, opacity and translation remain supported.

```rust
let style = Style::grid(1)
    .gap(16.)
    .background_token(ColorToken::Surface)
    .color_token(ColorToken::Text)
    .at_width(ResponsiveStyle {
        min_width: 700.,
        columns: Some(3),
        font_size: None,
        padding: None,
        direction: None,
    });
```

Each style accepts at most eight viewport overrides, applied in ascending minimum-width order. `Style::resolve` converts tokens and responsive values into concrete Rust-owned values before Taffy measurement and painting. `Theme::DARK` and `Theme::LIGHT` are initial palettes; `Theme` is a public value so applications can supply their own palette. An `Engine` provides `ThemeContext` to its root scope. A descendant reads the context once during setup and updates its `Signal<Theme>` from an event; `Engine::set_theme` provides the corresponding host-side API.

Taffy nodes remain keyed by retained runtime node IDs. Style, text measurement context, font epoch and children changes invalidate the relevant retained Taffy nodes; unchanged styles/context/children are reused. Measurement and paint use the same shaped Rust text. `LayoutEngine::changed`, `TextEngine::shape_count` and scene cache statistics support checking invalidation. Retained full-frame scene composition remains the baseline; this does not claim damage-region rendering or an O(changed nodes) traversal.

This is a finite typed style API, not a CSS cascade or a claim of complete browser CSS layout behavior. Percentage sizing, a full font-family/weight style system, general container queries, ellipsis and additional layout contracts remain subject to the blueprint and acceptance ledger.

## Unicode text and fonts

The bundled font database contains DejaVu Sans and monochrome Noto Emoji. Both shaping and the CPU renderer choose the font of each glyph run; the software renderer no longer rasterizes fallback glyph IDs through the DejaVu font unconditionally.

`TextEngine::shape_with(text, size, width, alignment, direction)` exposes explicit alignment and paragraph direction. `Shaped::clusters` retains source byte ranges and measured visual rectangles, including RTL flags. `byte_to_utf16` and `utf16_to_byte` convert only valid scalar boundaries; a UTF-16 offset inside a surrogate pair is rejected.

`TextEngine::add_font` validates the font budget, loads the data, increments the font epoch, and clears affected shaping results. The engine includes that epoch in text measurement and scene cache keys. Adding a ready font can therefore invalidate both layout and painting even when the text string is unchanged.

The fallback is monochrome. Its presence does not certify all Unicode or emoji sequences, complex color-font behavior, every language, or editing correctness. Cluster data is a shared basis for future caret/selection work; full grapheme/bidi editing, IME preedit, undo, clipboard and screen-reader editing remain separate Stage 3 contracts.

## Fixed-height virtual list

```rust
let list = VirtualListState::new(&cx, 100_000, 32., 320.);
let view = virtual_list(list.clone(), move |index| {
    Node::text(format!("Record {}", index + 1))
});
list.scroll_to(32_000.);
```

The configuration gives record count, fixed row height and viewport height. The range includes approximately two viewports of overscan on either side, plus a boundary row. Rows use stable index keys. Retained rows keep component state as the window moves; rows leaving the range are disposed. One focused row can stay materialized outside the current window. The list remains bounded by viewport size rather than the dataset length. With 20 visible rows the normal window is at most 101 rows, plus one retained focused row, within the blueprint's 120-row ceiling.

The API accepts at most 1,000,000 records. Offsets are finite, clamped to the scrollable extent, and represented with `f64` before conversion to layout coordinates. `scroll_by` and `scroll_to` drive the same scoped offset signal. The viewport remains a clipped Rust layout box; spacers reserve skipped row height rather than creating skipped controls.

The semantic model carries `list`/`listitem` roles and one-based item position/total count. Browser aria-owns links the flat projection into list membership; focus reveals nested rows and wheel events route to the Rust list owner. These data and browser automation do not substitute for a real screen-reader task. Focus retention is bounded scaffolding, not a claim that every virtual navigation pattern or assistive technology behavior has passed.

## Retained drawing surface

Portable drawing data lives in `rustx::drawing`. `CanvasScene` owns an immutable collection of `DrawShape` values; attach it to a `<Canvas>` with an `Rc`, and update the separate `Camera` value reactively.

```rust
use rustx::drawing::{Bounds, Camera, CanvasScene, DrawShape};
use std::rc::Rc;

let scene = Rc::new(CanvasScene::new(vec![
    DrawShape::rect(1, Bounds::new(20., 20., 120., 80.), 0x64b5f6),
]).expect("valid scene"));
let camera = Camera::default();
let canvas = Node::new(Kind::Canvas)
    .drawing(scene)
    .camera(camera)
    .style(Style::column().height(320.).clip());
```

Primitives include rounded rectangles, filled/stroked paths with line/quadratic/cubic segments, shaped text and registered images. Each shape has a stable ID, declared bounds, fill color and affine transform. The current finite contract uses explicit bounds and solid fills for these custom primitives; full SVG/DOM behavior is outside v1.

A 256-unit spatial index filters candidate shapes before either backend paints. Shapes covering many buckets use a separate bounded candidate list. Results preserve submission order, and hit queries search from the topmost candidate. The engine derives GPU and CPU geometry from the same portable path data.

World coordinates and camera arithmetic use `f64`. Geometry is rebased to each shape's local origin and then to the camera before conversion to renderer coordinates. `Camera::pan` and `zoom_at` preserve the zoom anchor; zoom is clamped to 0.05–20.0. Drawing coordinates are finite and limited to absolute values of 10¹². This is a finite, navigable retained scene with culling, not infinite memory or full CAD precision.

`CanvasScene::new` rejects duplicate IDs, invalid bounds/transforms and oversized data. Limits include 100,000 shapes, 4,096 path segments per shape, and 8 KiB of UTF-8 text per drawing text item. Application-provided bounds must conservatively describe custom primitive coverage. Kurbo winding/stroked outlines test rounded/path geometry in inverse-transformed coordinates. Declared bounds validate control points and expand for round stroke coverage. Executed tests cover these contracts; general conformance still needs the full target matrix.

## Resource and renderer lifecycle

The shaping cache and software glyph cache use entry/byte-bounded LRU storage. Images are registered explicitly with the retained scene, and an image epoch invalidates dependent fragments and synchronizes software copies. Removing an image releases its registration and invalidates stale scene references.

| Resource | Current limit or policy |
| --- | --- |
| Shaped text cache | 512 entries, 8 MiB accounted payload |
| Software glyph cache | 2,048 entries, 4 MiB accounted payload |
| Added font data | 8 MiB per add, 32 MiB total, 16-face admission limit |
| Image registration | 64 images, 32 MiB RGBA payload, dimensions at most 4,096×4,096 |
| Retained scene fragments | Cleared beyond 4,096 entries or 16 MiB accounted encoding |
| CPU opacity groups | Scratch bounds derived from visible subtree extent rather than a full frame per group |
| Shadow blur | Clamped to 32 logical pixels |
| Surface | At most 8,192 pixels per dimension and 16,777,216 pixels; browser reduces DPR before allocation |

These are cache/payload policies, not a claim that total allocator, driver, process or Wasm committed memory stays below their sum. Font parsers, renderer internals, framebuffer allocations, masks, temporary work and CPU/GPU resource copies also consume memory. Lifecycle evidence must distinguish live resources from retained Wasm capacity and process RSS.

The GPU path presents directly through its surface. Normal drawing does not route a frame through CPU readback. CPU/GPU comparisons allow explicitly declared text-rasterizer tolerance and do not certify platform parity merely because both APIs compile.

Browser recovery observes actual WebGPU device loss and uncaptured errors, drops GPU state, replaces the canvas with a CPU-capable canvas, and invalidates the existing Rust engine. It retains application state and networking. `window.rustx.retryGPU()` requests a new GPU surface/device against that same engine; `destroyGPU()` is a diagnostic hook that destroys the actual device for testing. Initialization failure and post-initialization device loss are distinct checks.

Native surface loss/outdated errors recreate/retry the surface. Device loss/uncaptured errors wake the native window; the host attempts CPU recovery on the next draw. None of this source behavior alone certifies actual Windows or physical device-loss recovery. Record the real path exercised in the ledger.

## Scheduling and acceptance limits

The browser uses demand-driven animation frames, pauses painting while hidden, resumes on visibility changes, and observes DPR changes separately from viewport resize. Service completions still enter Rust and pump host work independently of painting. The native event loop waits when settled and requests redraw for invalidation or recovery.

Stage 2 acceptance still requires the declared browser/native parity matrix, actual forced fallback and device-loss recovery, mixed Persian/English text, bounded virtualization, idle behavior and preliminary measured performance. Logs must identify exact source, OS, browser/backend, viewport/DPR and hardware. A small offscreen image, text injection, mock device failure, native compilation or a successful Windows cross-build cannot close a missing real-target gate.

Refer to [PLATFORM_CHECKS.md](PLATFORM_CHECKS.md) for the inherited Windows, IME and assistive-technology tasks. The earlier Firefox startup failures and the distinction between WebKit automation and actual Safari remain explicit until new evidence supersedes them. No universal performance comparison, release certification or Stage 1/2 acceptance is implied by this API document.

## Contracts added during the four-stage review

`Length::{Auto,Px,Percent}` supplies typed width/height and physical inset units. Percent uses the 0–100 scale and Taffy resolves it against the parent content box; percentage height requires a definite containing height. Existing `width(f32)`/`height(f32)` remain pixel shorthands and the last setter wins. `min_height`/`max_height` complement width constraints.

`Insets::new(top,right,bottom,left)` supplies physical pixel padding and margins. `padding_logical`/`margin_logical` take block-start, inline-end, block-end, inline-start, mapping inline edges using explicit `TextDirection`. Vertical writing and percentage margins/padding remain outside this finite API. `absolute`, typed `top/right/bottom/left` and `z_index` feed retained Rust geometry and stacking. CPU/GPU traversal and hit testing share paint order; transparent browser hit surfaces use that order while semantic/Tab order remains logical construction order. General affine style transforms remain unimplemented; the drawing API separately supports affine geometry.

`font_family`, `font_weight` and pixel `line_height` affect shared shaping, measurement, paint and caret geometry. The requested face must exist in bundled/loaded fonts; otherwise the text library selects its available fallback/nearest weight. Adding font bytes still invalidates the bounded cache. `ellipsis(max_lines)` uses shared shaping and grapheme boundaries, clips the visible text, retains original source offsets and exposes the full semantic text. Editable/selectable models retain all lines rather than eliding their caret geometry. Explicit per-language glyph-variant shaping remains a dependency/target conformance gap; language semantics do not certify it.

The new native contracts are in `rustx-engine/tests/stage2_contracts.rs`; an actually compiled browser playground fixture checks overlap picking and logical Tab order. These tests do not replace actual Windows/Safari/mobile/IME/screen-reader tasks.
