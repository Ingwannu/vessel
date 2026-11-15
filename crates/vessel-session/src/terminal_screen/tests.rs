use super::*;
use vessel_proto::terminals::{CaptureLine, CaptureRange, TerminalAddress};
use vessel_proto::{SessionId, TerminalId};
fn options(start: &str, end: &str) -> CaptureOptions {
    CaptureOptions {
        range: CaptureRange {
            start: Some(start.parse::<CaptureLine>().unwrap()),
            end: Some(end.parse::<CaptureLine>().unwrap()),
        },
        ..CaptureOptions::default()
    }
}
#[test]
fn split_utf8_ansi_cursor_rewrites_and_history_are_screen_rows() {
    let id = TerminalId::random();
    let (_endpoint, mut authority) = ScreenAuthority::channel(
        TerminalAddress::Standalone(id),
        TerminalSize { rows: 2, cols: 8 },
    );
    let bytes = "한글".as_bytes();
    authority.output(&bytes[..2]);
    authority.output(&bytes[2..]);
    authority.output(b"\r\x1b[2Kok\r\nnext\r\nlast");
    let all = authority.capture(CaptureOptions::default()).unwrap();
    assert_eq!(all.terminal, id);
    assert_eq!(all.lines, ["ok", "next", "last"]);
    assert_eq!(all.total_lines.get(), 3);
    assert_eq!(
        authority.capture(options("-2", "-1")).unwrap().lines,
        ["next", "last"]
    );
    assert!(
        authority
            .capture(options("2", "0"))
            .unwrap()
            .lines
            .is_empty()
    );
}
#[test]
fn resizing_retains_geometry_and_oversized_geometry_refuses_explicitly() {
    let (_endpoint, mut authority) = ScreenAuthority::channel(
        TerminalAddress::Standalone(TerminalId::random()),
        TerminalSize { rows: 2, cols: 8 },
    );
    authority.output(b"first\r\nsecond");
    authority.resize(TerminalSize { rows: 3, cols: 12 });
    assert_eq!(
        authority.capture(CaptureOptions::default()).unwrap().lines,
        ["first", "second", ""]
    );
    authority.resize(TerminalSize {
        rows: u16::MAX,
        cols: u16::MAX,
    });
    assert_eq!(
        authority.capture(CaptureOptions::default()),
        Err(CaptureError::Geometry)
    );
    assert!(geometry(TerminalSize { rows: 0, cols: 8 }).is_err());
    assert!(geometry(TerminalSize { rows: 2, cols: 1 }).is_err());
}
#[test]
fn shared_engine_feed_advances_each_prefix_once_even_across_split_utf8() {
    let (_endpoint, mut authority) = ScreenAuthority::channel(
        TerminalAddress::Standalone(TerminalId::random()),
        TerminalSize { rows: 2, cols: 8 },
    );
    let mut bytes = vec![b'a'; vessel_terminal_screen::FEED_BUDGET.saturating_sub(1)];
    bytes.extend_from_slice("한".as_bytes());
    bytes.extend_from_slice(b"\r\nend");
    authority.output(&bytes);
    let captured = authority.capture(CaptureOptions::default()).unwrap();
    assert!(captured.lines.iter().any(|line| line.contains('한')));
    assert!(captured.lines.iter().all(|line| !line.contains('\u{fffd}')));
    assert_eq!(captured.lines.last().map(String::as_str), Some("end"));
}
#[test]
fn capture_observer_stays_silent_and_preserve_does_not_synthesize_style_bytes() {
    let (_endpoint, mut authority) = ScreenAuthority::channel(
        TerminalAddress::Standalone(TerminalId::random()),
        TerminalSize { rows: 2, cols: 8 },
    );
    authority.output(b"\x1b[31mred\x1b[0m\x1b[6n\x1b[c");
    let plain = authority.capture(CaptureOptions::default()).unwrap();
    let preserved = authority
        .capture(CaptureOptions {
            style: vessel_proto::terminals::CaptureStyle::Preserve,
            ..CaptureOptions::default()
        })
        .unwrap();
    // The actor's feed rejects nonempty replies. Success also proves this
    // observer did not answer queries already visible to legacy raw clients.
    assert_eq!(plain, preserved);
    assert_eq!(plain.lines, ["red", ""]);
}
#[tokio::test]
async fn final_screen_survives_actor_shutdown_without_replaying_the_ring() {
    let (endpoint, mut authority) = ScreenAuthority::channel(
        TerminalAddress::Standalone(TerminalId::random()),
        TerminalSize { rows: 2, cols: 8 },
    );
    authority.output(b"final");
    authority.finish();
    drop(authority);
    assert_eq!(
        endpoint
            .capture(CaptureOptions::default())
            .await
            .unwrap()
            .lines,
        ["final", ""]
    );
}
#[tokio::test]
async fn capture_request_and_resize_share_the_actor_inbox_order() {
    let (endpoint, mut authority) = ScreenAuthority::channel(
        TerminalAddress::Standalone(TerminalId::random()),
        TerminalSize { rows: 2, cols: 8 },
    );
    endpoint.resize(TerminalSize { rows: 3, cols: 8 }).unwrap();
    let task = tokio::spawn(async move {
        let resized = authority.next().await.unwrap();
        authority.command(resized);
        let request = authority.next().await.unwrap();
        authority.command(request);
    });
    assert_eq!(
        endpoint
            .capture(CaptureOptions::default())
            .await
            .unwrap()
            .total_lines
            .get(),
        3
    );
    task.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn session_target_cannot_be_reinterpreted_as_a_standalone_capture() {
    let (endpoint, mut authority) = ScreenAuthority::channel(
        TerminalAddress::Session(SessionId::random()),
        TerminalSize { rows: 2, cols: 8 },
    );
    authority.output(b"session TUI");
    assert_eq!(
        endpoint.capture(CaptureOptions::default()).await,
        Err(CaptureError::WrongTarget)
    );
    assert!(matches!(
        authority.inbox.try_recv(),
        Err(mpsc::error::TryRecvError::Empty)
    ));
    authority.finish();
    assert_eq!(
        endpoint.capture(CaptureOptions::default()).await,
        Err(CaptureError::WrongTarget)
    );
}

#[tokio::test]
async fn final_owner_is_sealed_and_second_finish_does_not_replace_the_retained_cut() {
    let (endpoint, mut authority) = ScreenAuthority::channel(
        TerminalAddress::Standalone(TerminalId::random()),
        TerminalSize { rows: 2, cols: 8 },
    );
    authority.output(b"final");
    authority.finish();
    let first = endpoint.final_screen.borrow().clone().unwrap();
    authority.output(b"must not feed after EOF");
    authority.resize(TerminalSize { rows: 3, cols: 12 });
    authority.finish();
    let second = endpoint.final_screen.borrow().clone().unwrap();
    assert!(Arc::ptr_eq(&first.owner, &second.owner));
    assert_eq!(
        endpoint
            .capture(CaptureOptions::default())
            .await
            .unwrap()
            .lines,
        ["final", ""]
    );
    assert_eq!(second.owner.snapshot().unwrap().size().cols(), 8);
    assert_eq!(second.owner.snapshot().unwrap().size().rows(), 2);
}
