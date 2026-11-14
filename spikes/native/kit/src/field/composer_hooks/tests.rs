//! The hooks against the real text engine and GPUI's real key bindings:
//! a passed key does exactly what it did before (the lead's conditions for
//! this control: Enter sends, Tab indents), a taken key never reaches the
//! text, an input method composing is never asked, and a text paste is
//! stopped only when the screen takes the images.

use std::cell::{Cell, RefCell};

use gpui::{ClipboardItem, Entity, Image, ImageFormat, Render, TestAppContext};

use super::*;
use crate::theme::{Scheme, Tokens};

struct Probe {
    hooked: TextField,
    /// The same composer with no hooks: what "as before" means.
    plain: TextField,
    take: Rc<Cell<bool>>,
    keys: Rc<RefCell<Vec<ComposerKey>>>,
    pasted: Rc<RefCell<Vec<Pasted>>>,
    submitted: Vec<String>,
    edits: usize,
}

impl Probe {
    fn new(window: &mut Window, cx: &mut Context<'_, Self>) -> Self {
        let on_submit: fn(&mut Self, String, &mut Window, &mut Context<'_, Self>) =
            |probe, text, _, _| probe.submitted.push(text);
        let hooked = TextField::composer("Message", on_submit, window, cx);
        let plain = TextField::composer("Message", on_submit, window, cx);
        hooked.focus(window, cx);
        hooked.on_edit(
            |probe: &mut Self, _, _| probe.edits = probe.edits.saturating_add(1),
            window,
            cx,
        );
        Self {
            hooked,
            plain,
            take: Rc::new(Cell::new(false)),
            keys: Rc::new(RefCell::new(Vec::new())),
            pasted: Rc::new(RefCell::new(Vec::new())),
            submitted: Vec::new(),
            edits: 0,
        }
    }
}

impl Render for Probe {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(Scheme::Dark);
        let (take, keys) = (Rc::clone(&self.take), Rc::clone(&self.keys));
        let (take_paste, pasted) = (Rc::clone(&self.take), Rc::clone(&self.pasted));
        let element = self.hooked.element(&tokens, false, cx);
        div()
            .child(hooked(
                &self.hooked,
                element,
                move |key, _, _| {
                    keys.borrow_mut().push(key);
                    if take.get() {
                        KeyAnswer::Taken
                    } else {
                        KeyAnswer::Pass
                    }
                },
                move |items, _, _| {
                    pasted.borrow_mut().extend(items);
                    if take_paste.get() {
                        KeyAnswer::Taken
                    } else {
                        KeyAnswer::Pass
                    }
                },
            ))
            .child(self.plain.element(&tokens, false, cx))
    }
}

fn open(cx: &mut TestAppContext) -> (Entity<Probe>, &mut gpui::VisualTestContext) {
    cx.update(gpui_base::init);
    let (probe, cx) = cx.add_window_view(Probe::new);
    cx.run_until_parked();
    (probe, cx)
}

fn paste_keys(cx: &gpui::VisualTestContext) -> &'static str {
    cx.read(|cx| {
        if cx.platform_info().is_mac() {
            "cmd-v"
        } else {
            "ctrl-v"
        }
    })
}

/// Type and press `keys` into `field`, giving it the keys first.
fn drive(
    probe: &Entity<Probe>,
    field: fn(&Probe) -> &TextField,
    cx: &mut gpui::VisualTestContext,
) -> String {
    probe.update_in(cx, |probe, window, cx| field(probe).focus(window, cx));
    cx.simulate_input("ab");
    cx.simulate_keystrokes("left up down tab");
    cx.simulate_input("c");
    cx.simulate_keystrokes("escape shift-enter");
    cx.simulate_input("d");
    cx.run_until_parked();
    cx.update(|_, cx| probe.read_with(cx, |probe, cx| field(probe).text(cx)))
}

