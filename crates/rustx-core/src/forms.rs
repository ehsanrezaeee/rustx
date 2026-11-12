//! Scoped controlled forms with explicit validation and submission generations.
use crate::{Scope, Signal};
use serde::{ser::SerializeStruct, Serialize, Serializer};
use std::{
    cell::RefCell,
    collections::BTreeMap,
    fmt,
    rc::{Rc, Weak},
};

type Validator = Rc<dyn Fn(&str) -> Option<String>>;
type FormValidator = Rc<dyn Fn(&BTreeMap<String, String>) -> BTreeMap<String, String>>;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Controlledness {
    Controlled,
    Uncontrolled,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FormError {
    Disposed,
    DuplicateField,
    UnknownField,
    ControllednessChanged,
    Invalid,
    SubmissionInProgress,
}

#[derive(Clone)]
pub struct FieldState {
    pub value: String,
    pub initial: String,
    pub dirty: bool,
    pub touched: bool,
    pub error: Option<String>,
    pub validating: bool,
    pub controlledness: Controlledness,
    pub sensitive: bool,
    generation: u64,
    async_error: Option<String>,
}
impl fmt::Debug for FieldState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FieldState")
            .field(
                "value",
                &if self.sensitive {
                    "[redacted]"
                } else {
                    &self.value
                },
            )
            .field(
                "initial",
                &if self.sensitive {
                    "[redacted]"
                } else {
                    &self.initial
                },
            )
            .field("dirty", &self.dirty)
            .field("touched", &self.touched)
            .field("error", &self.error)
            .field("validating", &self.validating)
            .field("controlledness", &self.controlledness)
            .finish()
    }
}
impl Serialize for FieldState {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("FieldState", 8)?;
        state.serialize_field(
            "value",
            if self.sensitive {
                "[redacted]"
            } else {
                &self.value
            },
        )?;
        state.serialize_field(
            "initial",
            if self.sensitive {
                "[redacted]"
            } else {
                &self.initial
            },
        )?;
        state.serialize_field("dirty", &self.dirty)?;
        state.serialize_field("touched", &self.touched)?;
        state.serialize_field("error", &self.error)?;
        state.serialize_field("validating", &self.validating)?;
        state.serialize_field("controlledness", &self.controlledness)?;
        state.serialize_field("sensitive", &self.sensitive)?;
        state.end()
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct FormSnapshot {
    pub fields: BTreeMap<String, FieldState>,
    pub dirty: bool,
    pub touched: bool,
    pub valid: bool,
    pub validating: bool,
    pub submitting: bool,
    pub submit_error: Option<String>,
    pub form_error: Option<String>,
    generation: u64,
    submission: u64,
}
impl Default for FormSnapshot {
    fn default() -> Self {
        Self {
            fields: BTreeMap::new(),
            dirty: false,
            touched: false,
            valid: true,
            validating: false,
            submitting: false,
            submit_error: None,
            form_error: None,
            generation: 0,
            submission: 0,
        }
    }
}
impl FormSnapshot {
    fn aggregate(&mut self) {
        self.dirty = self.fields.values().any(|f| f.dirty);
        self.touched = self.fields.values().any(|f| f.touched);
        self.validating = self.fields.values().any(|f| f.validating);
        self.valid = self.form_error.is_none()
            && !self.validating
            && self.fields.values().all(|f| f.error.is_none());
    }
}
struct FormInner {
    state: Signal<FormSnapshot>,
    validators: RefCell<BTreeMap<String, Vec<Validator>>>,
    form_validators: RefCell<Vec<FormValidator>>,
}
#[derive(Clone)]
pub struct FormState(Rc<FormInner>);
#[derive(Clone)]
pub struct Field {
    form: FormState,
    name: String,
}
/// A ticket is bound to the field value and form revision that created it.
#[derive(Clone)]
pub struct ValidationTicket {
    owner: Weak<FormInner>,
    name: String,
    generation: u64,
    value: String,
}
impl fmt::Debug for ValidationTicket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ValidationTicket")
            .field("field", &self.name)
            .field("generation", &self.generation)
            .finish()
    }
}
impl ValidationTicket {
    pub fn value(&self) -> &str {
        &self.value
    }
    pub fn field(&self) -> &str {
        &self.name
    }
}
#[derive(Clone)]
pub struct SubmissionTicket {
    owner: Weak<FormInner>,
    generation: u64,
    submission: u64,
}
impl fmt::Debug for SubmissionTicket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SubmissionTicket")
            .field("generation", &self.generation)
            .field("submission", &self.submission)
            .finish()
    }
}

