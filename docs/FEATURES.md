# RustX features

RustX is an experimental Rust UI framework with TSX-like markup for typed components. One retained Rust application can render into a browser canvas through WebAssembly or open a native desktop window. The RustX runtime owns state, layout, text shaping, editing, and hit testing in Rust.

## Author an application

Compose typed components with the `rustx!` macro in `.rs` or `.rustx` modules. Component setup runs once per mount. Signals update the parts of the view that read them. Keyed children preserve identity when reordered, and removal disposes their scopes. Use context for scoped dependencies and external stores for coherent snapshots on the UI thread.

```rust
use rustx::prelude::*;

pub fn app(cx: Scope) -> View {
    let count = cx.signal(0);
    let shown = count.clone();
    rustx! {
        <Column>
            <Text>{ move || format!("Count: {}", shown.get()) }</Text>
            <Button on_click={move |_| { let _ = count.update(|n| *n += 1); }}>
                "Increase"
            </Button>
        </Column>
    }
}
```

## Layout and rendering

- Typed flex and grid styles, gaps, dimensions, responsive styles, absolute placement, themes, and reusable components.
- Shared text layout with fallbacks, multiline editing, selection, and grapheme-aware operations.
- Canvas drawing, images, viewport transforms, hit testing, and virtual lists. The browser uses WebGPU when available and can use the Rust CPU renderer. Native desktop rendering uses the same scene with platform-specific presentation.

## Interaction and services

- Pointer, keyboard, wheel, and touch event paths. Widgets include buttons, text fields, choices, sliders, dialogs, and lists.
- On native windows, Tab and Shift+Tab scroll the top-level page when the next focused control is off-screen.
- Forms support validation, error state, reset, and controlled values. The browser projects semantics into the DOM and uses a text input bridge. Native hosts project semantics through AccessKit.
- Navigation, scoped HTTP requests, streaming, sockets, clipboard, storage, files, asset loading, and printing are available where the host supports them. Local fixture routes provide reproducible demos.

## Built-in demo

| Route | What you can try |
| --- | --- |
| `#/capabilities` | Guided tour and links to the examples |
| `#/applications` | Forms, editing, and interaction |
| `#/people` | Local sample, remote API, and request states |
| `#/engine` | Layout, text, drawing, and virtual lists |
| `#/lab` | Keyed identity and reactive scopes |
| `#/store` | External-store snapshots |
| `#/playground` | RustX source examples and optional local compilation |

`cargo install --path crates/rustx-cli --bin rustx --locked` installs the CLI from a checkout. `rustx new my-app` generates an independent counter project. Use `rustx check`, `rustx fmt`, `rustx dev`, and `rustx build` in that project.

The isolated compiler runs locally on supported hosts. The ordinary demo can run without it. For editor support, install rust-analyzer and run `npm run package:editor` to build the optional VS Code extension. Native IME, human screen-reader use, and physical mobile coverage still require verification. The browser and native speed goals have their own evidence requirements. [Vision implementation guide](VISION_IMPLEMENTATION.md)
