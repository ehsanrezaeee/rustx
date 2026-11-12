//! Platform-independent, grapheme-safe editing. Browser selection uses UTF-16;
//! the Rust model and shaped text use UTF-8 byte boundaries.
use serde::Serialize;
use std::{cell::RefCell, fmt, ops::Range, rc::Rc};
use unicode_segmentation::UnicodeSegmentation;

pub const MAX_EDIT_BYTES: usize = 1_048_576;
const HISTORY_ENTRIES: usize = 128;
const HISTORY_BYTES: usize = 8 * 1_048_576;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Selection {
    pub anchor: usize,
    pub focus: usize,
}
impl Selection {
    pub fn caret(position: usize) -> Self {
        Self {
            anchor: position,
            focus: position,
        }
    }
    pub fn range(self) -> Range<usize> {
        self.anchor.min(self.focus)..self.anchor.max(self.focus)
    }
    pub fn is_collapsed(self) -> bool {
        self.anchor == self.focus
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditError {
    InvalidSelection,
    TooLarge,
    ReadOnly,
    PasswordClipboard,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveDirection {
    Backward,
    Forward,
    Start,
    End,
}

/// Safe to include in semantic diagnostics. Password text and byte positions are
/// projected onto bullet graphemes; actual values require an explicit `value()`.
#[derive(Clone, Debug, Serialize)]
pub struct EditingState {
    pub scroll_y: f32,
    pub text: String,
    pub selection: Selection,
    pub anchor_utf16: usize,
    pub focus_utf16: usize,
    pub composing: bool,
    pub preedit: Option<Selection>,
    pub password: bool,
    pub read_only: bool,
    pub revision: u64,
}
#[derive(Clone)]
struct Checkpoint {
    value: String,
    selection: Selection,
}
impl Checkpoint {
    fn bytes(&self) -> usize {
        self.value.len()
    }
}
struct Model {
    scroll_y: f32,
    scroll_limit: f32,
    current: Checkpoint,
    composition: Option<Checkpoint>,
    preedit: Option<Selection>,
    undo: Vec<Checkpoint>,
    redo: Vec<Checkpoint>,
    password: bool,
    read_only: bool,
    revision: u64,
}
/// Shared editing identity survives reactive updates and keyed reordering.
#[derive(Clone)]
pub struct Editor(Rc<RefCell<Model>>);
impl fmt::Debug for Editor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.snapshot().fmt(f)
    }
}
pub fn edit_byte_to_utf16(text: &str, byte: usize) -> Option<usize> {
    text.get(..byte).map(|s| s.encode_utf16().count())
}
pub fn edit_utf16_to_byte(text: &str, offset: usize) -> Option<usize> {
    let mut units = 0;
    for (byte, c) in text.char_indices() {
        if units == offset {
            return Some(byte);
        }
        units += c.len_utf16();
        if units > offset {
            return None;
        }
    }
    (units == offset).then_some(text.len())
}
fn floor_grapheme(text: &str, byte: usize) -> usize {
    if byte >= text.len() {
        return text.len();
    }
    text.grapheme_indices(true)
        .take_while(|(i, _)| *i <= byte)
        .last()
        .map_or(0, |(i, _)| i)
}
fn next_grapheme(text: &str, byte: usize) -> usize {
    text.grapheme_indices(true)
        .find(|(i, _)| *i > byte)
        .map_or(text.len(), |(i, _)| i)
}
fn ceil_grapheme(text: &str, byte: usize) -> usize {
    text.grapheme_indices(true)
        .find(|(i, _)| *i >= byte)
        .map_or(text.len(), |(i, _)| i)
}
fn previous_grapheme(text: &str, byte: usize) -> usize {
    text.grapheme_indices(true)
        .take_while(|(i, _)| *i < byte)
        .last()
        .map_or(0, |(i, _)| i)
}
fn trim_history(history: &mut Vec<Checkpoint>) {
    let mut bytes: usize = history.iter().map(Checkpoint::bytes).sum();
    while history.len() > HISTORY_ENTRIES || bytes > HISTORY_BYTES {
        bytes -= history.remove(0).bytes();
    }
}
impl Editor {
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("editor initial value exceeds limit")
    }
    pub fn try_new(value: impl Into<String>) -> Result<Self, EditError> {
        let value = value.into();
        if value.len() > MAX_EDIT_BYTES {
            return Err(EditError::TooLarge);
        }
        Ok(Self(Rc::new(RefCell::new(Model {
            current: Checkpoint {
                value,
                selection: Selection::default(),
            },
            composition: None,
            preedit: None,
            undo: vec![],
            redo: vec![],
            password: false,
            read_only: false,
            revision: 0,
            scroll_y: 0.,
            scroll_limit: 0.,
        }))))
    }
    pub fn with_password(self, password: bool) -> Self {
        self.0.borrow_mut().password = password;
        self
    }
    pub fn with_read_only(self, read_only: bool) -> Self {
        self.0.borrow_mut().read_only = read_only;
        self
    }
    pub fn set_read_only(&self, read_only: bool) {
        self.0.borrow_mut().read_only = read_only;
    }
    pub fn is_password(&self) -> bool {
        self.0.borrow().password
    }
    pub fn is_read_only(&self) -> bool {
        self.0.borrow().read_only
    }
    pub fn is_composing(&self) -> bool {
        self.0.borrow().composition.is_some()
    }
    pub fn value(&self) -> String {
        self.0.borrow().current.value.clone()
    }
    pub fn display_text(&self) -> String {
        let m = self.0.borrow();
        if m.password {
            "•".repeat(m.current.value.graphemes(true).count())
        } else {
            m.current.value.clone()
        }
    }
    pub fn selection(&self) -> Selection {
        self.0.borrow().current.selection
    }
    pub fn selection_utf16(&self) -> (usize, usize) {
        let m = self.0.borrow();
        (
            edit_byte_to_utf16(&m.current.value, m.current.selection.anchor).unwrap(),
            edit_byte_to_utf16(&m.current.value, m.current.selection.focus).unwrap(),
        )
    }
    pub fn snapshot(&self) -> EditingState {
        let m = self.0.borrow();
        let selection = m.current.selection;
        let project = |s: Selection| {
            if m.password {
                Selection {
                    anchor: m.current.value[..s.anchor].graphemes(true).count() * '•'.len_utf8(),
                    focus: m.current.value[..s.focus].graphemes(true).count() * '•'.len_utf8(),
                }
            } else {
                s
            }
        };
        EditingState {
            scroll_y: m.scroll_y,
            text: if m.password {
                "•".repeat(m.current.value.graphemes(true).count())
            } else {
                m.current.value.clone()
            },
            selection: project(selection),
            anchor_utf16: edit_byte_to_utf16(&m.current.value, selection.anchor).unwrap(),
            focus_utf16: edit_byte_to_utf16(&m.current.value, selection.focus).unwrap(),
            composing: m.composition.is_some(),
            preedit: m.preedit.map(project),
            password: m.password,
            read_only: m.read_only,
            revision: m.revision,
        }
    }
    pub fn select(&self, anchor: usize, focus: usize) -> Result<(), EditError> {
        let mut m = self.0.borrow_mut();
        if !m.current.value.is_char_boundary(anchor) || !m.current.value.is_char_boundary(focus) {
            return Err(EditError::InvalidSelection);
        }
        m.current.selection = Selection {
            anchor: floor_grapheme(&m.current.value, anchor),
            focus: floor_grapheme(&m.current.value, focus),
        };
        m.revision += 1;
        Ok(())
    }
    pub fn scroll_y(&self) -> f32 {
        self.0.borrow().scroll_y
    }
    pub fn scroll_by(&self, delta: f32) -> bool {
        if !delta.is_finite() {
            return false;
        }
        let mut model = self.0.borrow_mut();
        let next = (model.scroll_y + delta).clamp(0., model.scroll_limit);
        if next == model.scroll_y {
            return false;
        }
        model.scroll_y = next;
        true
    }
    /// Renderer-supplied text bounds and optional caret reveal, never DOM measurement.
    pub fn viewport(&self, limit: f32, caret: Option<(f32, f32)>, height: f32) -> f32 {
        let mut model = self.0.borrow_mut();
        model.scroll_limit = if limit.is_finite() { limit.max(0.) } else { 0. };
        model.scroll_y = model.scroll_y.clamp(0., model.scroll_limit);
        if let Some((top, bottom)) = caret {
            if top < model.scroll_y {
                model.scroll_y = top.max(0.);
            } else if bottom > model.scroll_y + height {
                model.scroll_y = (bottom - height).clamp(0., model.scroll_limit);
            }
        }
        model.scroll_y
    }
    pub fn select_utf16(&self, anchor: usize, focus: usize) -> Result<(), EditError> {
        let value = self.value();
        self.select(
            edit_utf16_to_byte(&value, anchor).ok_or(EditError::InvalidSelection)?,
            edit_utf16_to_byte(&value, focus).ok_or(EditError::InvalidSelection)?,
        )
    }
    pub fn select_all(&self) {
        let _ = self.select(0, self.value().len());
    }
    fn remember(m: &mut Model) {
        if m.composition.is_none() {
            m.undo.push(m.current.clone());
            trim_history(&mut m.undo);
            m.redo.clear();
        }
    }
    /// Synchronize a browser input transaction. Invalid UTF-16 is rejected before
    /// either text or history changes. Composition becomes one undo transaction.
    pub fn sync(
        &self,
        value: impl Into<String>,
        anchor_utf16: usize,
        focus_utf16: usize,
        composing: bool,
    ) -> Result<(), EditError> {
        let value = value.into();
        if value.len() > MAX_EDIT_BYTES {
            return Err(EditError::TooLarge);
        }
        let anchor = edit_utf16_to_byte(&value, anchor_utf16).ok_or(EditError::InvalidSelection)?;
        let focus = edit_utf16_to_byte(&value, focus_utf16).ok_or(EditError::InvalidSelection)?;
        let mut m = self.0.borrow_mut();
        if m.read_only && value != m.current.value {
            return Err(EditError::ReadOnly);
        }
        if composing && m.composition.is_none() {
            m.composition = Some(m.current.clone());
            m.preedit = Some(m.current.selection);
        }
        if value != m.current.value {
            Self::remember(&mut m);
        }
        m.current = Checkpoint {
            selection: Selection {
                anchor: floor_grapheme(&value, anchor),
                focus: floor_grapheme(&value, focus),
            },
            value,
        };
        if composing {
            if let Some(before) = &m.composition {
                let common_prefix = before
                    .value
                    .chars()
                    .zip(m.current.value.chars())
                    .take_while(|(a, b)| a == b)
                    .map(|(_, c)| c.len_utf8())
                    .sum::<usize>();
                let suffix_limit = before
                    .value
                    .len()
                    .min(m.current.value.len())
                    .saturating_sub(common_prefix);
                let mut suffix_bytes = 0;
                for (a, b) in before
                    .value
                    .chars()
                    .rev()
                    .zip(m.current.value.chars().rev())
                {
                    if a != b || suffix_bytes + b.len_utf8() > suffix_limit {
                        break;
                    }
                    suffix_bytes += b.len_utf8();
                }
                m.preedit = Some(Selection {
                    anchor: common_prefix,
                    focus: m.current.value.len() - suffix_bytes,
                });
            }
        }
        if !composing {
            Self::finish_composition(&mut m);
        }
        m.revision += 1;
        Ok(())
    }
    pub fn set_value(&self, value: impl Into<String>) -> Result<(), EditError> {
        let value = value.into();
        let end = value.encode_utf16().count();
        self.sync(value, end, end, false)
    }
    /// Application-controlled content can replace a read-only model. User
    /// editing entry points still reject every mutation of that model.
    pub(crate) fn set_controlled_value(&self, value: String) -> Result<(), EditError> {
        let read_only = self.is_read_only();
        self.set_read_only(false);
        let result = self.set_value(value);
        self.set_read_only(read_only);
        result
    }
    pub fn insert(&self, text: &str) -> Result<(), EditError> {
        self.replace_range(self.selection().range(), text)
    }
    fn replace_range(&self, range: Range<usize>, text: &str) -> Result<(), EditError> {
        let mut m = self.0.borrow_mut();
        if m.read_only {
            return Err(EditError::ReadOnly);
        }
        if m.current.value.len() - range.len() + text.len() > MAX_EDIT_BYTES {
            return Err(EditError::TooLarge);
        }
        Self::remember(&mut m);
        m.current.value.replace_range(range.clone(), text);
        // Insertion/deletion can join graphemes across the edited range, such
        // as a base before an existing combining mark or regional indicators.
        m.current.selection =
            Selection::caret(ceil_grapheme(&m.current.value, range.start + text.len()));
        m.revision += 1;
        Ok(())
    }
    pub fn paste(&self, text: &str) -> Result<(), EditError> {
        self.insert(text)
    }
    pub fn copy(&self) -> Result<String, EditError> {
        let m = self.0.borrow();
        if m.password {
            return Err(EditError::PasswordClipboard);
        }
        Ok(m.current.value[m.current.selection.range()].to_owned())
    }
    pub fn cut(&self) -> Result<String, EditError> {
        if self.is_read_only() {
            return Err(EditError::ReadOnly);
        }
        let selected = self.copy()?;
        if !selected.is_empty() {
            self.insert("")?;
        }
        Ok(selected)
    }
    pub fn delete_backward(&self) -> Result<(), EditError> {
        let mut selection = self.selection();
        if selection.is_collapsed() {
            selection.anchor = previous_grapheme(&self.value(), selection.focus);
        }
        self.replace_range(selection.range(), "")
    }
    pub fn delete_forward(&self) -> Result<(), EditError> {
        let mut selection = self.selection();
        if selection.is_collapsed() {
            selection.focus = next_grapheme(&self.value(), selection.focus);
        }
        self.replace_range(selection.range(), "")
    }
    /// Logical grapheme motion. Visual bidi navigation uses shaped cluster order
    /// and calls `select`; source order is never confused with visual order.
    pub fn move_grapheme(&self, direction: MoveDirection, extend: bool) {
        let value = self.value();
        let selection = self.selection();
        let position = if !extend
            && !selection.is_collapsed()
            && matches!(direction, MoveDirection::Backward | MoveDirection::Forward)
        {
            if direction == MoveDirection::Backward {
                selection.range().start
            } else {
                selection.range().end
            }
        } else {
            match direction {
                MoveDirection::Backward => previous_grapheme(&value, selection.focus),
                MoveDirection::Forward => next_grapheme(&value, selection.focus),
                MoveDirection::Start => 0,
                MoveDirection::End => value.len(),
            }
        };
        let _ = self.select(if extend { selection.anchor } else { position }, position);
    }
    /// Logical Unicode-word navigation. Punctuation and whitespace separate
    /// words; UTF-8 offsets always pass through grapheme-safe selection.
    pub fn move_word(&self, direction: MoveDirection, extend: bool) {
        let value = self.value();
        let selection = self.selection();
        let position = match direction {
            MoveDirection::Backward => value
                .unicode_word_indices()
                .map(|(byte, _)| byte)
                .take_while(|byte| *byte < selection.focus)
                .last()
                .unwrap_or(0),
            MoveDirection::Forward => value
                .unicode_word_indices()
                .map(|(byte, _)| byte)
                .find(|byte| *byte > selection.focus)
                .unwrap_or(value.len()),
            MoveDirection::Start => 0,
            MoveDirection::End => value.len(),
        };
        let _ = self.select(if extend { selection.anchor } else { position }, position);
    }
    pub fn composition_start(&self) -> Result<(), EditError> {
        let mut m = self.0.borrow_mut();
        if m.read_only {
            return Err(EditError::ReadOnly);
        }
        if m.composition.is_none() {
            m.composition = Some(m.current.clone());
            m.preedit = Some(m.current.selection);
        }
        Ok(())
    }
    pub fn composition_update(&self, text: &str) -> Result<(), EditError> {
        self.composition_start()?;
        let mut m = self.0.borrow_mut();
        let start = m.composition.as_ref().unwrap().selection.range().start;
        let range = m.preedit.unwrap_or(m.current.selection).range();
        if m.current.value.len() - range.len() + text.len() > MAX_EDIT_BYTES {
            return Err(EditError::TooLarge);
        }
        m.current.value.replace_range(range, text);
        // Preedit can join a grapheme with adjoining text. Keep the visible
        // caret on a boundary while retaining the exact replacement span for
        // subsequent composition updates, so they preserve those neighbors.
        m.current.selection = Selection::caret(ceil_grapheme(&m.current.value, start + text.len()));
        m.preedit = Some(Selection {
            anchor: start,
            focus: start + text.len(),
        });
        m.revision += 1;
        Ok(())
    }
    fn finish_composition(m: &mut Model) {
        if let Some(before) = m.composition.take() {
            if before.value != m.current.value {
                m.undo.push(before);
                trim_history(&mut m.undo);
                m.redo.clear();
            }
        }
        m.preedit = None;
    }
    pub fn composition_commit(&self) {
        let mut m = self.0.borrow_mut();
        Self::finish_composition(&mut m);
        m.revision += 1;
    }
    pub fn composition_cancel(&self) {
        let mut m = self.0.borrow_mut();
        if let Some(before) = m.composition.take() {
            m.current = before;
        }
        m.preedit = None;
        m.revision += 1;
    }
    pub fn undo(&self) -> bool {
        let mut m = self.0.borrow_mut();
        if m.read_only {
            return false;
        }
        Self::finish_composition(&mut m);
        if let Some(before) = m.undo.pop() {
            let current = std::mem::replace(&mut m.current, before);
            m.redo.push(current);
            trim_history(&mut m.redo);
            m.revision += 1;
            true
        } else {
            false
        }
    }
    pub fn redo(&self) -> bool {
        let mut m = self.0.borrow_mut();
        if m.read_only {
            return false;
        }
        if let Some(after) = m.redo.pop() {
            let current = std::mem::replace(&mut m.current, after);
            m.undo.push(current);
            trim_history(&mut m.undo);
            m.revision += 1;
            true
        } else {
            false
        }
    }
    pub fn history_len(&self) -> (usize, usize) {
        let m = self.0.borrow();
        (m.undo.len(), m.redo.len())
    }
}
