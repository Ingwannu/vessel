//! The answered set through the model: Paseo's bytes for each answered
//! query, nothing for each refused one, nothing through `replay` or across
//! the hand-over, and which viewer's duty it is (ADR 0073's named
//! fixtures).

use super::{LONGEST_REPLY, REPLY_RATIO};
use crate::terminal::grid::{CursorCell, Rgb, Scroll};
use crate::terminal::paint::Palette;
use crate::terminal::{
    Focus, GridSize, Origin, Others, QueryDuty, ScrollbackLines, TerminalModel, Typed, query_duty,
};

/// A 10 × 4 model whose palette draws `e6e6e6` on `0b0b0b`, cursor `ff8000`.
fn model() -> TerminalModel {
    let size = GridSize::new(10, 4).unwrap_or_default();
    let mut model = TerminalModel::new(size, ScrollbackLines::default());
    model.set_palette(Palette {
        ansi: [Rgb::default(); 16],
        foreground: Rgb {
            r: 0xe6,
            g: 0xe6,
            b: 0xe6,
        },
        background: Rgb {
            r: 0x0b,
            g: 0x0b,
            b: 0x0b,
        },
        cursor: Rgb {
            r: 0xff,
            g: 0x80,
            b: 0x00,
        },
    });
    model
}

/// The replies to `bytes` fed live to a fresh model.
fn live(bytes: &[u8]) -> Vec<u8> {
    model().feed(bytes).replies
}

/// The answered set, each with Paseo's reply bytes.
const ANSWERED: &[(&[u8], &[u8])] = &[
    (b"\x1b[c", b"\x1b[?62;4;22c"),
    (b"\x1b[0c", b"\x1b[?62;4;22c"),
    (b"\x1b[5n", b"\x1b[0n"),
    (b"\x1b[6n", b"\x1b[1;1R"),
    (b"\x1b[?6n", b"\x1b[?1;1R"),
    (b"\x1b]10;?\x07", b"\x1b]10;rgb:e6e6/e6e6/e6e6\x1b\\"),
    (b"\x1b]11;?\x1b\\", b"\x1b]11;rgb:0b0b/0b0b/0b0b\x1b\\"),
    (b"\x1b]12;?\x07", b"\x1b]12;rgb:ffff/8080/0000\x1b\\"),
];

/// ADR 0073's refused queries: none is answered.
const REFUSED: &[&[u8]] = &[
    b"\x1b[20t",
    b"\x1b[21t",
    b"\x1b[14t",
    b"\x1b[16t",
    b"\x1b[18t",
    b"\x1b[19t",
    b"\x1bP$qm\x1b\\",
    b"\x1bP+q544e\x1b\\",
    b"\x1b]52;c;?\x07",
    b"\x05",
    b"\x1b[?2004$p",
    b"\x1b[4$p",
    b"\x1b[>c",
    b"\x1b[=c",
    b"\x1b[1c",
    b"\x1b]10;? \x07",
    b"\x1b]11;rgb:0/0/0\x07",
    b"\x1b]12\x07",
    b"\x1b[7n",
    b"\x1b[?5n",
    b"\x1b[6;1n",
    b"\x1b[5;9n",
    b"\x1b[?6;1n",
    b"\x1b[?u",
];

#[test]
fn each_answered_query_gets_paseos_bytes() {
    for (query, reply) in ANSWERED {
        assert_eq!(live(query), *reply, "{query:?}");
    }
}

#[test]
fn each_refused_query_fed_live_gets_nothing() {
    for query in REFUSED {
        assert!(live(query).is_empty(), "{query:?}");
    }
}

#[test]
fn a_live_cursor_query_answers_the_cursors_cell_and_replay_answers_nothing() {
    let mut term = model();
    assert!(term.feed(b"\x1b[3;7Hab").replies.is_empty());
    assert_eq!(term.feed(b"\x1b[6n").replies, b"\x1b[3;9R");
    assert_eq!(term.feed(b"\x1b[?6n").replies, b"\x1b[?3;9R");
    // A pending wrap reports the last column.
    assert_eq!(term.feed(b"\x1b[4;9Hxy\x1b[6n").replies, b"\x1b[4;10R");
    // The same queries through replay: nothing.
    assert!(
        term.replay(b"\x1b[6n\x1b[?6n\x1b[c\x1b[5n\x1b]10;?\x07")
            .replies
            .is_empty()
    );
}

#[test]
fn queries_are_answered_where_they_stand_among_the_bytes() {
    assert_eq!(
        live(b"\x1b[6nab\x1b[6n\r\n\x1b[6n"),
        b"\x1b[1;1R\x1b[1;3R\x1b[2;1R"
    );
}

