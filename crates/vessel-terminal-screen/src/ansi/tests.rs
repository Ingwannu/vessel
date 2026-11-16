//! Pure display admission plus exported cases for the independent pinned terminal oracle.
use super::*;
use crate::{FrameRows, Screen, ScrollbackLines, ViewportRequest};
use base64::Engine as _;
use std::io::Write as _;

fn ink() -> Attributes {
    Attributes::new(Color::Default, Color::Default, [])
}
fn defaults() -> CursorDefaults {
    CursorDefaults::new(CursorShape::Bar, CursorBlink::On)
}
#[test]
fn readonly_cells_refuse_control_injection_and_invalid_spans() {
    for text in [
        "\x1b[6n",
        "\r",
        "\n",
        "\x00",
        "\u{009b}",
        "abcdefghijklmnopqrstuvw",
    ] {
        assert_eq!(
            Cell::from_parts(text, Width::One, ink()),
            Err(ScreenError::CellContents)
        );
    }
    assert_eq!(
        Cell::from_parts("AB", Width::One, ink()),
        Err(ScreenError::CellContents)
    );
    assert_eq!(
        Cell::from_parts("AB", Width::Two, ink()),
        Err(ScreenError::CellContents)
    );
    assert_eq!(
        Cell::from_parts("", Width::Two, ink()),
        Err(ScreenError::CellContents)
    );
    assert_eq!(
        Cell::from_parts("a", Width::Continuation, ink()),
        Err(ScreenError::CellContents)
    );
    assert!(Cell::from_parts("한", Width::Two, ink()).is_ok());
    assert!(Cell::from_parts("e\u{301}", Width::One, ink()).is_ok());
}
#[test]
fn malformed_wide_pair_returns_no_display_frame() {
    let wide = Cell::from_parts("한", Width::Two, ink()).unwrap();
    let wrong = Cell::from_parts("x", Width::One, ink()).unwrap();
    let result = paint_cells(
        Size::new(4, 1).unwrap(),
        FrameCursor::OutsideViewport,
        CursorAppearance::default(),
        defaults(),
        InputModes::default(),
        |_, column| Ok(if column.0 == 0 { wide } else { wrong }),
    );
    assert_eq!(result, Err(ScreenError::CellContents));
    let orphan = Cell::from_parts("", Width::Continuation, ink()).unwrap();
    assert_eq!(
        paint_cells(
            Size::new(4, 1).unwrap(),
            FrameCursor::OutsideViewport,
            CursorAppearance::default(),
            defaults(),
            InputModes::default(),
            |_, _| Ok(orphan)
        ),
        Err(ScreenError::CellContents)
    );
}
#[test]
fn invalid_cursor_refuses_before_any_cell_is_read() {
    let mut calls = 0;
    let result = paint_cells(
        Size::new(4, 3).unwrap(),
        FrameCursor::Present(Cursor {
            row: Row(3),
            column: Column(0),
            visibility: Visibility::Shown,
        }),
        CursorAppearance::default(),
        defaults(),
        InputModes::default(),
        |_, _| {
            calls += 1;
            Cell::from_parts("", Width::One, ink())
        },
    );
    assert_eq!(result, Err(ScreenError::Geometry));
    assert_eq!(calls, 0);
}
#[test]
fn encoded_byte_budget_stops_work_before_walking_a_maximum_grid() {
    let glyph = Cell::from_parts("a", Width::One, ink()).unwrap();
    let mut calls = 0_u32;
    let result = paint_cells(
        Size::new(1000, 1000).unwrap(),
        FrameCursor::OutsideViewport,
        CursorAppearance::default(),
        defaults(),
        InputModes::default(),
        |_, _| {
            calls += 1;
            Ok(glyph)
        },
    );
    assert_eq!(result, Err(ScreenError::DisplayTooLarge));
    assert!(calls > 0 && calls < 1_000_000);
}

