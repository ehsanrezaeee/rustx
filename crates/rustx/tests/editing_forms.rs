use rustx::prelude::*;
use std::{cell::RefCell, rc::Rc};

#[test]
fn grapheme_motion_and_deletion_preserve_joined_emoji_and_combining_marks() {
    let editor = Editor::new("A👩‍👩‍👧‍👦e\u{301}ب");
    editor.move_grapheme(MoveDirection::End, false);
    editor.delete_backward().unwrap();
    assert_eq!(editor.value(), "A👩‍👩‍👧‍👦e\u{301}");
    editor.delete_backward().unwrap();
    assert_eq!(editor.value(), "A👩‍👩‍👧‍👦");
    editor.delete_backward().unwrap();
    assert_eq!(editor.value(), "A");
    assert!(editor.undo());
    assert_eq!(editor.value(), "A👩‍👩‍👧‍👦");
    editor.move_grapheme(MoveDirection::Start, false);
    editor.delete_forward().unwrap();
    assert_eq!(editor.value(), "👩‍👩‍👧‍👦");
}

#[test]
fn insertion_caret_stays_on_a_grapheme_boundary_when_neighbors_join() {
    for (initial, inserted, joined) in [
        ("\u{301}", "a", "a\u{301}"),
        ("🇧", "🇦", "🇦🇧"),
        ("👩", "👨‍", "👨‍👩"),
    ] {
        let editor = Editor::new(initial);
        editor.insert(inserted).unwrap();
        assert_eq!(editor.value(), joined);
        assert_eq!(editor.selection(), Selection::caret(joined.len()));
        editor.delete_backward().unwrap();
        assert_eq!(editor.value(), "");
        assert!(editor.undo());
        assert_eq!(editor.value(), joined);
        assert_eq!(editor.selection(), Selection::caret(joined.len()));
        assert!(editor.undo());
        assert_eq!(editor.value(), initial);
        assert_eq!(editor.selection(), Selection::caret(0));
    }
}

#[test]
fn readonly_deletion_preserves_selection_and_editable_deletion_undo_restores_it() {
    let editor = Editor::new("A🌍B");
    editor.select(5, 5).unwrap();
    editor.set_read_only(true);
    assert_eq!(editor.delete_backward(), Err(EditError::ReadOnly));
    assert_eq!(editor.delete_forward(), Err(EditError::ReadOnly));
    assert_eq!(editor.selection(), Selection::caret(5));
    assert_eq!(editor.history_len(), (0, 0));
    editor.set_read_only(false);
    editor.delete_backward().unwrap();
    assert_eq!(editor.value(), "AB");
    assert!(editor.undo());
    assert_eq!(editor.value(), "A🌍B");
    assert_eq!(editor.selection(), Selection::caret(5));
    editor.delete_forward().unwrap();
    assert_eq!(editor.value(), "A🌍");
    assert!(editor.undo());
    assert_eq!(editor.selection(), Selection::caret(5));
}

#[test]
fn word_navigation_uses_unicode_words_and_extends_grapheme_safe_ranges() {
    let text = "one، سلام e\u{301} two";
    let editor = Editor::new(text);
    editor.move_word(MoveDirection::Forward, false);
    assert_eq!(
        editor.selection(),
        Selection::caret(text.find("سلام").unwrap())
    );
    editor.move_word(MoveDirection::Forward, true);
    assert_eq!(editor.copy().unwrap(), "سلام ");
    editor.move_word(MoveDirection::Forward, false);
    assert_eq!(
        editor.selection(),
        Selection::caret(text.find("two").unwrap())
    );
    editor.move_word(MoveDirection::Backward, true);
    assert_eq!(editor.copy().unwrap(), "e\u{301} ");
    editor.move_word(MoveDirection::Start, false);
    editor.move_word(MoveDirection::Backward, false);
    assert_eq!(editor.selection(), Selection::caret(0));
    editor.move_word(MoveDirection::End, false);
    editor.move_word(MoveDirection::Forward, false);
    assert_eq!(editor.selection(), Selection::caret(text.len()));
}

