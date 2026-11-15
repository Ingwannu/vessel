//! The oracle executes the reference server's actual registered handlers on headless xterm.
use crate::*;

#[derive(serde::Deserialize)]
struct Fixture {
    name: String,
    input: String,
    replies: String,
}

#[test]
fn server_query_oracle_matches_whole_and_byte_split_feeds() {
    let fixtures: Vec<Fixture> =
        serde_json::from_str(include_str!("../tests/query-corpus.json")).unwrap();
    for fixture in fixtures {
        for chunk_size in [1, FEED_BUDGET] {
            let mut screen = Screen::with_replies(
                Size::new(4, 3).unwrap(),
                ScrollbackLines::default(),
                ReplyDuty::NodeAuthority,
            )
            .unwrap();
            let mut replies = Vec::new();
            for bytes in fixture.input.as_bytes().chunks(chunk_size) {
                let feed = screen.feed(bytes);
                assert_eq!(feed.consumed.get(), bytes.len());
                replies.extend_from_slice(feed.replies.unwrap().as_bytes());
            }
            assert_eq!(
                replies,
                fixture.replies.as_bytes(),
                "{} chunk {chunk_size}",
                fixture.name
            );
        }
        let mut observer =
            Screen::new(Size::new(4, 3).unwrap(), ScrollbackLines::default()).unwrap();
        assert!(
            observer
                .feed(fixture.input.as_bytes())
                .replies
                .unwrap()
                .as_bytes()
                .is_empty()
        );
    }
}

#[test]
fn carried_query_and_burst_remain_bounded_and_ordered() {
    let mut screen = Screen::with_replies(
        Size::new(4, 3).unwrap(),
        ScrollbackLines::default(),
        ReplyDuty::NodeAuthority,
    )
    .unwrap();
    assert!(
        screen
            .feed(b"\x1b]10;?")
            .replies
            .unwrap()
            .as_bytes()
            .is_empty()
    );
    let input = [
        b"\x07".as_slice(),
        b"\x1b[5n".repeat(FEED_BUDGET).as_slice(),
    ]
    .concat();
    let mut left = input.as_slice();
    let mut all = Vec::new();
    while !left.is_empty() {
        let feed = screen.feed(left);
        assert_eq!(feed.consumed.get(), left.len().min(FEED_BUDGET));
        let replies = feed.replies.unwrap();
        assert!(replies.as_bytes().len() <= MAX_REPLY_BYTES);
        all.extend_from_slice(replies.as_bytes());
        left = &left[feed.consumed.get()..];
    }
    let expected = [
        b"\x1b]10;rgb:e6e6/e6e6/e6e6\x1b\\".as_slice(),
        b"\x1b[0n".repeat(FEED_BUDGET).as_slice(),
    ]
    .concat();
    assert_eq!(all, expected);
    assert!(screen.feed(b"").replies.unwrap().as_bytes().is_empty());
}
