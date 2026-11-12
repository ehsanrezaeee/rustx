# RustX visual flow guide

Open [rustx-app-flow.pdf](rustx-app-flow.pdf) for six landscape pages explaining the current application and UI rendering pipeline. [rustx-app-flow.html](rustx-app-flow.html) is the editable, self-contained HTML/SVG version. Preview images are included alongside it.

Regenerate from the project root with `node scripts/create-flow-guide.mjs` after installing the existing npm development dependencies. The exporter uses local Google Chrome on macOS; set `CHROME_PATH` to a Chrome/Chromium executable elsewhere. No Python or new project dependency is required.

The guide describes inspected source, rather than a new runtime or performance certification. It distinguishes the CPU layout-tree painter from Vello scene rendering, and keeps existing platform acceptance limits. The runnable application source and prebuilt `dist/` remain intact.