#[test]
fn utf16_validation_is_atomic_and_bidi_selections_keep_source_ranges() {
    let editor = Editor::new("a😀سلام");
    editor.select_utf16(3, 7).unwrap();
    assert_eq!(editor.copy().unwrap(), "سلام");
    let previous = editor.value();
    assert_eq!(
        editor.sync("x😀y", 2, 2, false),
        Err(EditError::InvalidSelection)
    );
    assert_eq!(editor.value(), previous);
    assert_eq!(editor.select_utf16(2, 2), Err(EditError::InvalidSelection));
    editor.select_utf16(7, 3).unwrap();
    assert_eq!(editor.cut().unwrap(), "سلام");
    assert_eq!(editor.value(), "a😀");
    editor.paste("درود").unwrap();
    assert_eq!(editor.value(), "a😀درود");
}

#[test]
fn composition_is_one_undo_transaction_and_cancel_restores_selection() {
    let editor = Editor::new("hello world");
    editor.select(6, 11).unwrap();
    editor.composition_update("س").unwrap();
    editor.composition_update("سلام").unwrap();
    let state = editor.snapshot();
    assert!(state.composing);
    assert_eq!(state.preedit.unwrap().range(), 6..14);
    assert_eq!(editor.value(), "hello سلام");
    editor.composition_cancel();
    assert_eq!(editor.value(), "hello world");
    assert_eq!(
        editor.selection(),
        Selection {
            anchor: 6,
            focus: 11
        }
    );
    editor.composition_update("سلام").unwrap();
    editor.composition_commit();
    assert_eq!(editor.history_len(), (1, 0));
    assert!(editor.undo());
    assert_eq!(editor.value(), "hello world");
    assert!(editor.redo());
    assert_eq!(editor.value(), "hello سلام");
}

#[test]
fn composition_caret_preserves_joined_graphemes_without_consuming_neighbor_text() {
    for (initial, first, last, joined) in [
        ("\u{301}", "a", "e", "e\u{301}"),
        ("🇧", "🇦", "🇨", "🇨🇧"),
        ("👩", "👨‍", "👧‍", "👧‍👩"),
    ] {
        let editor = Editor::new(initial);
        editor.composition_update(first).unwrap();
        assert_eq!(editor.selection(), Selection::caret(editor.value().len()));
        editor.composition_update(last).unwrap();
        assert_eq!(editor.value(), joined);
        assert_eq!(editor.selection(), Selection::caret(joined.len()));
        assert_eq!(editor.snapshot().preedit.unwrap().range(), 0..last.len());
        editor.composition_cancel();
        assert_eq!(editor.value(), initial);
        assert_eq!(editor.selection(), Selection::caret(0));
        assert_eq!(editor.history_len(), (0, 0));

        editor.composition_update(last).unwrap();
        editor.composition_commit();
        editor.delete_backward().unwrap();
        assert_eq!(editor.value(), "");
        assert!(editor.undo());
        assert_eq!(editor.value(), joined);
        assert_eq!(editor.selection(), Selection::caret(joined.len()));
        assert!(editor.undo());
        assert_eq!(editor.value(), initial);
        assert_eq!(editor.selection(), Selection::caret(0));
    }
}

#[test]
fn browser_composition_sync_commits_once_and_invalid_input_does_not_mutate() {
    let editor = Editor::new("abc");
    editor.select(3, 3).unwrap();
    editor.sync("abcس", 4, 4, true).unwrap();
    editor.sync("abcسلام", 7, 7, true).unwrap();
    assert_eq!(editor.history_len(), (0, 0));
    editor.sync("abcسلام", 7, 7, false).unwrap();
    assert_eq!(editor.history_len(), (1, 0));
    assert!(editor.undo());
    assert_eq!(editor.value(), "abc");
    assert_eq!(
        editor.sync("x".repeat(MAX_EDIT_BYTES + 1), 0, 0, false),
        Err(EditError::TooLarge)
    );
    assert_eq!(editor.value(), "abc");
}