#[test]
fn without_a_palette_the_colour_queries_are_not_answered() {
    let size = GridSize::new(10, 4).unwrap_or_default();
    let mut term = TerminalModel::new(size, ScrollbackLines::default());
    assert!(
        term.feed(b"\x1b]10;?\x07\x1b]11;?\x07\x1b]12;?\x07")
            .replies
            .is_empty()
    );
    assert_eq!(term.feed(b"\x1b[5n").replies, b"\x1b[0n");
}

#[test]
fn records_buffered_during_load_are_not_answered() {
    // History and the records that arrived while it loaded: all replay.
    let mut term = model();
    for record in [&b"\x1b[6n"[..], b"\x1b[c", b"\x1b]10;?\x07", b"\x1b[5n"] {
        assert!(term.replay(record).replies.is_empty());
    }
    // The first live record after the hand-over answers its own query.
    assert_eq!(term.feed(b"\x1b[5n").replies, b"\x1b[0n");
}

#[test]
fn a_query_split_across_the_handover_is_not_answered() {
    for split in [1, 2, 3] {
        let mut term = model();
        let (head, tail) = b"\x1b[6n".split_at(split);
        assert!(term.replay(head).replies.is_empty());
        assert!(term.feed(tail).replies.is_empty(), "split at {split}");
        // The next query, begun live, is answered.
        assert_eq!(term.feed(b"\x1b[5n").replies, b"\x1b[0n");
    }
    let mut term = model();
    assert!(term.replay(b"\x1b]10;").replies.is_empty());
    assert!(term.feed(b"?\x07").replies.is_empty());
}

#[test]
fn the_opener_answers_before_anyone_qualifies() {
    for (focus, typed) in [
        (Focus::Lost, Typed::Never),
        (Focus::Held, Typed::Never),
        (Focus::Lost, Typed::Once),
    ] {
        let duty = query_duty(Origin::Opened, focus, typed, Others::NoneQualify);
        assert_eq!(duty, QueryDuty::Answer);
    }
    // And through the model, every answered query from the opener.
    for (query, reply) in ANSWERED {
        assert_eq!(live(query), *reply);
    }
}

#[test]
fn nobody_answers_a_live_query_when_no_viewer_may() {
    for (focus, typed) in [
        (Focus::Lost, Typed::Never),
        (Focus::Held, Typed::Never),
        (Focus::Lost, Typed::Once),
    ] {
        let duty = query_duty(Origin::Attached, focus, typed, Others::NoneQualify);
        assert_eq!(duty, QueryDuty::Silent);
    }
    // `Silent` is `replay`: the same queries, no bytes.
    let mut term = model();
    for (query, _) in ANSWERED {
        assert!(term.replay(query).replies.is_empty());
    }
}

#[test]
fn once_a_viewer_qualifies_only_it_answers() {
    let typing = query_duty(
        Origin::Attached,
        Focus::Held,
        Typed::Once,
        Others::NoneQualify,
    );
    assert_eq!(typing, QueryDuty::Answer);
    let opener = query_duty(
        Origin::Opened,
        Focus::Lost,
        Typed::Never,
        Others::OneQualifies,
    );
    assert_eq!(opener, QueryDuty::Silent);
    let watcher = query_duty(
        Origin::Attached,
        Focus::Held,
        Typed::Never,
        Others::OneQualifies,
    );
    assert_eq!(watcher, QueryDuty::Silent);
}

#[test]
fn replies_stay_within_the_stated_ratio_of_the_bytes_read() {
    let flood = [&b"\x1b]10;?\x07"[..], b"\x1b[c", b"\x1b[?6n"]
        .concat()
        .repeat(2048);
    let mut term = model();
    let mut rest = &flood[..];
    while !rest.is_empty() {
        let feed = term.feed(rest);
        assert!(feed.consumed > 0);
        let bound = feed
            .consumed
            .saturating_mul(REPLY_RATIO)
            .saturating_add(LONGEST_REPLY);
        assert!(feed.replies.len() <= bound);
        rest = rest.get(feed.consumed..).unwrap_or_default();
    }
}

#[test]
fn kittys_keyboard_query_push_pop_and_set_neither_answer_nor_move_the_cursor() {
    let mut term = model();
    assert!(term.feed(b"\x1b[3;4H\x1b7\x1b[1;1H").replies.is_empty());
    for sequence in [&b"\x1b[?u"[..], b"\x1b[>1u", b"\x1b[<u", b"\x1b[=1;1u"] {
        assert!(term.feed(sequence).replies.is_empty(), "{sequence:?}");
        // None of them is a restore of the saved cursor (3;4).
        assert_eq!(
            term.snapshot(Scroll(0)).cursor,
            Some(CursorCell { row: 0, col: 0 })
        );
    }
}
