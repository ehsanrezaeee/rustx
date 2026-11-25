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
- The native host retries a GPU surface timeout. If the GPU renderer fails, the host can resume on CPU with the same application state.

## Interaction and services

- Pointer, keyboard, wheel, and touch event paths. Widgets include buttons, text fields, choices, sliders, dialogs, and lists.
- Focus reveal uses Rust layout to scroll nested regions from the inside out, then reveal the focused control on the top-level page. Scroll state clamps each request, and later manual scrolling stays in place.
- Disabling a group removes its descendants from keyboard traversal and projects their disabled state to the browser and native accessibility trees.
- Forms support validation, error state, reset, and controlled values. The browser projects semantics into the DOM and uses a text input bridge. Native hosts project semantics through AccessKit.
- Native controls use their linked labels for accessible names and expose linked help and errors as accessible descriptions.
- While a native modal dialog is open, accessibility navigation stays inside it. Closing the dialog restores the previous accessibility tree and focus.
- On macOS, restoring a dialog's background keeps unchanged status messages quiet. New feedback still posts an accessibility announcement.
- Native text fields track the caret and selection reported inside marked text. Canceling composition restores the original value and selection. A committed composition is a single undoable edit.
- Navigation, scoped HTTP requests, streaming, sockets, clipboard, storage, files, asset loading, and printing are available where the host supports them. Local fixture routes provide reproducible demos.

## Built-in demo

| Route | What you can try |
| --- | --- |
| `#/capabilities` | Guided tour and links to the examples |
| `#/applications` | Forms, editing, interaction, and nested Workspace shortcuts |
| `#/people` | Local sample, remote API, and request states |
| `#/engine` | Layout, text, drawing, and virtual lists |
| `#/lab` | Keyed identity and reactive scopes |
| `#/store` | External-store snapshots |
| `#/playground` | RustX source examples and optional local compilation |

`cargo install --path crates/rustx-cli --bin rustx --locked` installs the CLI from a checkout. `rustx new my-app` generates an independent counter project. Use `rustx check`, `rustx fmt`, `rustx dev`, and `rustx build` in that project.

The isolated compiler runs locally on supported hosts. The ordinary demo can run without it. For editor support, install rust-analyzer and run `npm run package:editor` to build the optional VS Code extension. Native IME, human screen-reader use, and physical mobile coverage still require verification. The browser and native speed goals have their own evidence requirements. [Vision implementation guide](VISION_IMPLEMENTATION.md)