#[gpui::test]
fn passed_keys_edit_and_send_exactly_as_an_unhooked_composer(cx: &mut TestAppContext) {
    let (probe, cx) = open(cx);
    let hooked_text = drive(&probe, |probe| &probe.hooked, cx);
    let plain_text = drive(&probe, |probe| &probe.plain, cx);
    assert_eq!(
        hooked_text, plain_text,
        "Tab indents and the arrows move as before"
    );
    assert!(
        hooked_text.contains('\n'),
        "Shift+Enter still starts a line"
    );
    cx.update(|_, cx| {
        probe.read_with(cx, |probe, _| {
            assert_eq!(
                *probe.keys.borrow(),
                [
                    ComposerKey::Up,
                    ComposerKey::Down,
                    ComposerKey::Tab,
                    ComposerKey::Escape
                ],
                "offered, not Shift+Enter, and nothing from the unhooked field"
            );
        });
    });

    // Enter passed sends exactly what the unhooked composer sends from the
    // same text and caret (whatever the engine does with the key first).
    let fields: [fn(&Probe) -> &TextField; 2] = [|probe| &probe.hooked, |probe| &probe.plain];
    for field in fields {
        probe.update_in(cx, |probe, window, cx| field(probe).focus(window, cx));
        cx.simulate_keystrokes("enter");
        cx.run_until_parked();
    }
    cx.update(|_, cx| {
        probe.read_with(cx, |probe, _| {
            assert_eq!(probe.submitted.len(), 2, "Enter passed sends");
            assert_eq!(probe.submitted.first(), probe.submitted.get(1));
            assert_eq!(probe.keys.borrow().last(), Some(&ComposerKey::Enter));
        });
    });
}

#[gpui::test]
fn taken_keys_never_reach_the_text(cx: &mut TestAppContext) {
    let (probe, cx) = open(cx);
    cx.simulate_input("ab");
    cx.simulate_keystrokes("left");
    probe.update(cx, |probe, _| probe.take.set(true));
    cx.simulate_keystrokes("up down tab enter escape");
    cx.simulate_input("X");
    cx.run_until_parked();
    cx.update(|_, cx| {
        probe.read_with(cx, |probe, cx| {
            assert_eq!(probe.hooked.text(cx), "aXb", "the caret did not move");
            assert!(probe.submitted.is_empty(), "a taken Enter does not send");
            assert_eq!(
                *probe.keys.borrow(),
                [
                    ComposerKey::Up,
                    ComposerKey::Down,
                    ComposerKey::Tab,
                    ComposerKey::Enter,
                    ComposerKey::Escape
                ]
            );
        });
    });
}

#[gpui::test]
fn an_input_method_composing_is_never_asked(cx: &mut TestAppContext) {
    let (probe, cx) = open(cx);
    probe.update(cx, |probe, _| probe.take.set(true));
    probe.update_in(cx, |probe, window, cx| {
        if let Kind::Lines(state, _) = &probe.hooked.kind {
            state.update(cx, |state, cx| {
                state.replace_and_mark_text_in_range(None, "하", None, window, cx);
            });
        }
    });
    cx.simulate_keystrokes("up down escape");
    cx.run_until_parked();
    cx.update(|_, cx| {
        probe.read_with(cx, |probe, _| {
            assert!(
                probe.keys.borrow().is_empty(),
                "the keys were the input method's"
            );
        });
    });
}

#[gpui::test]
fn the_caret_counts_bytes_and_a_replacement_keeps_characters_whole(cx: &mut TestAppContext) {
    let (probe, cx) = open(cx);
    cx.simulate_input("/리뷰 more");
    cx.run_until_parked();
    probe.update_in(cx, |probe, window, cx| {
        let field = &probe.hooked;
        assert_eq!(field.caret(cx), "/리뷰 more".len());
        let end = "/리뷰 more".len();
        assert_eq!(field.selection(cx), end..end, "nothing selected");
        // `/리뷰` is 1 + 3 + 3 bytes.
        field.replace_range(0..7, "/review", window, cx);
        assert_eq!(field.text(cx), "/review more");
        assert_eq!(field.caret(cx), "/review".len(), "after what was written");
        // 2..4 of "/한" begins inside 한 (bytes 1..4): from its start.
        field.replace_all("/한", window, cx);
        assert_eq!(field.caret(cx), "/한".len(), "the caret at the end");
        field.replace_range(2..4, "x", window, cx);
        assert_eq!(field.text(cx), "/x", "the range begins at 한's start");
    });
}