#[test]
fn readonly_and_password_never_expose_secret_in_diagnostics_or_clipboard() {
    let editor = Editor::new("p😀ssword").with_password(true);
    editor.select_all();
    assert_eq!(editor.copy(), Err(EditError::PasswordClipboard));
    assert_eq!(editor.cut(), Err(EditError::PasswordClipboard));
    assert_eq!(editor.display_text(), "••••••••");
    let diagnostic = format!("{editor:?}");
    assert!(!diagnostic.contains("p😀ssword"));
    assert_eq!(editor.snapshot().selection.range(), 0..24);
    editor.set_read_only(true);
    assert_eq!(editor.insert("X"), Err(EditError::ReadOnly));
    assert_eq!(editor.sync("oops", 4, 4, false), Err(EditError::ReadOnly));
    assert!(!editor.undo());
    assert_eq!(editor.value(), "p😀ssword");
}

#[test]
fn editing_history_is_bounded_and_new_edit_discards_redo_branch() {
    let editor = Editor::new("");
    for _ in 0..150 {
        editor.insert("a").unwrap();
    }
    assert_eq!(editor.history_len(), (128, 0));
    assert!(editor.undo());
    editor.insert("b").unwrap();
    assert!(!editor.redo());
    assert!(editor.value().ends_with('b'));
}

fn form_fixture() -> (Runtime, FormState, Field, Field) {
    let out = Rc::new(RefCell::new(None));
    let saved = out.clone();
    let runtime = Runtime::new(move |cx| {
        let form = FormState::new(&cx);
        let name = form.field("name", "").validator(required("Name required"));
        let email = form
            .field("email", "")
            .validator(|v| (!v.contains('@')).then(|| "Invalid email".to_owned()));
        *saved.borrow_mut() = Some((form, name, email));
        Node::text("form")
    });
    let (form, name, email) = out.borrow_mut().take().unwrap();
    (runtime, form, name, email)
}

#[test]
fn form_tracks_dirty_touched_validation_and_reset() {
    let (_runtime, form, name, email) = form_fixture();
    assert!(!form.state().dirty);
    assert!(!form.validate());
    assert_eq!(name.state().error.as_deref(), Some("Name required"));
    assert!(name.state().touched);
    assert_eq!(name.error_id(), "name-error");
    name.set("Ada").unwrap();
    email.set("ada@example.test").unwrap();
    assert!(form.validate());
    assert!(form.state().dirty);
    form.reset().unwrap();
    assert_eq!(name.value(), "");
    assert!(!form.state().dirty);
    assert!(!form.state().touched);
    assert!(form.state().fields.values().all(|f| f.error.is_none()));
}

#[test]
fn stale_validation_and_submission_are_rejected_after_edit_reset_and_disposal() {
    let (runtime, form, name, email) = form_fixture();
    name.set("first").unwrap();
    let first = name.begin_validation().unwrap();
    name.set("second").unwrap();
    let second = name.begin_validation().unwrap();
    assert!(!name.finish_validation(first, Some("stale".to_owned())));
    email.set("second@example.test").unwrap();
    assert!(name.finish_validation(second, None));
    let pending = name.begin_validation().unwrap();
    assert!(matches!(form.begin_submit(), Err(FormError::Invalid)));
    assert!(name.finish_validation(pending, Some("Unavailable name".to_owned())));
    assert!(matches!(form.begin_submit(), Err(FormError::Invalid)));
    name.set("third").unwrap();
    let ticket = form.begin_submit().unwrap();
    assert!(matches!(
        form.begin_submit(),
        Err(FormError::SubmissionInProgress)
    ));
    form.reset().unwrap();
    assert!(!form.finish_submit(ticket, Ok(())));
    let ticket = name.begin_validation().unwrap();
    drop(runtime);
    assert!(!name.finish_validation(ticket, None));
    assert_eq!(name.set("late"), Err(FormError::Disposed));
}

#[test]
fn async_validation_preserves_synchronous_errors_and_can_clear_async_only_errors() {
    let (_runtime, form, name, _email) = form_fixture();
    assert!(!name.validate());
    let pending = name.begin_validation().unwrap();
    assert!(name.finish_validation(pending, None));
    assert_eq!(name.state().error.as_deref(), Some("Name required"));
    assert!(!form.state().valid);
    let pending = name.begin_validation().unwrap();
    assert!(name.finish_validation(pending, Some("Name unavailable".to_owned())));
    assert_eq!(name.state().error.as_deref(), Some("Name required"));

    name.set("Ada").unwrap();
    let pending = name.begin_validation().unwrap();
    assert!(name.finish_validation(pending, Some("Name unavailable".to_owned())));
    assert_eq!(name.state().error.as_deref(), Some("Name unavailable"));
    assert!(!form.state().valid);
    let pending = name.begin_validation().unwrap();
    assert!(name.finish_validation(pending, None));
    assert_eq!(name.state().error, None);
    assert!(form.state().valid);
}

