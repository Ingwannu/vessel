//! The decoder: typed output, the closed query set's shapes, hostile
//! input (bounded, no panic) and the hand-over rule (ADR 0073).

use super::{
    Action, Attributes, ColourSlot, Control, Count, Csi, Dcs, Decoded, Decoder, Erase, FEED_BUDGET,
    Mode, OSC_LIMIT, Osc, Source, Status, Switch,
};

/// Every chunk decoded to its end, in order.
fn decode(chunks: &[(&[u8], Source)]) -> Vec<Decoded> {
    let mut decoder = Decoder::new();
    let mut out = Vec::new();
    for (bytes, source) in chunks {
        let mut rest = *bytes;
        while !rest.is_empty() {
            let taken = decoder.decode(rest, *source, &mut out);
            assert!(taken > 0 && taken <= FEED_BUDGET);
            rest = rest.get(taken..).unwrap_or_default();
        }
    }
    out
}

fn live(bytes: &[u8]) -> Vec<Action> {
    decode(&[(bytes, Source::Live)])
        .into_iter()
        .map(|decoded| decoded.action)
        .collect()
}

fn count(n: u16) -> Count {
    Count::of(n)
}

#[test]
fn text_controls_and_sequences_are_typed() {
    assert_eq!(
        live(b"a\r\nb\x1b[2;3H\x1b[1;38:2:1:2:3m\x1b[?1049h\x1b[K\x1b[3J"),
        [
            Action::Print('a'),
            Action::Control(Control::CarriageReturn),
            Action::Control(Control::LineFeed),
            Action::Print('b'),
            Action::Csi(Csi::Goto(count(2), count(3))),
            Action::Csi(Csi::Graphic(vec![vec![1], vec![38, 2, 1, 2, 3]])),
            Action::Csi(Csi::Modes(Switch::Set, vec![Mode::AltScreenSaveCursor])),
            Action::Csi(Csi::EraseLine(Erase::ToEnd)),
            Action::Csi(Csi::EraseDisplay(Erase::Scrollback)),
        ]
    );
    assert_eq!(live(b"\x1b[m"), [Action::Csi(Csi::Graphic(vec![vec![0]]))]);
}

#[test]
fn a_hangul_syllable_split_across_calls_prints_once() {
    let bytes = "한".as_bytes();
    let (head, tail) = bytes.split_at(1);
    let actions: Vec<_> = decode(&[(head, Source::Live), (tail, Source::Live)])
        .into_iter()
        .map(|decoded| decoded.action)
        .collect();
    assert_eq!(actions, [Action::Print('한')]);
}

#[test]
fn a_bare_save_or_restore_is_scosc_and_one_with_margins_is_not() {
    // `vte` sends a bare sequence as one parameter of 0.
    assert_eq!(live(b"\x1b[s"), [Action::Csi(Csi::SaveCursor)]);
    assert_eq!(live(b"\x1b[0s"), [Action::Csi(Csi::SaveCursor)]);
    assert_eq!(live(b"\x1b[u"), [Action::Csi(Csi::RestoreCursor)]);
    assert_eq!(live(b"\x1b[2;40s"), [Action::Csi(Csi::Unsupported)]);
    assert_eq!(live(b"\x1b[1u"), [Action::Csi(Csi::Unsupported)]);
}

#[test]
fn the_answered_queries_have_their_own_shapes() {
    assert_eq!(
        live(b"\x1b[c\x1b[0c\x1b[5n\x1b[6n\x1b[?6n\x1b]10;?\x07\x1b]11;?\x1b\\\x1b]12;?\x07"),
        [
            Action::Csi(Csi::Attributes(Attributes::Primary)),
            Action::Csi(Csi::Attributes(Attributes::Primary)),
            Action::Csi(Csi::Status(Status::Operating)),
            Action::Csi(Csi::Status(Status::Cursor)),
            Action::Csi(Csi::Status(Status::PrivateCursor)),
            Action::Osc(Osc::ColourQuery(ColourSlot::Foreground)),
            Action::Osc(Osc::ColourQuery(ColourSlot::Background)),
            Action::Esc(super::Esc::Other),
            Action::Osc(Osc::ColourQuery(ColourSlot::Cursor)),
        ]
    );
}

