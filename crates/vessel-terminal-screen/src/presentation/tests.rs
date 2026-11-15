//! Raw-client and replay regressions use a silent copy of the sole shared engine.
use super::*;
use crate::{CaptureRange, Column, Row};

fn mux() -> Multiplexer {
    Multiplexer::new(Size::new(80, 24).unwrap(), ScrollbackLines::default()).unwrap()
}

#[test]
fn every_query_split_omits_requests_but_keeps_replies_and_rendering() {
    for query in [
        b"\x1b[c".as_slice(),
        b"\x1b[5n",
        b"\x1b[6n",
        b"\x1b[?6n",
        b"\x1b]10;?\x07",
        b"\x1b]11;?\x1b\\",
        b"\x1b]12;?\x1b\\",
    ] {
        for split in 0..=query.len() {
            let mut authority = mux();
            let a = authority.feed(&query[..split]);
            let b = authority.feed(&query[split..]);
            assert_eq!(a.outcome, Ok(()));
            assert_eq!(b.outcome, Ok(()));
            assert!(a.presentation.as_bytes().is_empty(), "{query:?}/{split}");
            assert!(b.presentation.as_bytes().is_empty(), "{query:?}/{split}");
            assert!(!a.replies.as_bytes().is_empty() || !b.replies.as_bytes().is_empty());
            assert!(a.events.is_empty() && b.events.is_empty());
        }
    }
}

#[test]
fn old_raw_client_and_history_replay_never_receive_normal_query_requests() {
    let mut authority = mux();
    let source = b"\x1b[31mred\x1b[0m\x1b[6n\r\n\x1b]2;title\x07end\x1b]11;?\x1b\\";
    let expected = b"\x1b[31mred\x1b[0m\r\n\x1b]2;title\x07end";
    let mut records = Vec::new();
    let mut node_replies = Vec::new();
    for &byte in source {
        let result = authority.feed(&[byte]);
        assert_eq!(result.outcome, Ok(()));
        assert_eq!(result.consumed.get(), 1);
        records.push(result.presentation.as_bytes().to_vec());
        node_replies.extend_from_slice(result.replies.as_bytes());
    }
    assert_eq!(records.concat(), expected);
    assert!(records.iter().any(Vec::is_empty));
    assert_eq!(node_replies, b"\x1b[1;4R\x1b]11;rgb:0b0b/0b0b/0b0b\x1b\\");
    for _attach_or_replay in 0..2 {
        let mut client =
            Screen::new(Size::new(80, 24).unwrap(), ScrollbackLines::default()).unwrap();
        for record in &records {
            assert!(client.feed(record).replies.unwrap().as_bytes().is_empty());
        }
        assert_eq!(
            client.capture(CaptureRange::default()).unwrap(),
            authority.screen().capture(CaptureRange::default()).unwrap()
        );
    }
}

#[test]
fn unfinished_utf8_controls_and_unknown_sequences_are_preserved() {
    let mut authority = mux();
    let source = "한글\x1b[?2004h\x1b]2;hello\x1b\\\x1b[?9999n\x1bP1;2qpayload\x1b\\".as_bytes();
    let mut output = Vec::new();
    for &byte in source {
        let result = authority.feed(&[byte]);
        assert_eq!(result.outcome, Ok(()));
        assert!(result.replies.as_bytes().is_empty());
        output.extend_from_slice(result.presentation.as_bytes());
    }
    assert_eq!(output, source);
    assert_eq!(
        authority.screen().cell(Row(0), Column(0)).unwrap().text(),
        "한"
    );
    let mut pending = mux();
    assert!(pending.feed(b"\x1b[6").presentation.as_bytes().is_empty());
    assert_eq!(pending.feed(b"n").presentation.as_bytes(), b"");
}

#[test]
fn executed_controls_survive_inside_an_omitted_query() {
    let mut authority = mux();
    let result = authority.feed(b"\x1b[\x07\r\n6n");
    assert_eq!(result.outcome, Ok(()));
    assert_eq!(result.presentation.as_bytes(), b"\x07\r\n");
    assert_eq!(result.replies.as_bytes(), b"\x1b[2;1R");
}

#[test]
fn ambiguous_title_and_cancelled_csi_spans_remain_unchanged_and_unanswered() {
    for source in [
        b"\x1b]2;title\x1b[6n".as_slice(),
        b"\x1b[12\x1b[6n",
        b"\xe4\x1b[6n",
    ] {
        for split in 0..=source.len() {
            let mut authority = mux();
            let a = authority.feed(&source[..split]);
            let b = authority.feed(&source[split..]);
            assert_eq!(a.outcome, Ok(()));
            assert_eq!(b.outcome, Ok(()));
            assert!(a.replies.as_bytes().is_empty() && b.replies.as_bytes().is_empty());
            assert_eq!(
                [a.presentation.as_bytes(), b.presentation.as_bytes()].concat(),
                source
            );
            assert_eq!(a.events.len() + b.events.len(), 1);
            let event = a.events.first().or(b.events.first()).unwrap();
            let PresentationEvent::AmbiguousOverlap { start, end } = event;
            assert_eq!(start.get(), 0);
            assert_eq!(end.get(), source.len());
        }
    }
}

#[test]
fn token_refusal_preserves_progress_and_never_publishes_the_partial_token() {
    let mut authority = mux();
    let token = [b"\x1b]2;".as_slice(), &vec![b'x'; MAX_CONTROL_TOKEN_BYTES]].concat();
    let mut consumed = 0;
    loop {
        let result = authority.feed(&token[consumed..]);
        consumed += result.consumed.get();
        assert!(result.presentation.as_bytes().is_empty());
        if result.outcome.is_err() {
            break;
        }
        assert!(result.consumed.get() > 0);
    }
    assert_eq!(consumed, MAX_CONTROL_TOKEN_BYTES);
    let fenced = authority.feed(b"\x07");
    assert_eq!(fenced.consumed.get(), 0);
    assert_eq!(fenced.outcome, Err(ScreenError::PresentationUnavailable));
}

#[derive(Default)]
struct Parameters(Vec<Vec<vt100::DecodedParameter>>);
impl vt100::Callbacks for Parameters {
    fn decoded_csi(
        &mut self,
        _: &mut vt100::Screen,
        params: &vt100::DecodedParams,
        _: &[u8],
        _: bool,
        _: char,
    ) {
        self.0.push(params.explicit().collect());
    }
}
#[test]
fn decoded_parameter_presence_distinguishes_bare_and_explicit_zero_without_a_scanner() {
    let mut parser = vt100::Parser::new_with_callbacks(
        std::num::NonZeroU16::new(24).unwrap(),
        std::num::NonZeroU16::new(80).unwrap(),
        0,
        Parameters::default(),
    );
    parser.process(b"\x1b[>u\x1b[>0u\x1b[=;0u\x1b[=0;u");
    use vt100::DecodedParameter::{Explicit, Omitted};
    assert_eq!(
        parser.callbacks().0,
        vec![
            vec![Omitted],
            vec![Explicit(0)],
            vec![Omitted, Explicit(0)],
            vec![Explicit(0), Omitted]
        ]
    );
}
