//! The Markdown preview's document is derived once per text.

use std::cell::Cell;
use std::rc::Rc;

use vessel_proto::workspace::RelPath;

use super::{Look, PreviewKey};

fn readme() -> Look {
    let path: RelPath = "README.md".parse().expect("a relative path");
    Look::of(&path, false, true)
}

#[test]
fn renders_of_one_text_derive_its_document_once() {
    let mut look = readme();
    let derived = Cell::new(0_u32);
    let text = |words: &str| {
        derived.set(derived.get().saturating_add(1));
        format!("---\ntitle: Notes\n---\n{words}\n")
    };
    let first = look.preview(PreviewKey::Edit(1), || text("one"));
    let again = look.preview(PreviewKey::Edit(1), || text("one"));
    assert!(Rc::ptr_eq(&first, &again));
    assert_eq!(derived.get(), 1);
    let document = first.document.as_ref().expect("within the budget");
    assert_eq!(document.front_matter.len(), 1);
    assert_eq!(document.body, "one\n");

    // Typed: a new revision is derived again.
    let typed = look.preview(PreviewKey::Edit(2), || text("one two"));
    assert_eq!(derived.get(), 2);
    assert_ne!(typed.version, first.version);

    // The read shown is kept until a new read replaces it.
    let _read = look.preview(PreviewKey::Read, || text("read"));
    let _kept = look.preview(PreviewKey::Read, || text("read"));
    assert_eq!(derived.get(), 3);
    look.text_read();
    let reread = look.preview(PreviewKey::Read, || text("read again"));
    assert_eq!(derived.get(), 4);
    assert_eq!(
        reread.document.as_ref().map(|document| document.body.as_str()),
        Some("read again\n")
    );
}
