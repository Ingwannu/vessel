//! Live output, reflow and reset must never reinterpret another opening's marker.
use super::*;
use crate::{CaptureRange, ScrollbackLines, Size};
use base64::Engine as _;

fn screen() -> Screen {
    Screen::new(Size::new(8, 3).unwrap(), ScrollbackLines::new(3).unwrap()).unwrap()
}
fn feed(screen: &mut Screen, bytes: &[u8]) {
    assert!(screen.feed(bytes).replies.unwrap().as_bytes().is_empty());
}
fn row(screen: &Screen, anchor: &ViewportAnchor) -> LineIndex {
    match screen.resolve_marker(anchor).unwrap().state {
        AnchorState::Retained(row) => row,
        other => panic!("unexpected marker {other:?}"),
    }
}
#[test]
fn inspected_output_is_retained_until_genuinely_evicted_and_return_live_releases() {
    let mut engine = screen();
    feed(&mut engine, b"0\r\n1\r\n2\r\n3\r\n4");
    let anchor = engine.create_marker(LineIndex(1)).unwrap();
    let retained = engine.snapshot();
    feed(&mut engine, b"\r\n5\r\n6");
    assert_eq!(row(&engine, &anchor), LineIndex(0));
    assert_eq!(
        engine
            .capture(CaptureRange {
                start: Some(0.into()),
                end: Some(0.into())
            })
            .unwrap()
            .lines,
        vec!["1"]
    );
    assert_eq!(
        retained.resolve_marker(&anchor).unwrap().state,
        AnchorState::Retained(LineIndex(1))
    );
    feed(&mut engine, b"\r\n7\r\n8\r\n9");
    assert_eq!(
        engine.resolve_marker(&anchor).unwrap().state,
        AnchorState::Evicted {
            oldest: LineIndex(0)
        }
    );
    assert_eq!(engine.release_marker(&anchor), AnchorRelease::Released);
    assert_eq!(engine.release_marker(&anchor), AnchorRelease::AlreadyGone);
    assert_eq!(
        engine.resolve_marker(&anchor).unwrap().state,
        AnchorState::Released
    );
}
#[test]
fn reset_and_other_authority_never_revive_local_marker_numbers() {
    let mut engine = screen();
    let anchor = engine.create_marker(LineIndex(0)).unwrap();
    let mut other = screen();
    let other_anchor = other.create_marker(LineIndex(0)).unwrap();
    assert_ne!(anchor, other_anchor);
    assert_eq!(
        other.resolve_marker(&anchor).unwrap().state,
        AnchorState::Reset
    );
    assert_eq!(other.release_marker(&anchor), AnchorRelease::Reset);
    feed(&mut engine, b"\x1bc");
    let fresh = engine.create_marker(LineIndex(0)).unwrap();
    assert_ne!(anchor, fresh);
    assert_eq!(
        engine.resolve_marker(&anchor).unwrap().state,
        AnchorState::Reset
    );
    assert_eq!(engine.release_marker(&anchor), AnchorRelease::Reset);
    assert_eq!(row(&engine, &fresh), LineIndex(0));
}
#[test]
fn full_capacity_replace_is_atomic_and_stale_handles_cannot_release_it() {
    let mut engine = screen();
    let anchors: Vec<_> = (0..MAX_VIEWPORT_MARKERS)
        .map(|_| engine.create_marker(LineIndex(0)).unwrap())
        .collect();
    assert_eq!(
        engine.create_marker(LineIndex(0)),
        Err(ScreenError::MarkerCapacity)
    );
    assert_eq!(
        engine.replace_marker(&anchors[0], LineIndex(99)),
        Err(ScreenError::MarkerRow)
    );
    assert_eq!(row(&engine, &anchors[0]), LineIndex(0));
    let fresh = engine.replace_marker(&anchors[0], LineIndex(1)).unwrap();
    assert_eq!(
        engine.release_marker(&anchors[0]),
        AnchorRelease::AlreadyGone
    );
    assert_eq!(row(&engine, &fresh), LineIndex(1));
    assert_eq!(
        engine.create_marker(LineIndex(0)),
        Err(ScreenError::MarkerCapacity)
    );
}
#[test]
fn primary_anchor_survives_alternate_and_history_clear_resets_explicitly() {
    let mut engine = screen();
    feed(&mut engine, b"0\r\n1\r\n2\r\n3\r\n4");
    let anchor = engine.create_marker(LineIndex(1)).unwrap();
    feed(&mut engine, b"\x1b[?1049halt");
    assert_eq!(
        engine.resolve_marker(&anchor).unwrap().presence,
        BufferPresence::Inactive
    );
    assert_eq!(
        engine.replace_marker(&anchor, LineIndex(0)),
        Err(ScreenError::MarkerInactive)
    );
    let alt = engine.create_marker(LineIndex(0)).unwrap();
    feed(&mut engine, b"\x1b[?1049l");
    assert_eq!(row(&engine, &anchor), LineIndex(1));
    assert_eq!(
        engine.resolve_marker(&alt).unwrap().presence,
        BufferPresence::Inactive
    );
    feed(&mut engine, b"\x1b[?1049h");
    assert_eq!(
        engine.resolve_marker(&alt).unwrap().state,
        AnchorState::Reset
    );
    feed(&mut engine, b"\x1b[?1049l\x1b[3J");
    assert_eq!(
        engine.resolve_marker(&anchor).unwrap().state,
        AnchorState::Reset
    );
}