/// Each query ADR 0073 refuses by name decodes to a shape the answering
/// slice never replies to.
#[test]
fn refused_queries_decode_as_refused() {
    let refused: [(&[u8], Action); 15] = [
        (b"\x1b[20t", Action::Csi(Csi::RefusedQuery)),
        (b"\x1b[21t", Action::Csi(Csi::RefusedQuery)),
        (b"\x1b[14t", Action::Csi(Csi::RefusedQuery)),
        (b"\x1b[16t", Action::Csi(Csi::RefusedQuery)),
        (b"\x1b[18t", Action::Csi(Csi::RefusedQuery)),
        (b"\x1b[19t", Action::Csi(Csi::RefusedQuery)),
        (b"\x1bP$qm\x1b\\", Action::Dcs(Dcs::RequestSetting)),
        (b"\x1bP+q544e\x1b\\", Action::Dcs(Dcs::TermcapQuery)),
        (b"\x1b]52;c;?\x07", Action::Osc(Osc::ClipboardRead)),
        (b"\x05", Action::Control(Control::Enquiry)),
        (b"\x1b[?1$p", Action::Csi(Csi::RefusedQuery)),
        (b"\x1b[4$p", Action::Csi(Csi::RefusedQuery)),
        (b"\x1b[>c", Action::Csi(Csi::Attributes(Attributes::Other))),
        (b"\x1b[=c", Action::Csi(Csi::Attributes(Attributes::Other))),
        (b"\x1b]11;rgb:ffff/0000/0000\x07", Action::Osc(Osc::Other)),
    ];
    for (bytes, expected) in refused {
        let actions = live(bytes);
        assert_eq!(actions.first(), Some(&expected), "{bytes:?}");
    }
    // Two colour queries in one OSC are not the answered shape either.
    assert_eq!(live(b"\x1b]10;?;?\x07"), [Action::Osc(Osc::Other)]);
}

#[test]
fn huge_parameters_saturate_and_zero_means_one() {
    assert_eq!(
        live(b"\x1b[99999999999999A\x1b[0;0H\x1b[65535;65535H\x1b[999;5r"),
        [
            Action::Csi(Csi::Up(count(u16::MAX))),
            Action::Csi(Csi::Goto(Count::ONE, Count::ONE)),
            Action::Csi(Csi::Goto(count(u16::MAX), count(u16::MAX))),
            Action::Csi(Csi::ScrollRegion(Some(count(999)), Some(count(5)))),
        ]
    );
}

#[test]
fn too_many_parameters_keep_at_most_the_parsers_limit() {
    let mut bytes = b"\x1b[".to_vec();
    for _ in 0..200 {
        bytes.extend_from_slice(b"1;");
    }
    bytes.push(b'm');
    let actions = live(&bytes);
    assert_eq!(actions.len(), 1);
    if let Some(Action::Csi(Csi::Graphic(groups))) = actions.first() {
        assert!(groups.iter().map(Vec::len).sum::<usize>() <= 32);
    }
}

#[test]
fn an_overlong_osc_is_cut_inside_the_parser() {
    let mut bytes = b"\x1b]2;".to_vec();
    bytes.resize(bytes.len() + 4 * 1024 * 1024, b'x');
    bytes.push(0x07);
    let actions = live(&bytes);
    let [Action::Osc(Osc::Title(title))] = actions.as_slice() else {
        panic!("{:?}", actions.len());
    };
    assert!(title.len() <= OSC_LIMIT, "{}", title.len());
}

#[test]
fn a_dcs_keeps_none_of_its_data() {
    let mut bytes = b"\x1bP1;2|".to_vec();
    bytes.resize(bytes.len() + 4 * 1024 * 1024, b'y');
    bytes.extend_from_slice(b"\x1b\\after");
    let actions = live(&bytes);
    // The DCS, then ST's `\\` (an escape of its own), then the text.
    assert_eq!(actions.first(), Some(&Action::Dcs(Dcs::Other)));
    assert_eq!(actions.len(), 2 + "after".len());
}

#[test]
fn can_aborts_a_sequence() {
    let actions = live(b"\x1b[6\x18n");
    assert!(!actions.contains(&Action::Csi(Csi::Status(Status::Cursor))));
    assert_eq!(actions.last(), Some(&Action::Print('n')));
}

#[test]
fn one_call_takes_at_most_the_budget() {
    let mut decoder = Decoder::new();
    let mut out = Vec::new();
    let bytes = vec![b'a'; FEED_BUDGET + 4096];
    assert_eq!(decoder.decode(&bytes, Source::Live, &mut out), FEED_BUDGET);
    assert_eq!(out.len(), FEED_BUDGET);
}

fn tags(chunks: &[(&[u8], Source)]) -> Vec<(Action, Source)> {
    decode(chunks)
        .into_iter()
        .map(|decoded| (decoded.action, decoded.began))
        .collect()
}

