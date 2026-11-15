//! Viewport selection and borrowed cells do not reconstruct or clone a decoder.
use super::*;
use crate::{AnchorRelease, ScrollbackLines};

fn screen() -> Screen {
    Screen::new(Size::new(8, 3).unwrap(), ScrollbackLines::new(3).unwrap()).unwrap()
}
fn feed(engine: &mut Screen, bytes: &[u8]) {
    assert!(engine.feed(bytes).replies.unwrap().as_bytes().is_empty());
}
fn rows() -> FrameRows {
    FrameRows::new(3).unwrap()
}
#[test]
fn trimmed_history_still_projects_every_promised_live_and_snapshot_cell() {
    let mut engine = screen();
    feed(&mut engine, b"first\r\n\r\nthird\r\nfourth\r\nlast");
    let anchor = engine.create_marker(LineIndex(0)).unwrap();
    let snapshot = engine.snapshot();
    let frame = engine
        .frame(ViewportRequest::Pinned(&anchor), rows())
        .unwrap();
    let retained = snapshot
        .frame(ViewportRequest::Pinned(&anchor), rows())
        .unwrap();
    assert_eq!(frame.cells().get(), 24);
    for projected in [&frame, &retained] {
        let mut visited = 0;
        for row in 0..projected.rows().get() {
            for column in 0..projected.columns() {
                let cell = projected.cell(Row(row), Column(column)).unwrap();
                visited += 1;
                if row == 1 || column >= 5 {
                    assert_eq!(cell, Cell::blank());
                }
            }
        }
        assert_eq!(visited, projected.cells().get());
        assert!(projected.cell(Row(3), Column(0)).is_none());
        assert!(projected.cell(Row(0), Column(8)).is_none());
    }
    assert!(engine.buffer_cell(LineIndex(0), Column(7)).is_none());
    assert!(engine.buffer_cell(LineIndex(1), Column(0)).is_none());
    assert_eq!(frame.cell(Row(0), Column(0)).unwrap().text(), "f");
    assert_eq!(frame.cell(Row(2), Column(0)).unwrap().text(), "t");
}

fn assert_complete(frame: &FrameView<'_>) {
    let mut cells = 0_u32;
    for row in 0..frame.rows().get() {
        for column in 0..frame.columns() {
            assert!(frame.cell(Row(row), Column(column)).is_some());
            cells = cells.checked_add(1).unwrap();
        }
    }
    assert_eq!(cells, frame.cells().get());
    assert!(frame.cell(Row(0), Column(frame.columns())).is_none());
    assert!(frame.cell(Row(frame.rows().get()), Column(0)).is_none());
}

