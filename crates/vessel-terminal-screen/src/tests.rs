//! Actual program output and oracle-derived golden cells; these are not mirrored implementation tests.
use crate::*;
use base64::Engine as _;
use sha2::{Digest as _, Sha256};
use std::fmt::Write as _;

#[derive(serde::Deserialize)]
struct Corpus {
    programs: Vec<Program>,
}
#[derive(serde::Deserialize)]
struct Program {
    name: String,
    rows: u16,
    cols: u16,
    events: Vec<Event>,
    snapshots: Vec<Expected>,
}
#[derive(serde::Deserialize)]
#[serde(untagged)]
enum Event {
    Output { output: String },
    Resize { resize: Dimensions },
    Checkpoint { checkpoint: String },
}
#[derive(serde::Deserialize)]
struct Dimensions {
    rows: u16,
    cols: u16,
}
#[derive(serde::Deserialize)]
struct Expected {
    checkpoint: String,
    rows: u16,
    cols: u16,
    cursor: (u16, u16),
    hidden: bool,
    alternate: bool,
    history: usize,
    lines: Vec<String>,
    cells_sha256: String,
}
#[derive(serde::Serialize)]
struct GoldenCell {
    text: String,
    width: u8,
    fg: GoldenColor,
    bg: GoldenColor,
    flags: u16,
    underline: u8,
}
#[derive(serde::Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
enum GoldenColor {
    Default,
    Indexed(u8),
    Rgb(u32),
}
fn color(value: Color) -> GoldenColor {
    match value {
        Color::Default => GoldenColor::Default,
        Color::Indexed(index) => GoldenColor::Indexed(index),
        Color::Rgb(r, g, b) => {
            GoldenColor::Rgb(u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b))
        }
    }
}
fn golden_cells(screen: &Screen) -> String {
    let mut cells = Vec::new();
    for row in 0..screen.size().rows() {
        for col in 0..screen.size().cols() {
            let cell = screen.cell(Row(row), Column(col)).unwrap();
            let attr = cell.attributes();
            let flags = u16::from(attr.has(Style::Bold))
                | u16::from(attr.has(Style::Dim)) << 1
                | u16::from(attr.has(Style::Italic)) << 2
                | u16::from(attr.has(Style::Inverse)) << 3;
            cells.push(GoldenCell {
                text: cell.text().to_owned(),
                width: match cell.width() {
                    Width::One => 1,
                    Width::Two => 2,
                    Width::Continuation => 0,
                },
                fg: color(attr.foreground()),
                bg: color(attr.background()),
                flags,
                underline: u8::from(attr.has(Style::Underline)),
            });
        }
    }
    let digest = Sha256::digest(serde_json::to_vec(&cells).unwrap());
    let mut hash = String::new();
    for byte in &digest {
        write!(&mut hash, "{byte:02x}").unwrap();
    }
    hash
}
fn feed(screen: &mut Screen, mut bytes: &[u8]) {
    while !bytes.is_empty() {
        let consumed = screen.feed(bytes).consumed.get();
        assert!(consumed > 0);
        bytes = &bytes[consumed..];
    }
}
#[test]
fn real_ls_git_vim_all_eight_checkpoints_match_paseo_server() {
    check_corpus(include_str!("tests/real-corpus.json"), 8);
}

#[test]
fn actual_cat_primary_resize_matches_paseo_server() {
    check_corpus(include_str!("tests/resize-corpus.json"), 7);
}

#[test]
fn saved_cursor_held_runs_wide_padding_and_colors_match_paseo_server() {
    check_corpus(include_str!("tests/resize-contracts.json"), 13);
}

#[test]
fn bold_dim_independence_and_intensity_reset_match_paseo_server() {
    check_corpus(include_str!("tests/attribute-contracts.json"), 3);
}