const CURSOR: Action = Action::Csi(Csi::Status(Status::Cursor));

#[test]
fn records_buffered_during_load_are_not_answered() {
    // What arrived while history loaded is applied with replay.
    assert_eq!(
        tags(&[(b"x\x1b[6n", Source::Replay), (b"\x1b[6n", Source::Live)]),
        [
            (Action::Print('x'), Source::Replay),
            (CURSOR, Source::Replay),
            (CURSOR, Source::Live),
        ]
    );
}

#[test]
fn a_query_split_across_the_handover_is_not_answered() {
    assert_eq!(
        tags(&[(b"\x1b[6", Source::Replay), (b"n\x1b[6n", Source::Live)]),
        [(CURSOR, Source::Replay), (CURSOR, Source::Live)]
    );
    // An OSC whose ST arrives live.
    assert_eq!(
        tags(&[(b"\x1b]11;?", Source::Replay), (b"\x1b\\", Source::Live)])
            .first()
            .cloned(),
        Some((
            Action::Osc(Osc::ColourQuery(ColourSlot::Background)),
            Source::Replay
        ))
    );
    // A control inside the sequence keeps it open; the end is still replay's.
    assert_eq!(
        tags(&[
            (b"\x1b[6", Source::Replay),
            (b"\n", Source::Live),
            (b"n", Source::Live),
        ]),
        [
            (Action::Control(Control::LineFeed), Source::Replay),
            (CURSOR, Source::Replay),
        ]
    );
}

#[test]
fn a_sequence_begun_live_after_an_abandoned_one_is_live() {
    // ESC starts a new sequence; CAN abandons the old one.
    assert_eq!(
        tags(&[(b"\x1b[6", Source::Replay), (b"\x1b[6n", Source::Live)]),
        [(CURSOR, Source::Live)]
    );
    assert_eq!(
        tags(&[(b"\x1b[6", Source::Replay), (b"\x18\x1b[6n", Source::Live)])
            .last()
            .cloned(),
        Some((CURSOR, Source::Live))
    );
}

/// A fixed-seed generator (xorshift64), so a failure is repeatable.
struct Bytes(u64);

impl Bytes {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0.wrapping_shl(13);
        self.0 ^= self.0.wrapping_shr(7);
        self.0 ^= self.0.wrapping_shl(17);
        self.0
    }

    /// A value below `bound` (at least 1).
    fn below(&mut self, bound: u64) -> u64 {
        self.next().checked_rem(bound).unwrap_or(0)
    }

    /// A byte, half the time from the grammar's own bytes so sequences
    /// open, nest and break often.
    fn byte(&mut self) -> u8 {
        const GRAMMAR: &[u8] =
            b"\x1b\x1b\x1b[]P?;;0123456789$+> \x07\x18\x1a\\mHJKnctq\x9b\xc3\xed\x95";
        let [low, pick, ..] = self.next().to_le_bytes();
        if pick.is_multiple_of(2) {
            usize::from(low)
                .checked_rem(GRAMMAR.len())
                .and_then(|at| GRAMMAR.get(at).copied())
                .unwrap_or(0)
        } else {
            low
        }
    }
}

#[test]
fn random_bytes_never_panic_and_stay_bounded() {
    let mut random = Bytes(0x5eed_7e44_1a1b_2c3d);
    let mut decoder = Decoder::new();
    let mut out = Vec::new();
    let mut total = 0_usize;
    while total < 4 * 1024 * 1024 {
        let length = usize::try_from(random.below(40_000)).unwrap_or(1);
        let chunk: Vec<u8> = (0..length).map(|_| random.byte()).collect();
        let source = if random.below(3) == 0 {
            Source::Replay
        } else {
            Source::Live
        };
        let mut rest = chunk.as_slice();
        while !rest.is_empty() {
            out.clear();
            let taken = decoder.decode(rest, source, &mut out);
            assert!(taken > 0 && taken <= FEED_BUDGET);
            // At most one action per byte.
            assert!(out.len() <= taken);
            for decoded in &out {
                if let Action::Osc(Osc::Title(title)) = &decoded.action {
                    assert!(title.len() <= OSC_LIMIT.saturating_mul(3));
                }
                if decoded.began == Source::Live {
                    assert_eq!(source, Source::Live, "live only from live bytes");
                }
            }
            rest = rest.get(taken..).unwrap_or_default();
        }
        total = total.saturating_add(length);
    }
}