#[test]
fn compacted_padding_survives_resize_and_preserves_wide_ink_and_snapshot_geometry() {
    let mut engine = screen();
    feed(
        &mut engine,
        "\x1b[31m한\x1b[0m\r\n\r\nthird\r\nfourth\r\nlast".as_bytes(),
    );
    let anchor = engine.create_marker(LineIndex(0)).unwrap();
    let retained = engine.snapshot();
    let snapshot = retained
        .frame(ViewportRequest::Pinned(&anchor), rows())
        .unwrap();
    assert_complete(&snapshot);
    let lead = snapshot.cell(Row(0), Column(0)).unwrap();
    let continuation = snapshot.cell(Row(0), Column(1)).unwrap();
    assert_eq!(lead.text(), "한");
    assert_eq!(lead.width(), crate::Width::Two);
    assert_eq!(lead.attributes().foreground(), crate::Color::Indexed(1));
    assert_eq!(continuation.width(), crate::Width::Continuation);
    assert_eq!(continuation.attributes(), lead.attributes());
    assert_eq!(snapshot.cell(Row(0), Column(7)), Some(Cell::blank()));
    assert!(
        crate::projection::frame_cell(engine.parser.screen(), LineIndex(u32::MAX), Column(0))
            .is_none()
    );
    for columns in [16, 4] {
        engine.resize(Size::new(columns, 3).unwrap()).unwrap();
        let pinned = engine
            .frame(ViewportRequest::Pinned(&anchor), rows())
            .unwrap();
        assert_eq!(pinned.columns(), columns);
        assert_complete(&pinned);
        assert_complete(&engine.frame(ViewportRequest::FollowLive, rows()).unwrap());
        assert_eq!(snapshot.columns(), 8);
        assert_complete(&snapshot);
        assert_eq!(snapshot.cell(Row(0), Column(0)), Some(lead));
    }
    feed(&mut engine, b"\x1b[?1049halt");
    assert_complete(
        &engine
            .frame(ViewportRequest::Pinned(&anchor), rows())
            .unwrap(),
    );
    feed(&mut engine, b"\x1bc");
    assert_complete(&engine.frame(ViewportRequest::FollowLive, rows()).unwrap());
}
#[test]
fn live_and_pinned_frames_report_actual_origin_cursor_modes_and_borrowed_cells() {
    let mut engine = screen();
    feed(&mut engine, b"0\r\n1\r\n2\r\n3\r\n4\x1b[>7u");
    let anchor = engine.create_marker(LineIndex(1)).unwrap();
    let live = engine.frame(ViewportRequest::FollowLive, rows()).unwrap();
    assert_eq!(live.first(), LineIndex(2));
    assert_eq!(live.cells().get(), 24);
    assert_eq!(live.origin(), FrameOrigin::FollowLive);
    assert_eq!(live.cell(Row(0), Column(0)).unwrap().text(), "2");
    assert_eq!(live.input_modes().kitty.get(), 7);
    assert!(matches!(
        live.cursor(),
        FrameCursor::Present(Cursor { row: Row(2), .. })
    ));
    let pinned = engine
        .frame(ViewportRequest::Pinned(&anchor), rows())
        .unwrap();
    assert_eq!(pinned.first(), LineIndex(1));
    assert_eq!(pinned.cursor(), FrameCursor::OutsideViewport);
    assert_eq!(
        pinned.origin(),
        FrameOrigin::Pinned {
            projection: AnchorProjection {
                presence: BufferPresence::Active,
                state: AnchorState::Retained(LineIndex(1))
            },
            adjustment: ViewportAdjustment::Unchanged,
        }
    );
    let cell = pinned.cell(Row(0), Column(0)).unwrap();
    assert_eq!(cell.text(), "1");
    assert_eq!(
        cell.text().as_ptr(),
        engine
            .buffer_cell(LineIndex(1), Column(0))
            .unwrap()
            .text()
            .as_ptr()
    );
    assert!(pinned.cell(Row(3), Column(0)).is_none());
    assert!(pinned.cell(Row(0), Column(8)).is_none());
}
#[test]
fn new_output_eviction_reset_and_return_live_have_explicit_dispositions() {
    let mut engine = screen();
    feed(&mut engine, b"0\r\n1\r\n2\r\n3\r\n4");
    let anchor = engine.create_marker(LineIndex(1)).unwrap();
    feed(&mut engine, b"\r\n5\r\n6");
    let retained = engine
        .frame(ViewportRequest::Pinned(&anchor), rows())
        .unwrap();
    assert_eq!(retained.first(), LineIndex(0));
    assert_eq!(retained.cell(Row(0), Column(0)).unwrap().text(), "1");
    feed(&mut engine, b"\r\n7\r\n8\r\n9");
    let evicted = engine
        .frame(ViewportRequest::Pinned(&anchor), rows())
        .unwrap();
    assert_eq!(evicted.first(), LineIndex(0));
    assert!(matches!(
        evicted.origin(),
        FrameOrigin::Pinned {
            adjustment: ViewportAdjustment::Evicted,
            ..
        }
    ));
    feed(&mut engine, b"\x1b[3J");
    let reset = engine
        .frame(ViewportRequest::Pinned(&anchor), rows())
        .unwrap();
    assert!(matches!(
        reset.origin(),
        FrameOrigin::Pinned {
            adjustment: ViewportAdjustment::Reset,
            ..
        }
    ));
    assert_eq!(reset.first(), LineIndex(0));
    assert_eq!(engine.release_marker(&anchor), AnchorRelease::Reset);
    assert_eq!(
        engine
            .frame(ViewportRequest::FollowLive, rows())
            .unwrap()
            .origin(),
        FrameOrigin::FollowLive
    );
}
#[test]
fn inactive_primary_and_latent_alternate_columns_cannot_retarget_the_frame() {
    let mut engine = screen();
    feed(&mut engine, b"0\r\n1\r\n2\r\n3\r\n4");
    let primary = engine.create_marker(LineIndex(1)).unwrap();
    // This fixture isolates latent-column bounds, not alternate entry position.
    // The separate headless oracle verifies bare1049 inheritance at nonzero/pending columns.
    feed(&mut engine, b"\x1b[?1049h\x1b[HABCDEFG");
    engine.resize(Size::new(4, 3).unwrap()).unwrap();
    let alternate = engine
        .frame(ViewportRequest::Pinned(&primary), rows())
        .unwrap();
    assert_eq!(alternate.buffer_kind(), BufferKind::Alternate);
    assert_eq!(alternate.columns(), 4);
    assert_eq!(alternate.cells().get(), 12);
    assert_eq!(alternate.cell(Row(0), Column(0)).unwrap().text(), "A");
    assert!(alternate.cell(Row(0), Column(4)).is_none());
    assert_eq!(
        engine.buffer_cell(LineIndex(0), Column(6)).unwrap().text(),
        "G"
    );
    assert!(matches!(
        alternate.origin(),
        FrameOrigin::Pinned {
            adjustment: ViewportAdjustment::Inactive,
            ..
        }
    ));
    feed(&mut engine, b"\x1b[?1049l");
    assert_eq!(
        engine
            .frame(ViewportRequest::Pinned(&primary), rows())
            .unwrap()
            .buffer_kind(),
        BufferKind::Primary
    );
}
#[test]
fn released_handles_refuse_but_retained_snapshots_keep_their_own_cut() {
    let mut engine = screen();
    feed(&mut engine, b"0\r\n1\r\n2\r\n3\r\n4\x1b[>7u");
    let anchor = engine.create_marker(LineIndex(1)).unwrap();
    let snapshot = engine.snapshot();
    assert_eq!(engine.release_marker(&anchor), AnchorRelease::Released);
    assert_eq!(
        engine
            .frame(ViewportRequest::Pinned(&anchor), rows())
            .unwrap_err(),
        ScreenError::MarkerReleased
    );
    feed(&mut engine, b"\x1b[>0u\r\n5");
    drop(engine);
    let final_frame = snapshot
        .frame(ViewportRequest::Pinned(&anchor), rows())
        .unwrap();
    assert_eq!(final_frame.first(), LineIndex(1));
    assert_eq!(final_frame.input_modes().kitty.get(), 7);
    assert_eq!(final_frame.cell(Row(0), Column(0)).unwrap().text(), "1");
}
#[test]
fn rows_are_validated_and_below_live_markers_report_the_clamp() {
    assert_eq!(FrameRows::new(0), Err(ScreenError::Geometry));
    let mut engine = screen();
    let anchor = engine.create_marker(LineIndex(2)).unwrap();
    let frame = engine
        .frame(
            ViewportRequest::Pinned(&anchor),
            FrameRows::new(100).unwrap(),
        )
        .unwrap();
    assert_eq!(frame.first(), LineIndex(0));
    assert_eq!(frame.rows(), rows());
    assert_eq!(frame.cells().get(), 24);
    assert!(matches!(
        frame.origin(),
        FrameOrigin::Pinned {
            adjustment: ViewportAdjustment::BeyondLive,
            ..
        }
    ));
}