#[test]
fn async_validation_preserves_form_level_field_errors_without_touching_other_fields() {
    let output = Rc::new(RefCell::new(None));
    let saved = output.clone();
    let _runtime = Runtime::new(move |cx| {
        let form = FormState::new(&cx).validator(|values| {
            (values["password"] != values["repeat"])
                .then(|| ("repeat".to_owned(), "Passwords differ".to_owned()))
                .into_iter()
                .collect()
        });
        let password = form.field("password", "secret");
        let repeat = form.field("repeat", "different");
        *saved.borrow_mut() = Some((form, password, repeat));
        Node::text("form")
    });
    let (form, password, repeat) = output.borrow_mut().take().unwrap();
    let pending = repeat.begin_validation().unwrap();
    assert!(repeat.finish_validation(pending, None));
    assert_eq!(repeat.state().error.as_deref(), Some("Passwords differ"));
    assert!(!form.state().valid);
    assert!(!password.state().touched);
    repeat.set("secret").unwrap();
    let pending = repeat.begin_validation().unwrap();
    assert!(repeat.finish_validation(pending, None));
    assert_eq!(repeat.state().error, None);
    assert!(form.state().valid);
    assert!(!password.state().touched);
}

#[test]
fn tickets_cannot_cross_forms_and_controlledness_changes_are_diagnosed() {
    let (_r1, form1, name1, email1) = form_fixture();
    let (_r2, form2, name2, email2) = form_fixture();
    for (name, email) in [(&name1, &email1), (&name2, &email2)] {
        name.set("Ada").unwrap();
        email.set("ada@example.test").unwrap();
    }
    let t1 = name1.begin_validation().unwrap();
    let t2 = name2.begin_validation().unwrap();
    assert!(!name2.finish_validation(t1, None));
    assert!(name2.finish_validation(t2, None));
    let t1 = name1.begin_validation().unwrap();
    assert!(name1.finish_validation(t1, None));
    let s1 = form1.begin_submit().unwrap();
    let s2 = form2.begin_submit().unwrap();
    assert!(!form2.finish_submit(s1, Ok(())));
    assert!(form2.finish_submit(s2, Err("Server rejected request".to_owned())));
    assert_eq!(
        form2.state().submit_error.as_deref(),
        Some("Server rejected request")
    );
    assert_eq!(
        name2.set_controlledness(Controlledness::Uncontrolled),
        Err(FormError::ControllednessChanged)
    );
}

#[test]
fn form_level_validation_and_sensitive_debug_use_declared_associations() {
    let output = Rc::new(RefCell::new(None));
    let saved = output.clone();
    let _runtime = Runtime::new(move |cx| {
        let form = FormState::new(&cx).validator(|values| {
            if values["password"] != values["repeat"] {
                [("repeat".to_owned(), "Passwords differ".to_owned())].into()
            } else {
                BTreeMap::new()
            }
        });
        let password = form.field("password", "private-secret").sensitive();
        let repeat = form.field("repeat", "different").sensitive();
        *saved.borrow_mut() = Some((form, password, repeat));
        Node::text("form")
    });
    let (form, password, repeat) = output.borrow_mut().take().unwrap();
    assert!(!form.validate());
    assert_eq!(repeat.state().error.as_deref(), Some("Passwords differ"));
    assert!(!format!("{:?}", form.state()).contains("private-secret"));
    assert!(!format!("{:?}", password.begin_validation().unwrap()).contains("private-secret"));
    repeat.set("private-secret").unwrap();
    let pending = password.begin_validation().unwrap();
    password.finish_validation(pending, None);
    assert!(form.validate());
}

use std::collections::BTreeMap;
