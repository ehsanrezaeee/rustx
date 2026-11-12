//! Synchronous events dispatched over the last committed Rust tree.
use crate::NodeId;
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    PointerDown,
    PointerMove,
    PointerUp,
    PointerCancel,
    PointerEnter,
    PointerLeave,
    Wheel,
    Scroll,
    KeyDown,
    KeyUp,
    Focus,
    Blur,
    BeforeInput,
    Input,
    CompositionStart,
    CompositionUpdate,
    CompositionEnd,
    Click,
    Copy,
    Cut,
    Paste,
    FileDrop,
    Resize,
    ResourceError,
    Submit,
    Reset,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventPhase {
    Capture,
    Target,
    Bubble,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PointerType {
    Mouse,
    Touch,
    Pen,
    #[default]
    #[serde(other)]
    Unknown,
}

pub const MAX_DROPPED_FILES: usize = 16;
/// A drop grants metadata only. Explicit file selection remains the byte import API.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DroppedFileMetadata {
    pub name: String,
    pub mime: Option<String>,
    pub size: Option<u64>,
}
impl DroppedFileMetadata {
    pub fn is_valid(&self) -> bool {
        !self.name.is_empty()
            && self.name.len() <= 1024
            && !matches!(self.name.as_str(), "." | "..")
            && !self.name.contains(['/', '\\', '\0', '\r', '\n'])
            && !self.name.chars().any(char::is_control)
            && self.mime.as_ref().is_none_or(|mime| {
                mime.len() <= 256 && mime.bytes().all(|b| (32..=126).contains(&b))
            })
            && self.size.is_none_or(|size| size <= crate::MAX_HOST_ID)
    }
}
impl std::fmt::Debug for DroppedFileMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DroppedFileMetadata")
            .field("name", &"[redacted]")
            .field("size", &self.size)
            .finish_non_exhaustive()
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResourceSource {
    Http,
    Service,
    Renderer,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResourceFailure {
    pub source: ResourceSource,
    /// Stable bounded error identifier, never a URL, file path, body or message.
    pub code: String,
    pub status: Option<u16>,
}
fn default_scale() -> f64 {
    1.
}

#[derive(Clone, Serialize, Deserialize)]
pub struct EventData {
    pub kind: EventKind,
    #[serde(default)]
    pub pointer_id: u64,
    #[serde(default)]
    pub pointer_type: PointerType,
    #[serde(default)]
    pub x: f64,
    #[serde(default)]
    pub y: f64,
    #[serde(default)]
    pub delta_x: f64,
    #[serde(default)]
    pub delta_y: f64,
    #[serde(default)]
    pub button: i16,
    #[serde(default)]
    pub buttons: u16,
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub value: String,
    #[serde(default)]
    pub input_type: String,
    #[serde(default)]
    pub shift: bool,
    #[serde(default)]
    pub ctrl: bool,
    #[serde(default)]
    pub alt: bool,
    #[serde(default)]
    pub meta: bool,
    #[serde(default)]
    pub composing: bool,
    #[serde(default)]
    pub files: Vec<DroppedFileMetadata>,
    #[serde(default)]
    pub width: f64,
    #[serde(default)]
    pub height: f64,
    #[serde(default = "default_scale")]
    pub scale: f64,
    #[serde(default)]
    pub resource_error: Option<ResourceFailure>,
}
impl std::fmt::Debug for EventData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventData")
            .field("kind", &self.kind)
            .field("pointer_id", &self.pointer_id)
            .field("x", &self.x)
            .field("y", &self.y)
            .field(
                "key",
                &if self.key.chars().count() == 1 {
                    "[text]"
                } else {
                    self.key.as_str()
                },
            )
            .field("value", &"[redacted]")
            .field("composing", &self.composing)
            .field("file_count", &self.files.len())
            .finish_non_exhaustive()
    }
}
impl EventData {
    pub fn new(kind: EventKind) -> Self {
        Self {
            kind,
            pointer_id: 0,
            pointer_type: PointerType::Unknown,
            x: 0.,
            y: 0.,
            delta_x: 0.,
            delta_y: 0.,
            button: 0,
            buttons: 0,
            key: String::new(),
            value: String::new(),
            input_type: String::new(),
            shift: false,
            ctrl: false,
            alt: false,
            meta: false,
            composing: false,
            files: vec![],
            width: 0.,
            height: 0.,
            scale: 1.,
            resource_error: None,
        }
    }
    pub fn is_valid(&self) -> bool {
        self.value.len() <= crate::MAX_EDIT_BYTES
            && self.key.len() <= 256
            && [
                self.x,
                self.y,
                self.delta_x,
                self.delta_y,
                self.width,
                self.height,
                self.scale,
            ]
            .iter()
            .all(|v| v.is_finite())
            && self.width >= 0.
            && self.height >= 0.
            && self.scale > 0.
            && self.files.len() <= MAX_DROPPED_FILES
            && self.files.iter().all(DroppedFileMetadata::is_valid)
            && (self.files.is_empty() || self.kind == EventKind::FileDrop)
            && self.resource_error.as_ref().is_none_or(|error| {
                !error.code.is_empty()
                    && error.code.len() <= 64
                    && error
                        .code
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
            })
    }
    pub fn key(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            ..Self::new(EventKind::KeyDown)
        }
    }
    pub fn pointer(kind: EventKind, pointer_id: u64, x: f64, y: f64) -> Self {
        Self {
            pointer_id,
            pointer_type: PointerType::Mouse,
            x,
            y,
            ..Self::new(kind)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pointer_type_preserves_known_host_inputs_and_defaults_unknown() {
        for (json, expected) in [
            (
                r#"{"kind":"pointer_down","pointer_type":"touch"}"#,
                PointerType::Touch,
            ),
            (
                r#"{"kind":"pointer_down","pointer_type":"pen"}"#,
                PointerType::Pen,
            ),
            (r#"{"kind":"pointer_down"}"#, PointerType::Unknown),
            (
                r#"{"kind":"pointer_down","pointer_type":"future-device"}"#,
                PointerType::Unknown,
            ),
        ] {
            let event: EventData = serde_json::from_str(json).unwrap();
            assert_eq!(event.pointer_type, expected);
        }
        assert_eq!(
            EventData::pointer(EventKind::PointerMove, 0, 0., 0.).pointer_type,
            PointerType::Mouse
        );
    }
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct DispatchResult {
    pub handled: bool,
    pub default_prevented: bool,
    pub pointer_capture: Option<NodeId>,
    pub pointer_released: bool,
}
#[derive(Default)]
pub(crate) struct DispatchState {
    pub result: DispatchResult,
    pub stopped: bool,
    pub immediate: bool,
}
#[derive(Clone)]
pub struct Event {
    pub target: NodeId,
    pub current_target: NodeId,
    pub phase: EventPhase,
    /// Logical coordinates relative to the current target's committed box.
    /// Runtime-only dispatch has no layout and uses the supplied coordinates.
    pub local_x: f64,
    pub local_y: f64,
    pub data: EventData,
    pub(crate) state: Rc<RefCell<DispatchState>>,
}
impl Event {
    pub fn prevent_default(&self) {
        self.state.borrow_mut().result.default_prevented = true;
    }
    pub fn stop_propagation(&self) {
        self.state.borrow_mut().stopped = true;
    }
    pub fn stop_immediate_propagation(&self) {
        let mut s = self.state.borrow_mut();
        s.stopped = true;
        s.immediate = true;
    }
    pub fn default_prevented(&self) -> bool {
        self.state.borrow().result.default_prevented
    }
    pub fn capture_pointer(&self) {
        self.state.borrow_mut().result.pointer_capture = Some(self.current_target);
    }
    pub fn release_pointer(&self) {
        self.state.borrow_mut().result.pointer_released = true;
    }
}
#[derive(Clone)]
pub struct EventListener {
    pub kind: EventKind,
    pub capture: bool,
    pub callback: Rc<dyn Fn(Event)>,
}

macro_rules! handlers {
    ($($method:ident => $kind:ident),* $(,)?) => {impl crate::Node {$ (
        pub fn $method(self, f: impl Fn(Event) + 'static) -> Self {self.on_event(EventKind::$kind, f)}
    )*}};
}
handlers! {
    on_pointer_down => PointerDown, on_pointer_move => PointerMove, on_pointer_up => PointerUp,
    on_pointer_cancel => PointerCancel, on_pointer_enter => PointerEnter, on_pointer_leave => PointerLeave,
    on_wheel => Wheel, on_key_down => KeyDown, on_key_up => KeyUp, on_blur => Blur,
    on_before_input => BeforeInput, on_composition_start => CompositionStart,
    on_composition_update => CompositionUpdate, on_composition_end => CompositionEnd,
    on_copy => Copy, on_cut => Cut, on_paste => Paste, on_file_drop => FileDrop,
    on_resize => Resize, on_resource_error => ResourceError, on_submit => Submit, on_reset => Reset,
}
