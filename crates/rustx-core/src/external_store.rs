//! Scope-owned subscriptions to immutable snapshots published outside UI commits.
use crate::{Disposed, Scope, Signal};
use std::{
    any::Any,
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::{Rc, Weak},
};

pub const MAX_STORE_SUBSCRIPTIONS: usize = 64;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExternalStoreError {
    Disposed,
    Busy,
    Capacity,
    Exhausted,
}

/// A coherent revision/value pair. Values must not be mutated through interior mutability.
pub struct StoreSnapshot<T> {
    pub revision: u64,
    pub value: Rc<T>,
}
impl<T> Clone for StoreSnapshot<T> {
    fn clone(&self) -> Self {
        Self {
            revision: self.revision,
            value: self.value.clone(),
        }
    }
}

struct StoreInner<T: 'static> {
    snapshot: RefCell<StoreSnapshot<T>>,
    subscriptions: RefCell<HashMap<u64, Weak<Binding<T>>>>,
    next: Cell<u64>,
    publishing: Cell<bool>,
}
/// A UI-thread store independent of any component or Runtime. Share clones across roots.
pub struct ExternalStore<T: 'static>(Rc<StoreInner<T>>);
impl<T> Clone for ExternalStore<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
struct Binding<T: 'static> {
    scope: Scope,
    signal: Signal<StoreSnapshot<T>>,
    source: RefCell<Option<ExternalStore<T>>>,
    id: Cell<u64>,
    cleanup: Cell<Option<u64>>,
}
/// Read-only reactive subscription. The owner retains it until disposal or disconnect.
pub struct StoreSubscription<T: 'static>(Rc<Binding<T>>);
impl<T> Clone for StoreSubscription<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
struct PublishGuard<'a>(&'a Cell<bool>);
impl Drop for PublishGuard<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

impl<T: 'static> ExternalStore<T> {
    pub fn new(value: T) -> Self {
        Self(Rc::new(StoreInner {
            snapshot: RefCell::new(StoreSnapshot {
                revision: 0,
                value: Rc::new(value),
            }),
            subscriptions: RefCell::new(HashMap::new()),
            next: Cell::new(0),
            publishing: Cell::new(false),
        }))
    }
    /// Untracked read; components should read their scoped subscription instead.
    pub fn snapshot(&self) -> StoreSnapshot<T> {
        self.0.snapshot.borrow().clone()
    }
    pub fn subscriber_count(&self) -> usize {
        self.0.subscriptions.borrow().len()
    }
    fn reserve(&self) -> Result<u64, ExternalStoreError> {
        if self.subscriber_count() >= MAX_STORE_SUBSCRIPTIONS {
            return Err(ExternalStoreError::Capacity);
        }
        let id = self
            .0
            .next
            .get()
            .checked_add(1)
            .ok_or(ExternalStoreError::Exhausted)?;
        self.0.next.set(id);
        Ok(id)
    }
    pub fn subscribe(&self, scope: &Scope) -> Result<StoreSubscription<T>, ExternalStoreError> {
        if !scope.is_alive() {
            return Err(ExternalStoreError::Disposed);
        }
        let id = self.reserve()?;
        let binding = Rc::new(Binding {
            scope: scope.clone(),
            signal: scope.signal(self.snapshot()),
            source: RefCell::new(Some(self.clone())),
            id: Cell::new(id),
            cleanup: Cell::new(None),
        });
        self.0
            .subscriptions
            .borrow_mut()
            .insert(id, Rc::downgrade(&binding));
        let owned = binding.clone();
        let cleanup = scope.on_cleanup(move || owned.detach());
        binding.cleanup.set(Some(cleanup));
        Ok(StoreSubscription(binding))
    }
}
impl<T: PartialEq + 'static> ExternalStore<T> {
    /// Publish atomically to subscriptions, then wake each runtime once.
    /// Busy leaves all snapshots unchanged; retry from the host after the current commit.
    pub fn publish(&self, value: T) -> Result<bool, ExternalStoreError> {
        if self.0.publishing.replace(true) {
            return Err(ExternalStoreError::Busy);
        }
        let _guard = PublishGuard(&self.0.publishing);
        let old = self.snapshot();
        if *old.value == value {
            return Ok(false);
        }
        let bindings: Vec<_> = self
            .0
            .subscriptions
            .borrow()
            .values()
            .filter_map(Weak::upgrade)
            .collect();
        if bindings.iter().any(|b| b.committing()) {
            return Err(ExternalStoreError::Busy);
        }
        let revision = old
            .revision
            .checked_add(1)
            .ok_or(ExternalStoreError::Exhausted)?;
        let next = StoreSnapshot {
            revision,
            value: Rc::new(value),
        };
        let retired = self.0.snapshot.replace(next.clone());
        let mut wakes = HashMap::new();
        let mut previous = Vec::new();
        for binding in &bindings {
            if let Some((value, wake)) = binding.write(next.clone()) {
                previous.push(value);
                if let Some((id, wake)) = wake {
                    wakes.insert(id, wake);
                }
            }
        }
        // Every subscriber has the same revision before any external code runs.
        for (id, wake) in wakes {
            // An earlier synchronous wake may dispose or disconnect another root.
            if bindings.iter().any(|b| {
                b.scope.inner.as_ptr() as usize == id
                    && b.scope.is_alive()
                    && b.source.borrow().is_some()
            }) {
                wake();
            }
        }
        drop(previous);
        drop(retired);
        Ok(true)
    }
}