fn image() -> ClipboardItem {
    ClipboardItem::new_image(&Image {
        format: ImageFormat::Png,
        bytes: vec![0x89, b'P', b'N', b'G'],
        id: 7,
    })
}

#[gpui::test]
fn a_taken_image_paste_inserts_no_text(cx: &mut TestAppContext) {
    let (probe, cx) = open(cx);
    probe.update(cx, |probe, _| probe.take.set(true));
    cx.write_to_clipboard(image());
    let keys = paste_keys(cx);
    cx.simulate_keystrokes(keys);
    cx.run_until_parked();
    cx.update(|_, cx| {
        probe.read_with(cx, |probe, cx| {
            assert_eq!(
                *probe.pasted.borrow(),
                [Pasted::Image {
                    mime: "image/png",
                    bytes: vec![0x89, b'P', b'N', b'G']
                }]
            );
            assert_eq!(probe.hooked.text(cx), "");
        });
    });
}

#[gpui::test]
fn text_on_the_clipboard_is_pasted_without_asking(cx: &mut TestAppContext) {
    let (probe, cx) = open(cx);
    probe.update(cx, |probe, _| probe.take.set(true));
    cx.write_to_clipboard(ClipboardItem::new_string("pasted".to_owned()));
    let keys = paste_keys(cx);
    cx.simulate_keystrokes(keys);
    cx.run_until_parked();
    cx.update(|_, cx| {
        probe.read_with(cx, |probe, cx| {
            assert!(probe.pasted.borrow().is_empty(), "no images, not asked");
            assert_eq!(probe.hooked.text(cx), "pasted");
        });
    });
}

#[gpui::test]
fn a_passed_image_paste_reaches_the_text_engine(cx: &mut TestAppContext) {
    let (probe, cx) = open(cx);
    cx.write_to_clipboard(image());
    let keys = paste_keys(cx);
    cx.simulate_keystrokes(keys);
    cx.run_until_parked();
    cx.update(|_, cx| {
        probe.read_with(cx, |probe, cx| {
            assert_eq!(probe.pasted.borrow().len(), 1, "offered");
            assert_eq!(
                probe.hooked.text(cx),
                "",
                "an image holds no text to insert"
            );
        });
    });
}

#[gpui::test]
fn every_edit_typed_or_written_is_reported_and_a_caret_move_is_not(cx: &mut TestAppContext) {
    let (probe, cx) = open(cx);
    cx.simulate_input("ab");
    cx.run_until_parked();
    let typed = cx.update(|_, cx| probe.read_with(cx, |probe, _| probe.edits));
    assert!(typed >= 1, "typing reports");
    cx.simulate_keystrokes("left");
    cx.run_until_parked();
    let moved = cx.update(|_, cx| probe.read_with(cx, |probe, _| probe.edits));
    assert_eq!(moved, typed, "a caret move is no edit");
    probe.update_in(cx, |probe, window, cx| {
        probe.hooked.replace_all("c", window, cx);
    });
    cx.run_until_parked();
    let written = cx.update(|_, cx| probe.read_with(cx, |probe, _| probe.edits));
    assert!(written > moved, "a written edit reports too");
}

#[test]
fn utf16_ranges_floor_to_character_boundaries() {
    assert_eq!(utf16_range("/한글", 0..4), 0..2);
    assert_eq!(utf16_range("/한글", 2..7), 1..3, "2 is inside 한");
    assert_eq!(
        utf16_range("a😀b", 1..5),
        1..3,
        "an astral character is two units"
    );
    assert_eq!(utf16_range("ab", 5..9), 2..2, "past the end");
    // An end before its start (built field by field: a reversed literal
    // range is what clippy rightly refuses elsewhere).
    let reversed = Range { start: 2, end: 1 };
    assert_eq!(utf16_range("ab", reversed), 2..2);
}