#[derive(serde::Deserialize)]
struct Corpus {
    programs: Vec<Program>,
}
#[derive(serde::Deserialize)]
struct Program {
    rows: u16,
    cols: u16,
    scrollback: u32,
    events: Vec<Event>,
    snapshots: Vec<Expected>,
}
#[derive(serde::Deserialize)]
#[serde(untagged)]
enum Event {
    Output { output: String },
    Pin { pin: Pin },
    Resize { resize: Dimensions },
    Capacity { capacity: u32 },
    Checkpoint { checkpoint: String },
}
#[derive(serde::Deserialize)]
struct Dimensions {
    rows: u16,
    cols: u16,
}
#[derive(serde::Deserialize)]
struct Pin {
    name: String,
    line: u32,
}
#[derive(serde::Deserialize)]
struct Expected {
    checkpoint: String,
    markers: Vec<ExpectedMarker>,
}
#[derive(serde::Deserialize)]
struct ExpectedMarker {
    name: String,
    disposed: bool,
    line: i64,
    text: Option<String>,
}
#[test]
fn eleven_physical_marker_checkpoints_match_actual_paseo_headless() {
    let corpus: Corpus = serde_json::from_str(include_str!("paseo-markers.json")).unwrap();
    let mut checked = 0;
    for program in corpus.programs {
        let mut engine = Screen::new(
            Size::new(program.cols, program.rows).unwrap(),
            ScrollbackLines::new(program.scrollback).unwrap(),
        )
        .unwrap();
        let mut markers = std::collections::BTreeMap::new();
        for event in program.events {
            match event {
                Event::Output { output } => feed(
                    &mut engine,
                    &base64::engine::general_purpose::STANDARD
                        .decode(output)
                        .unwrap(),
                ),
                Event::Pin { pin } => {
                    assert!(
                        markers
                            .insert(pin.name, engine.create_marker(LineIndex(pin.line)).unwrap())
                            .is_none()
                    );
                }
                Event::Resize { resize } => engine
                    .resize(Size::new(resize.cols, resize.rows).unwrap())
                    .unwrap(),
                Event::Capacity { capacity } => engine
                    .set_scrollback(ScrollbackLines::new(capacity).unwrap())
                    .unwrap(),
                Event::Checkpoint { checkpoint } => {
                    let expected = program
                        .snapshots
                        .iter()
                        .find(|s| s.checkpoint == checkpoint)
                        .unwrap();
                    for marker in &expected.markers {
                        let actual = engine
                            .resolve_marker(markers.get(&marker.name).unwrap())
                            .unwrap();
                        assert_eq!(actual.presence, BufferPresence::Active);
                        if marker.disposed {
                            assert_eq!(
                                actual.state,
                                AnchorState::Evicted {
                                    oldest: LineIndex(0)
                                },
                                "{checkpoint} {}",
                                marker.name
                            );
                            assert_eq!(marker.line, -1);
                        } else {
                            let line = LineIndex(u32::try_from(marker.line).unwrap());
                            assert_eq!(
                                actual.state,
                                AnchorState::Retained(line),
                                "{checkpoint} {}",
                                marker.name
                            );
                            let capture = engine
                                .capture(CaptureRange {
                                    start: Some(marker.line.into()),
                                    end: Some(marker.line.into()),
                                })
                                .unwrap();
                            assert_eq!(
                                capture.lines.first(),
                                marker.text.as_ref(),
                                "{checkpoint} {}",
                                marker.name
                            );
                        }
                    }
                    checked += 1;
                }
            }
        }
    }
    assert_eq!(checked, 11);
}