#[derive(serde::Serialize)]
struct Probe {
    name: &'static str,
    rows: u16,
    cols: u16,
    input_base64: String,
    pin: Option<u32>,
    first: u32,
    ansi_base64: String,
    modes: Modes,
}
#[derive(serde::Serialize)]
struct Modes {
    kitty: u32,
    win32: bool,
    application_cursor: bool,
    bracketed_paste: bool,
}
fn mode(state: ModeState) -> bool {
    match state {
        ModeState::Enabled => true,
        ModeState::Disabled => false,
    }
}
type DisplayScene = (&'static str, u16, u16, &'static str, Option<u32>);
const SCENES: &[DisplayScene] = &[
    ("plain", 12, 4, "hello\r\nworld", None),
    (
        "styled-wide",
        16,
        4,
        "\x1b[31;48;5;17;1;2;3;4;7me\u{301} 한글🙂\x1b[0m\r\nplain\x1b[>7u\x1b[?1;2004;9001h",
        None,
    ),
    ("history", 8, 3, "0\r\n1\r\n2\r\n3\r\n4", Some(1)),
    ("pending-wrap", 8, 3, "12345678", None),
    ("cursor-default", 8, 3, "default", None),
    ("cursor-1", 8, 3, "\x1b[1 qone", None),
    ("cursor-2", 8, 3, "\x1b[2 qtwo", None),
    ("cursor-3", 8, 3, "\x1b[3 qthree", None),
    ("cursor-4", 8, 3, "\x1b[4 qfour", None),
    ("cursor-5", 8, 3, "\x1b[5 qfive", None),
    ("cursor-6", 8, 3, "\x1b[6 qsix", None),
    ("cursor-0-reset", 8, 3, "\x1b[1 q\x1b[0 qzero", None),
    ("cursor-7-blink", 8, 3, "\x1b[7 qseven", None),
    ("cursor-8-steady", 8, 3, "\x1b[8 qeight", None),
    ("cursor-soft-reset", 8, 3, "\x1b[1 qkept\x1b[!pX", None),
    ("cursor-ris", 8, 3, "\x1b[1 qbefore\x1bcafter", None),
    (
        "mode-soft-reset",
        8,
        3,
        "\x1b[>3u\x1b[>7u\x1b[?1;2004;9001h\x1b[!pX",
        None,
    ),
    (
        "alternate-reentry",
        8,
        3,
        "main\x1b[2;3H\x1b[?1049hA\x1b[?1049hB",
        None,
    ),
    (
        "alternate-47-carry",
        8,
        3,
        "main\x1b[2;3H\x1b[?47hA\x1b[3;4H\x1b[?47lX",
        None,
    ),
    (
        "alternate-1047-carry",
        8,
        3,
        "main\x1b[2;3H\x1b[?1047hA\x1b[3;4H\x1b[?1047lX",
        None,
    ),
];
#[test]
fn physical_display_fixture_export() {
    let mut probes = Vec::new();
    for &(name, cols, rows, input, pin) in SCENES {
        let mut engine =
            Screen::new(Size::new(cols, rows).unwrap(), ScrollbackLines::default()).unwrap();
        assert!(
            engine
                .feed(input.as_bytes())
                .replies
                .unwrap()
                .as_bytes()
                .is_empty()
        );
        let anchor = pin.map(|line| engine.create_marker(crate::LineIndex(line)).unwrap());
        let viewport = anchor
            .as_ref()
            .map_or(ViewportRequest::FollowLive, ViewportRequest::Pinned);
        let frame = engine
            .frame(viewport, FrameRows::new(rows).unwrap())
            .unwrap();
        for row in 0..frame.rows().get() {
            for column in 0..frame.columns() {
                assert!(
                    frame.cell(Row(row), Column(column)).is_some(),
                    "{name}: missing selected cell {row},{column} at {:?}",
                    frame.first()
                );
            }
        }
        let encoded = paint_frame(&frame, defaults()).unwrap();
        assert!(encoded.as_bytes().len() <= MAX_ANSI_FRAME_BYTES);
        let modes = frame.input_modes();
        probes.push(Probe {
            name,
            rows,
            cols,
            input_base64: base64::engine::general_purpose::STANDARD.encode(input),
            pin,
            first: frame.first().0,
            ansi_base64: base64::engine::general_purpose::STANDARD.encode(encoded.as_bytes()),
            modes: Modes {
                kitty: modes.kitty.get(),
                win32: mode(modes.win32),
                application_cursor: mode(modes.application_cursor),
                bracketed_paste: mode(modes.bracketed_paste),
            },
        });
    }
    // This is a data-fixture export, not a claim of renderer/terminal parity.
    // The separate pinned headless oracle compares original input vs display bytes.
    writeln!(
        std::io::stdout().lock(),
        "TERMINAL_DISPLAY_ORACLE {}",
        serde_json::to_string(&probes).unwrap()
    )
    .unwrap();
}