fn check_corpus(input: &str, count: usize) {
    let corpus: Corpus = serde_json::from_str(input).unwrap();
    let mut checked = 0_usize;
    for program in corpus.programs {
        let mut screen = Screen::new(
            Size::new(program.cols, program.rows).unwrap(),
            ScrollbackLines::default(),
        )
        .unwrap();
        for event in program.events {
            match event {
                Event::Output { output } => feed(
                    &mut screen,
                    &base64::engine::general_purpose::STANDARD
                        .decode(output)
                        .unwrap(),
                ),
                Event::Resize { resize } => screen
                    .resize(Size::new(resize.cols, resize.rows).unwrap())
                    .unwrap(),
                Event::Checkpoint { checkpoint } => {
                    let expected = program
                        .snapshots
                        .iter()
                        .find(|snap| snap.checkpoint == checkpoint)
                        .unwrap();
                    assert_eq!(
                        (screen.size().rows(), screen.size().cols()),
                        (expected.rows, expected.cols)
                    );
                    let cursor = screen.cursor();
                    assert_eq!(
                        (cursor.row.0, cursor.column.0),
                        expected.cursor,
                        "{} {checkpoint}",
                        program.name
                    );
                    assert_eq!(cursor.visibility == Visibility::Hidden, expected.hidden);
                    assert_eq!(
                        screen.buffer_kind() == BufferKind::Alternate,
                        expected.alternate
                    );
                    assert_eq!(
                        screen.history_len().get(),
                        expected.history,
                        "{} {checkpoint}, rows: {:?}",
                        program.name,
                        screen.capture(CaptureRange::default())
                    );
                    assert_eq!(
                        screen.capture(CaptureRange::default()).unwrap().lines,
                        expected.lines,
                        "{} {checkpoint} capture",
                        program.name
                    );
                    assert_eq!(
                        golden_cells(&screen),
                        expected.cells_sha256,
                        "{} {checkpoint} all raw cells",
                        program.name
                    );
                    checked = checked.saturating_add(1);
                }
            }
        }
    }
    assert_eq!(checked, count);
}
#[test]
fn capture_ranges_trim_like_javascript_and_final_snapshot_survives_owner() {
    let snapshot = {
        let mut screen =
            Screen::new(Size::new(20, 2).unwrap(), ScrollbackLines::new(4).unwrap()).unwrap();
        feed(&mut screen, "one\r\ntwo\r\n한글\u{feff}".as_bytes());
        screen.snapshot()
    };
    let range = CaptureRange {
        start: Some((-1).into()),
        end: None,
    };
    assert_eq!(snapshot.capture(range).unwrap().lines, vec!["한글"]);
    assert_eq!(snapshot.capture(range).unwrap().total.get(), 3);
    assert!(
        snapshot
            .capture(CaptureRange {
                start: Some(2.into()),
                end: Some(0.into())
            })
            .unwrap()
            .lines
            .is_empty()
    );
}
#[test]
fn shrinking_history_preserves_newest_and_increase_never_revives_eviction() {
    let mut screen =
        Screen::new(Size::new(12, 2).unwrap(), ScrollbackLines::new(8).unwrap()).unwrap();
    feed(&mut screen, b"0\r\n1\r\n2\r\n3\r\n4\r\n5");
    screen
        .set_scrollback(ScrollbackLines::new(1).unwrap())
        .unwrap();
    assert_eq!(
        screen.capture(CaptureRange::default()).unwrap().lines,
        vec!["3", "4", "5"]
    );
    screen
        .set_scrollback(ScrollbackLines::new(8).unwrap())
        .unwrap();
    assert_eq!(screen.history_len().get(), 1);
    screen
        .set_scrollback(ScrollbackLines::new(0).unwrap())
        .unwrap();
    feed(&mut screen, b"\r\n6\r\n7");
    assert_eq!(screen.history_len().get(), 0);
}

