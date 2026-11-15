use super::*;
use vessel_proto::terminals::{CaptureLine, CaptureRange};
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
    let (_endpoint, mut authority) =
        ScreenAuthority::channel(id, TerminalSize { rows: 2, cols: 8 });
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
    let (_endpoint, mut authority) =
        ScreenAuthority::channel(TerminalId::random(), TerminalSize { rows: 2, cols: 8 });
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
}
#[tokio::test]
async fn final_screen_survives_actor_shutdown_without_replaying_the_ring() {
    let (endpoint, mut authority) =
        ScreenAuthority::channel(TerminalId::random(), TerminalSize { rows: 2, cols: 8 });
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
    let (endpoint, mut authority) =
        ScreenAuthority::channel(TerminalId::random(), TerminalSize { rows: 2, cols: 8 });
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
