# Stage 4 measurement contract

Frozen before this candidate's budget run: Apple M3 MacBook Air, 16 GiB RAM, macOS 27.0.1 arm64, installed Chrome 157.0.8103.40, built-in 60 Hz display, 1280×720 logical pixels, DPR 1, 10,000 drawing objects, at most 1,000 visible. Use a visible browser and actual Metal adapter/submissions for the GPU row. Five samples of 180 scripted pan frames retain every requestAnimationFrame interval; report p95 against the blueprint's unchanged **16.7 ms**. CPU is a separate functional/timing result without a GPU-speed promise.

The original Stage 1 hardware freeze was incomplete. This local desktop profile makes the current run reproducible; it does not retrospectively satisfy that contract or substitute for the required midrange Android device, actual iOS Safari and Windows native runs. Those exact devices/runs remain open. Thresholds are not increased after observing results.

Build the lab separately from the default production bundle:

    rustx build --target web --release --out dist-bench --benchmark --instrument
    rustx serve dist-bench 3012
    RUSTX_URL=http://127.0.0.1:3012 npm run test:stage4:budgets
    RUSTX_URL=http://127.0.0.1:3012 RUSTX_BACKEND=cpu npm run test:stage4:budgets

The inspector/Rust profiler are disabled during measurement. A minimal opt-in adapter exposes inspection, pan, draw count, committed Wasm capacity and the counting allocator. Keep all unrelated builds/compiler/browser load tests stopped while timing. The allocator wraps actual allocations/deallocations/reallocations, rather than using linear-memory capacity as a leak proxy. Warm five route sequences, then measure 100 sequences through engine, People with a slow request, Applications and home. Record every tenth result and compare counts/live bytes against the warmed home baseline (≤10% growth). Scope/generation cancellation, bounded scene/font caches and separate GPU-loss tests provide other resource evidence; this run alone does not establish all driver resource bounds.

Input timing uses a synthetic DOM activation that reaches the real Rust handler, then the animation frame after the changed Rust frame. This is an event-to-next-presentation **estimate**, not physical scanout or real pointer-device latency. Cold startup separately uses an empty browser context and actual CDP 10 Mbps down/100 ms RTT. The first usable semantic control is the endpoint. Compressed transfer counts the precached shell and executable dependencies; font bytes are embedded in Wasm. Full-demo/counter sizes and whether each unchanged 3 MiB/5 MiB/3-second budget passes belong in the evidence JSON.

Idle compares settled draw/GPU submission counts. Development reload polling is absent in the production/lab bundles. Service-worker installation and outstanding HTTP/TTL work are finite documented background work.

No optimized equivalent React DOM or mature canvas baseline has been run. These local observations cannot support universal performance claims. Exact raw samples, transfer totals, allocation/capacity records and failed gates are in `docs/evidence/stage4-budgets-*.json` and `stage4-deploy-*.json`. Missing target, comparative and physical-presentation evidence keeps RX-39/RX-40's full acceptance open.

## Delivery observations — 12 November 2026

The corrected desktop run has **140 visible shapes** from the 10,000-object board. GPU rAF p95 is **16.8 ms**, failing the unchanged **16.7 ms** budget. CPU rAF p95 is **16.8 ms**, reported separately without a GPU-speed promise. Input estimates are **33.4 ms GPU / 34.4 ms CPU**. After 100 measured route/request cycles, warmed runtime owner/signal/observer counts return to **[89, 7, 5]**; live allocations change **+0.785% GPU / -0.001% CPU**, within 10%, and idle draw/submission counts settle. These counts are not a complete driver/listener/process-memory audit. The full demo shell is **3,202,009 compressed bytes**: below 5 MiB but above the 3 MiB counter ceiling; this full-demo measurement does not substitute for an independently measured standalone counter. The isolated Chrome cold-load observation is **3659 ms**, failing 3 seconds at the configured 10 Mbps/100 ms RTT. One cold-load observation is not the required five-sample/device certification. Failed budgets remain open.

The earlier zero-visible-shape report is retained under evidence/history/stage4/pre-readable-delivery and is not a valid drawing benchmark. The final script focuses/reveals the actual Rust canvas and requires positive visible geometry before taking samples. The corrected GPU report still fails the original threshold; no budget was relaxed. Power/thermal control, repeated cold samples and the complete external-device/comparative matrix remain certification work.