#[test]
fn capture_budget_refuses_without_changing_retained_screen_or_shorter_reads() {
    let width = 1024_u16;
    let mut screen = Screen::new(Size::new(width, 2).unwrap(), ScrollbackLines::default()).unwrap();
    let line = format!("{}\r\n", "x".repeat(usize::from(width)));
    for _ in 0..=MAX_CAPTURE_BYTES.checked_div(usize::from(width)).unwrap() {
        feed(&mut screen, line.as_bytes());
    }
    let cursor = screen.cursor();
    let history = screen.history_len();
    assert_eq!(
        screen.capture(CaptureRange::default()),
        Err(ScreenError::CaptureTooLarge)
    );
    assert_eq!(screen.cursor(), cursor);
    assert_eq!(screen.history_len(), history);
    assert_eq!(
        screen
            .capture(CaptureRange {
                start: Some((-1).into()),
                end: None
            })
            .unwrap()
            .lines,
        vec![""]
    );
}

#[test]
fn geometry_reserves_both_screens_and_split_utf8_remains_original() {
    assert_eq!(Size::new(0, 2), Err(ScreenError::Geometry));
    assert_eq!(
        Size::new(MIN_COLUMNS.checked_sub(1).unwrap(), 2),
        Err(ScreenError::Geometry)
    );
    assert_eq!(Size::new(2, 0), Err(ScreenError::Geometry));
    assert_eq!(Size::new(2000, 1001), Err(ScreenError::Geometry));
    assert!(Size::new(2000, 1000).is_ok());
    assert_eq!(
        ScrollbackLines::new(ScrollbackLines::MAX.checked_add(1).unwrap()),
        Err(ScreenError::Scrollback)
    );
    let mut screen = Screen::new(Size::new(20, 2).unwrap(), ScrollbackLines::default()).unwrap();
    let text = "한글 cafe\u{301} ❤️";
    for byte in text.as_bytes() {
        let feed = screen.feed(&[*byte]);
        assert_eq!(feed.consumed.get(), 1);
        assert!(feed.replies.unwrap().as_bytes().is_empty());
    }
    assert_eq!(
        screen.capture(CaptureRange::default()).unwrap().lines,
        vec![text, ""]
    );
}

#[test]
fn retained_styled_cells_and_final_metadata_outlive_eviction_and_owner() {
    let snapshot = {
        let mut screen =
            Screen::new(Size::new(4, 2).unwrap(), ScrollbackLines::new(3).unwrap()).unwrap();
        feed(&mut screen, "\x1b[31m한!\x1b[0m\r\ntwo\r\nend".as_bytes());
        let lead = screen.buffer_cell(LineIndex(0), Column(0)).unwrap();
        let continuation = screen.buffer_cell(LineIndex(0), Column(1)).unwrap();
        assert_eq!(lead.text(), "한");
        assert_eq!(lead.attributes().foreground(), Color::Indexed(1));
        assert_eq!(continuation.width(), Width::Continuation);
        assert_eq!(continuation.attributes(), lead.attributes());
        let snapshot = screen.snapshot();
        screen
            .set_scrollback(ScrollbackLines::new(0).unwrap())
            .unwrap();
        feed(&mut screen, b"\r\nnew\r\nlast");
        assert_eq!(screen.history_len().get(), 0);
        snapshot
    };
    assert_eq!(snapshot.history_len().get(), 1);
    assert_eq!(snapshot.buffer_kind(), BufferKind::Primary);
    assert_eq!(
        snapshot.cursor(),
        Cursor {
            row: Row(1),
            column: Column(3),
            visibility: Visibility::Shown
        }
    );
    assert_eq!(
        snapshot
            .buffer_cell(LineIndex(0), Column(0))
            .unwrap()
            .text(),
        "한"
    );
    assert!(
        snapshot
            .buffer_cell(LineIndex(u32::MAX), Column(0))
            .is_none()
    );
    assert!(snapshot.cell(Row(0), Column(u16::MAX)).is_none());
    assert_eq!(
        snapshot.capture(CaptureRange::default()).unwrap().lines,
        vec!["한!", "two", "end"]
    );
}
