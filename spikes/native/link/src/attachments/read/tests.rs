//! How a sent image's pieces make it whole.

use bytes::Bytes;

use super::*;

fn chunk(data: &'static [u8], total: u64) -> BlobChunk {
    BlobChunk {
        data: Bytes::from_static(data),
        total,
    }
}

#[test]
fn pieces_make_the_image_whole_at_the_nodes_size() {
    let mut bytes = Vec::new();
    assert_eq!(take(&mut bytes, &chunk(b"0123", 10), 64), Ok(Next::More));
    assert_eq!(take(&mut bytes, &chunk(b"4567", 10), 64), Ok(Next::More));
    assert_eq!(take(&mut bytes, &chunk(b"89", 10), 64), Ok(Next::Done));
    assert_eq!(bytes, b"0123456789");
}

#[test]
fn an_empty_image_is_whole_at_once() {
    let mut bytes = Vec::new();
    assert_eq!(take(&mut bytes, &chunk(b"", 0), 64), Ok(Next::Done));
    assert!(bytes.is_empty());
}

#[test]
fn an_image_larger_than_a_node_takes_is_not_read() {
    let mut bytes = Vec::new();
    assert!(take(&mut bytes, &chunk(b"0123", 65), 64).is_err());
    assert!(bytes.is_empty(), "nothing kept");
}

#[test]
fn a_piece_bringing_nothing_before_the_end_stops_the_read() {
    let mut bytes = Vec::new();
    assert_eq!(take(&mut bytes, &chunk(b"0123", 10), 64), Ok(Next::More));
    let stopped = take(&mut bytes, &chunk(b"", 10), 64);
    assert!(stopped.is_err(), "{stopped:?}");
}
