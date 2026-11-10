//! Recoverable application failures are explicit values; a fatal Wasm trap
//! requires a fresh host session and is never represented as a caught panic.
use crate::{dynamic, Node, Scope, Signal, View, Widget};
use std::rc::Rc;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppError {
    pub code: String,
    pub message: String,
}
impl AppError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        let code = code.into().chars().take(64).collect();
        let message = message.into().chars().take(1024).collect();
        Self { code, message }
    }
}
#[derive(Clone)]
pub struct ErrorBoundary {
    pub error: Signal<Option<AppError>>,
}
impl ErrorBoundary {
    pub fn new(cx: &Scope) -> Self {
        Self {
            error: cx.signal(None),
        }
    }
    pub fn report(&self, error: AppError) {
        let _ = self.error.set(Some(error));
    }
    pub fn reset(&self) {
        let _ = self.error.set(None);
    }
    pub fn run<T>(&self, operation: impl FnOnce() -> Result<T, AppError>) -> Option<T> {
        match operation() {
            Ok(value) => Some(value),
            Err(error) => {
                self.report(error);
                None
            }
        }
    }
}
/// The failed subtree is disposed. Reset mounts a fresh subtree, cancelling
/// its former scoped tasks and requests through normal lifecycle ownership.
pub fn error_boundary(state: ErrorBoundary, content: impl Fn(Scope) -> View + 'static) -> View {
    let content = Rc::new(content);
    dynamic(move || match state.error.get() {
        None => {
            let content = content.clone();
            Node::component("ErrorBoundary.Content", move |cx| content(cx)).key("ready")
        }
        Some(error) => {
            let retry = state.clone();
            Node::widget(Widget::Column)
                .role("alert")
                .key("error")
                .label("Application error")
                .children(vec![
                    Node::text(error.message),
                    Node::widget(Widget::Button)
                        .value("Try again")
                        .on_click(move |_| retry.reset()),
                ])
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Runtime;
    use std::cell::RefCell;
    #[test]
    fn recoverable_error_disposes_then_remounts_child() {
        let state = Rc::new(RefCell::new(None));
        let saved = state.clone();
        let mounts = Rc::new(std::cell::Cell::new(0));
        let count = mounts.clone();
        let mut runtime = Runtime::new(move |cx| {
            let boundary = ErrorBoundary::new(&cx);
            *saved.borrow_mut() = Some(boundary.clone());
            let count = count.clone();
            error_boundary(boundary, move |_| {
                count.set(count.get() + 1);
                Node::text("content")
            })
        });
        runtime.flush();
        let boundary = state.borrow().clone().unwrap();
        assert_eq!(mounts.get(), 1);
        boundary.report(AppError::new("decode", "Unable to load this view"));
        runtime.flush();
        assert_eq!(mounts.get(), 1);
        boundary.reset();
        runtime.flush();
        assert_eq!(mounts.get(), 2);
    }
}