type Wake = Option<(usize, Rc<dyn Fn()>)>;
impl<T: 'static> Binding<T> {
    fn committing(&self) -> bool {
        self.scope
            .inner
            .upgrade()
            .is_some_and(|i| i.borrow().committing.get())
    }
    fn detach(&self) {
        let source = self.source.borrow_mut().take();
        if let Some(source) = source {
            source.0.subscriptions.borrow_mut().remove(&self.id.get());
        }
        if let Some(id) = self.cleanup.take() {
            self.scope.remove_cleanup(id);
        }
    }
    fn write(&self, snapshot: StoreSnapshot<T>) -> Option<(Box<dyn Any>, Wake)> {
        let i = self.scope.inner.upgrade()?;
        let mut b = i.borrow_mut();
        let value = b.values.get_mut(&self.signal.id)?;
        let retired = std::mem::replace(&mut value.value, Box::new(snapshot));
        let subscribers = value.subscribers.clone();
        b.dirty.extend(subscribers);
        let wake = b.wake.clone().map(|wake| (Rc::as_ptr(&i) as usize, wake));
        Some((retired, wake))
    }
}
impl<T: 'static> StoreSubscription<T> {
    pub fn try_get(&self) -> Result<StoreSnapshot<T>, Disposed> {
        if self.0.source.borrow().is_none() {
            return Err(Disposed);
        }
        self.0.signal.try_get()
    }
    pub fn get(&self) -> StoreSnapshot<T> {
        self.try_get()
            .expect("external store subscription disposed")
    }
    /// Idempotently unsubscribe. Signal storage is released with its owning scope.
    pub fn disconnect(&self) {
        self.0.detach();
    }
    /// Switch sources without allocating a new signal or cleanup registration.
    pub fn replace_source(&self, source: &ExternalStore<T>) -> Result<(), ExternalStoreError> {
        if !self.0.scope.is_alive() {
            return Err(ExternalStoreError::Disposed);
        }
        let old = self
            .0
            .source
            .borrow()
            .clone()
            .ok_or(ExternalStoreError::Disposed)?;
        if self.0.committing() || old.0.publishing.get() || source.0.publishing.get() {
            return Err(ExternalStoreError::Busy);
        }
        if Rc::ptr_eq(&old.0, &source.0) {
            return Ok(());
        }
        let id = source.reserve()?;
        source
            .0
            .subscriptions
            .borrow_mut()
            .insert(id, Rc::downgrade(&self.0));
        old.0.subscriptions.borrow_mut().remove(&self.0.id.get());
        self.0.id.set(id);
        self.0.source.replace(Some(source.clone()));
        if let Some((retired, wake)) = self.0.write(source.snapshot()) {
            if let Some((_, wake)) = wake {
                wake();
            }
            drop(retired);
        }
        Ok(())
    }
}