pub fn required(message: impl Into<String>) -> impl Fn(&str) -> Option<String> {
    let message = message.into();
    move |value| value.trim().is_empty().then(|| message.clone())
}
pub fn minimum_length(
    minimum: usize,
    message: impl Into<String>,
) -> impl Fn(&str) -> Option<String> {
    let message = message.into();
    move |value| {
        (unicode_segmentation::UnicodeSegmentation::graphemes(value, true).count() < minimum)
            .then(|| message.clone())
    }
}
impl FormState {
    pub fn new(cx: &Scope) -> Self {
        Self(Rc::new(FormInner {
            state: cx.signal(FormSnapshot::default()),
            validators: RefCell::new(BTreeMap::new()),
            form_validators: RefCell::new(vec![]),
        }))
    }
    pub fn state(&self) -> FormSnapshot {
        self.0.state.get()
    }
    pub fn try_state(&self) -> Result<FormSnapshot, FormError> {
        self.0.state.peek().map_err(|_| FormError::Disposed)
    }
    pub fn values(&self) -> BTreeMap<String, String> {
        self.state()
            .fields
            .into_iter()
            .map(|(name, f)| (name, f.value))
            .collect()
    }
    fn write(&self, mut state: FormSnapshot) -> Result<(), FormError> {
        state.aggregate();
        self.0.state.set(state).map_err(|_| FormError::Disposed)
    }
    pub fn field(&self, name: impl Into<String>, initial: impl Into<String>) -> Field {
        self.try_field(name, initial, Controlledness::Controlled)
            .expect("form field registration failed")
    }
    pub fn uncontrolled_field(&self, name: impl Into<String>, initial: impl Into<String>) -> Field {
        self.try_field(name, initial, Controlledness::Uncontrolled)
            .expect("form field registration failed")
    }
    pub fn try_field(
        &self,
        name: impl Into<String>,
        initial: impl Into<String>,
        controlledness: Controlledness,
    ) -> Result<Field, FormError> {
        let name = name.into();
        let initial = initial.into();
        let mut state = self.try_state()?;
        if state.fields.contains_key(&name) {
            return Err(FormError::DuplicateField);
        }
        state.fields.insert(
            name.clone(),
            FieldState {
                value: initial.clone(),
                initial,
                dirty: false,
                touched: false,
                error: None,
                validating: false,
                controlledness,
                sensitive: false,
                generation: 0,
                async_error: None,
            },
        );
        self.write(state)?;
        Ok(Field {
            form: self.clone(),
            name,
        })
    }
    pub fn validator(
        self,
        validator: impl Fn(&BTreeMap<String, String>) -> BTreeMap<String, String> + 'static,
    ) -> Self {
        self.0.form_validators.borrow_mut().push(Rc::new(validator));
        self
    }
    pub fn validate(&self) -> bool {
        let Ok(mut state) = self.try_state() else {
            return false;
        };
        for (name, field) in &mut state.fields {
            field.touched = true;
            // Keep an in-flight async check in flight. A synchronous submit may
            // never silently convert pending validation into success.
            field.error = self
                .0
                .validators
                .borrow()
                .get(name)
                .and_then(|validators| validators.iter().find_map(|v| v(&field.value)))
                .or_else(|| field.async_error.clone());
        }
        state.form_error = None;
        let values: BTreeMap<_, _> = state
            .fields
            .iter()
            .map(|(n, f)| (n.clone(), f.value.clone()))
            .collect();
        for validator in self.0.form_validators.borrow().iter() {
            for (name, error) in validator(&values) {
                if let Some(field) = state.fields.get_mut(&name) {
                    if field.error.is_none() {
                        field.error = Some(error);
                    }
                } else {
                    state.form_error = Some(error);
                }
            }
        }
        state.aggregate();
        let valid = state.valid;
        self.write(state).is_ok() && valid
    }
    pub fn begin_submit(&self) -> Result<SubmissionTicket, FormError> {
        if self.try_state()?.submitting {
            return Err(FormError::SubmissionInProgress);
        }
        if !self.validate() {
            return Err(FormError::Invalid);
        }
        let mut state = self.try_state()?;
        state.submitting = true;
        state.submit_error = None;
        state.submission = state.submission.wrapping_add(1);
        let ticket = SubmissionTicket {
            owner: Rc::downgrade(&self.0),
            generation: state.generation,
            submission: state.submission,
        };
        self.write(state)?;
        Ok(ticket)
    }
    pub fn finish_submit(&self, ticket: SubmissionTicket, result: Result<(), String>) -> bool {
        let Ok(mut state) = self.try_state() else {
            return false;
        };
        if !Weak::ptr_eq(&ticket.owner, &Rc::downgrade(&self.0))
            || !state.submitting
            || state.generation != ticket.generation
            || state.submission != ticket.submission
        {
            return false;
        }
        state.submitting = false;
        state.submit_error = result.err();
        self.write(state).is_ok()
    }
    pub fn reset(&self) -> Result<(), FormError> {
        let mut state = self.try_state()?;
        for field in state.fields.values_mut() {
            field.value = field.initial.clone();
            field.dirty = false;
            field.touched = false;
            field.error = None;
            field.async_error = None;
            field.validating = false;
            field.generation = field.generation.wrapping_add(1);
        }
        state.generation = state.generation.wrapping_add(1);
        state.submitting = false;
        state.submit_error = None;
        state.form_error = None;
        self.write(state)
    }
}
impl Field {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn state(&self) -> FieldState {
        self.form.state().fields[&self.name].clone()
    }
    pub fn value(&self) -> String {
        self.state().value
    }
    pub fn error_id(&self) -> String {
        format!("{}-error", self.name)
    }
    pub fn validator(self, validator: impl Fn(&str) -> Option<String> + 'static) -> Self {
        self.form
            .0
            .validators
            .borrow_mut()
            .entry(self.name.clone())
            .or_default()
            .push(Rc::new(validator));
        self
    }
    pub fn sensitive(self) -> Self {
        if let Ok(mut state) = self.form.try_state() {
            if let Some(field) = state.fields.get_mut(&self.name) {
                field.sensitive = true;
            }
            let _ = self.form.write(state);
        }
        self
    }
    pub fn set(&self, value: impl Into<String>) -> Result<(), FormError> {
        let mut state = self.form.try_state()?;
        let field = state
            .fields
            .get_mut(&self.name)
            .ok_or(FormError::UnknownField)?;
        let value = value.into();
        if field.value != value {
            field.value = value;
            field.dirty = field.value != field.initial;
            field.error = None;
            field.async_error = None;
            field.generation = field.generation.wrapping_add(1);
            field.validating = false;
            // An edited form supersedes the submitted snapshot.
            state.generation = state.generation.wrapping_add(1);
            state.submitting = false;
            state.submit_error = None;
            state.form_error = None;
        }
        self.form.write(state)
    }
    pub fn touch(&self) -> Result<(), FormError> {
        let mut state = self.form.try_state()?;
        state
            .fields
            .get_mut(&self.name)
            .ok_or(FormError::UnknownField)?
            .touched = true;
        self.form.write(state)
    }
    pub fn validate(&self) -> bool {
        let Ok(mut state) = self.form.try_state() else {
            return false;
        };
        let Some(field) = state.fields.get_mut(&self.name) else {
            return false;
        };
        field.touched = true;
        field.error = self
            .form
            .0
            .validators
            .borrow()
            .get(&self.name)
            .and_then(|validators| validators.iter().find_map(|v| v(&field.value)))
            .or_else(|| field.async_error.clone());
        let valid = field.error.is_none() && !field.validating;
        self.form.write(state).is_ok() && valid
    }
    pub fn set_controlledness(&self, controlledness: Controlledness) -> Result<(), FormError> {
        let state = self.form.try_state()?;
        let field = state
            .fields
            .get(&self.name)
            .ok_or(FormError::UnknownField)?;
        if field.controlledness != controlledness {
            Err(FormError::ControllednessChanged)
        } else {
            Ok(())
        }
    }
    pub fn begin_validation(&self) -> Result<ValidationTicket, FormError> {
        let mut state = self.form.try_state()?;
        let field = state
            .fields
            .get_mut(&self.name)
            .ok_or(FormError::UnknownField)?;
        field.generation = field.generation.wrapping_add(1);
        field.validating = true;
        field.touched = true;
        let ticket = ValidationTicket {
            owner: Rc::downgrade(&self.form.0),
            name: self.name.clone(),
            generation: field.generation,
            value: field.value.clone(),
        };
        self.form.write(state)?;
        Ok(ticket)
    }
    pub fn finish_validation(&self, ticket: ValidationTicket, error: Option<String>) -> bool {
        let Ok(mut state) = self.form.try_state() else {
            return false;
        };
        let Some(field) = state.fields.get_mut(&self.name) else {
            return false;
        };
        if !Weak::ptr_eq(&ticket.owner, &Rc::downgrade(&self.form.0))
            || ticket.name != self.name
            || ticket.generation != field.generation
            || ticket.value != field.value
            || !field.validating
        {
            return false;
        }
        field.validating = false;
        field.async_error = error.clone();
        // Async success clears only the async result. Synchronous field and
        // form validators still apply to the current value, without touching
        // unrelated fields or invalidating this completion's generation.
        field.error = self
            .form
            .0
            .validators
            .borrow()
            .get(&self.name)
            .and_then(|validators| validators.iter().find_map(|v| v(&field.value)))
            .or(error);
        if field.error.is_none() {
            let values: BTreeMap<_, _> = state
                .fields
                .iter()
                .map(|(name, field)| (name.clone(), field.value.clone()))
                .collect();
            let error = self
                .form
                .0
                .form_validators
                .borrow()
                .iter()
                .find_map(|validator| validator(&values).remove(&self.name));
            state.fields.get_mut(&self.name).unwrap().error = error;
        }
        self.form.write(state).is_ok()
    }
}
