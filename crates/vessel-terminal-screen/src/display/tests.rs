//! Recorded oracle JSON is decoded only at this test edge; product metadata stays typed.
use super::*;
use crate::{FrameRows, LineIndex, Row, Screen, ScrollbackLines, Size, ViewportRequest};
use base64::Engine as _;
use serde::Deserialize;

#[derive(Deserialize)]
struct Corpus {
    programs: Vec<Program>,
}
#[derive(Deserialize)]
struct Dimensions {
    rows: u16,
    cols: u16,
}
#[derive(Deserialize)]
struct Program {
    name: ProgramName,
    rows: u16,
    cols: u16,
    events: Vec<Event>,
    snapshots: Vec<Expected>,
}
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum ProgramName {
    CursorOverrides,
    PhysicalWraps,
    SoftResetRetainsCells,
    AlternatePendingWrap,
    RepeatedAlternate,
    SoftResetNegotiation,
    #[serde(rename = "alternate-47")]
    Alternate47,
    #[serde(rename = "alternate-1047")]
    Alternate1047,
}
#[derive(Deserialize)]
#[serde(untagged)]
enum Event {
    Output { output: String },
    Resize { resize: Dimensions },
    Checkpoint { checkpoint: String },
}
#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum Shape {
    Block,
    Underline,
    Bar,
}
#[derive(Deserialize)]
struct Expected {
    checkpoint: String,
    rows: u16,
    cols: u16,
    history: u32,
    shape: Option<Shape>,
    blink: Option<bool>,
    wraps_next: Vec<bool>,
    cursor: ExpectedCursor,
    lines: Vec<String>,
    foreground: Vec<Vec<Option<u8>>>,
    input_modes: ExpectedModes,
}
#[derive(Deserialize)]
struct ExpectedModes {
    kitty: u32,
    win32: bool,
    application_cursor: bool,
    bracketed_paste: bool,
}
impl ExpectedModes {
    fn modes(&self) -> crate::InputModes {
        fn state(enabled: bool) -> crate::ModeState {
            if enabled {
                crate::ModeState::Enabled
            } else {
                crate::ModeState::Disabled
            }
        }
        crate::InputModes {
            kitty: self.kitty.into(),
            win32: state(self.win32),
            application_cursor: state(self.application_cursor),
            bracketed_paste: state(self.bracketed_paste),
        }
    }
}
#[derive(Deserialize)]
struct ExpectedCursor {
    row: u16,
    column: u16,
    hidden: bool,
}
impl Expected {
    fn appearance(&self) -> CursorAppearance {
        CursorAppearance::new(
            self.shape.as_ref().map(|shape| match shape {
                Shape::Block => CursorShape::Block,
                Shape::Underline => CursorShape::Underline,
                Shape::Bar => CursorShape::Bar,
            }),
            self.blink.map(|on| {
                if on {
                    CursorBlink::On
                } else {
                    CursorBlink::Off
                }
            }),
        )
    }
}
fn check_frame(frame: &crate::FrameView<'_>, expected: &Expected) {
    assert_eq!(
        frame.input_modes(),
        expected.input_modes.modes(),
        "{}",
        expected.checkpoint
    );
    assert_eq!(frame.columns(), expected.cols, "{}", expected.checkpoint);
    assert_eq!(frame.rows().get(), expected.rows, "{}", expected.checkpoint);
    assert_eq!(
        frame.cursor_appearance(),
        expected.appearance(),
        "{}",
        expected.checkpoint
    );
    let cursor_line = expected
        .history
        .checked_add(u32::from(expected.cursor.row))
        .unwrap();
    let cursor = cursor_line
        .checked_sub(frame.first().0)
        .filter(|row| *row < u32::from(frame.rows().get()))
        .map_or(crate::FrameCursor::OutsideViewport, |row| {
            crate::FrameCursor::Present(crate::Cursor {
                row: Row(u16::try_from(row).unwrap()),
                column: crate::Column(expected.cursor.column),
                visibility: if expected.cursor.hidden {
                    crate::Visibility::Hidden
                } else {
                    crate::Visibility::Shown
                },
            })
        });
    assert_eq!(frame.cursor(), cursor, "{}", expected.checkpoint);
    for row in 0..frame.rows().get() {
        let line = frame.first().0.checked_add(u32::from(row)).unwrap();
        let expected_soft = expected
            .wraps_next
            .get(usize::try_from(line).unwrap())
            .copied()
            .unwrap();
        assert_eq!(
            frame.row_break(Row(row)),
            Some(if expected_soft {
                RowBreak::Soft
            } else {
                RowBreak::Hard
            }),
            "{} row {line}",
            expected.checkpoint
        );
        check_row(frame, Row(row), expected, usize::try_from(line).unwrap());
    }
    assert!(frame.row_break(Row(expected.rows)).is_none());
}
fn check_row(frame: &crate::FrameView<'_>, row: Row, expected: &Expected, line: usize) {
    let mut text = String::new();
    let colors = &expected.foreground[line];
    for column in 0..frame.columns() {
        let cell = frame.cell(row, crate::Column(column)).unwrap();
        if cell.width() != crate::Width::Continuation {
            text.push_str(if cell.text().is_empty() {
                " "
            } else {
                cell.text()
            });
        }
        let color = colors.get(usize::from(column)).copied().unwrap();
        assert_eq!(
            cell.attributes().foreground(),
            color.map_or(crate::Color::Default, crate::Color::Indexed)
        );
    }
    assert_eq!(
        text.trim_end(),
        expected.lines[line],
        "{} row{line}",
        expected.checkpoint
    );
}
fn check(program: &Program) {
    let mut engine = Screen::new(
        Size::new(program.cols, program.rows).unwrap(),
        ScrollbackLines::default(),
    )
    .unwrap();
    let mut snapshots = program.snapshots.iter();
    let mut retained = Vec::new();
    for event in &program.events {
        match event {
            Event::Output { output } => {
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(output)
                    .unwrap();
                let feed = engine.feed(&bytes);
                assert_eq!(feed.consumed.get(), bytes.len());
                assert!(feed.replies.unwrap().as_bytes().is_empty());
            }
            Event::Resize { resize } => {
                engine
                    .resize(Size::new(resize.cols, resize.rows).unwrap())
                    .unwrap();
            }
            Event::Checkpoint { checkpoint } => {
                let expected = snapshots.next().unwrap();
                assert_eq!(checkpoint, &expected.checkpoint, "{:?}", program.name);
                let rows = FrameRows::new(expected.rows).unwrap();
                let snapshot = engine.snapshot();
                let live = engine.frame(ViewportRequest::FollowLive, rows).unwrap();
                assert_eq!(live.first(), LineIndex(expected.history));
                check_frame(&live, expected);
                check_frame(
                    &snapshot.frame(ViewportRequest::FollowLive, rows).unwrap(),
                    expected,
                );
                retained.push((snapshot, expected));
                if expected.history > 0 {
                    let pin = engine.create_marker(LineIndex(0)).unwrap();
                    check_frame(
                        &engine.frame(ViewportRequest::Pinned(&pin), rows).unwrap(),
                        expected,
                    );
                    let _released = engine.release_marker(&pin);
                }
            }
        }
    }
    assert!(snapshots.next().is_none());
    for (snapshot, expected) in retained {
        check_frame(
            &snapshot
                .frame(
                    ViewportRequest::FollowLive,
                    FrameRows::new(expected.rows).unwrap(),
                )
                .unwrap(),
            expected,
        );
    }
}
#[test]
fn cursor_overrides_match_pinned_headless_controls() {
    check(&program(ProgramName::CursorOverrides));
}
#[test]
fn physical_row_wraps_match_pinned_headless_history_and_resize() {
    check(&program(ProgramName::PhysicalWraps));
}
#[test]
fn soft_reset_retains_cells_and_resets_saved_presentation_like_paseo() {
    check(&program(ProgramName::SoftResetRetainsCells));
}
#[test]
fn alternate_entry_inherits_the_pending_wrap_column_and_restores_primary() {
    check(&program(ProgramName::AlternatePendingWrap));
}
#[test]
fn repeated_alternate_entry_retains_cells_and_primary_saved_cursor_and_ink() {
    check(&program(ProgramName::RepeatedAlternate));
}
#[test]
fn soft_reset_clears_vt_negotiation_but_preserves_server_keyboard_tracker_stack() {
    check(&program(ProgramName::SoftResetNegotiation));
}
#[test]
fn alternate_47_matches_entry_reentry_exit_and_cursor_carry() {
    check(&program(ProgramName::Alternate47));
}
#[test]
fn alternate_1047_matches_entry_reentry_exit_and_cursor_carry() {
    check(&program(ProgramName::Alternate1047));
}
#[test]
fn redundant_alternate_activation_does_not_reset_retained_marker_identity() {
    let mut engine = Screen::new(Size::new(8, 3).unwrap(), ScrollbackLines::default()).unwrap();
    assert!(
        engine
            .feed(b"\x1b[?1049hA")
            .replies
            .unwrap()
            .as_bytes()
            .is_empty()
    );
    let marker = engine.create_marker(LineIndex(0)).unwrap();
    let before = engine.resolve_marker(&marker).unwrap();
    let retained = engine.snapshot();
    assert!(
        engine
            .feed(b"\x1b[?1049hB")
            .replies
            .unwrap()
            .as_bytes()
            .is_empty()
    );
    assert_eq!(engine.resolve_marker(&marker).unwrap(), before);
    assert_eq!(retained.resolve_marker(&marker).unwrap(), before);
    assert!(
        engine
            .feed(b"\x1b[?1049l\x1b[?1049h")
            .replies
            .unwrap()
            .as_bytes()
            .is_empty()
    );
    assert_eq!(
        engine.resolve_marker(&marker).unwrap().state,
        crate::AnchorState::Reset
    );
    assert_eq!(retained.resolve_marker(&marker).unwrap(), before);
}
fn program(name: ProgramName) -> Program {
    let corpus: Corpus = serde_json::from_str(include_str!("oracle.json")).unwrap();
    corpus
        .programs
        .into_iter()
        .find(|program| program.name == name)
        .unwrap()
}
