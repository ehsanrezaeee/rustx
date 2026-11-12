//! Opt-in bounded diagnostics. No text, key characters, URLs or request bodies enter traces.
use std::collections::VecDeque;

#[derive(Default)]
pub struct Diagnostics {
    pub enabled: bool,
    frames: VecDeque<serde_json::Value>,
    events: VecDeque<serde_json::Value>,
    serial: u64,
}

pub fn now() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|w| w.performance())
            .map_or(0., |p| p.now())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        static CLOCK: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
        CLOCK
            .get_or_init(std::time::Instant::now)
            .elapsed()
            .as_secs_f64()
            * 1000.
    }
}

impl Diagnostics {
    pub fn enable(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.frames.clear();
            self.events.clear();
        }
    }
    pub fn frame(
        &mut self,
        reactive: f64,
        layout: f64,
        scene: f64,
        reasons: Vec<&str>,
        changed: usize,
        measurements: Vec<serde_json::Value>,
    ) {
        if !self.enabled {
            return;
        }
        self.serial += 1;
        self.frames.push_back(serde_json::json!({
            "frame": self.serial,
            "reactive_ms": reactive.max(0.),
            "layout_ms": layout.max(0.),
            "scene_ms": scene.max(0.),
            "layout_changed": changed,
            "invalidation": reasons,
            "text_measurements": measurements,
        }));
        if self.frames.len() > 120 {
            self.frames.pop_front();
        }
    }
    pub fn event(
        &mut self,
        kind: rustx_core::EventKind,
        target: u64,
        path: Vec<u64>,
        result: &rustx_core::DispatchResult,
    ) {
        if !self.enabled {
            return;
        }
        self.events.push_back(serde_json::json!({
            "kind": kind,
            "target": target,
            "path": path,
            "handled": result.handled,
            "default_prevented": result.default_prevented,
        }));
        if self.events.len() > 256 {
            self.events.pop_front();
        }
    }
    pub fn json(&self) -> String {
        serde_json::json!({
            "enabled": self.enabled,
            "frames": self.frames,
            "events": self.events,
            "limits": { "frames": 120, "events": 256 },
            "timing_scope": "Rust reactive/layout/scene CPU time; excludes GPU execution and physical presentation",
        })
        .to_string()
    }
}
