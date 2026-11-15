//! Exact reference tracker outputs; no reference implementation is copied into product code.
use super::*;
use crate::{ReplyDuty, Screen, ScrollbackLines, Size};
use base64::Engine as _;

#[derive(serde::Deserialize)]
struct Corpus {
    source: Provenance,
    cases: Vec<Case>,
}
#[derive(serde::Deserialize)]
struct Provenance {
    path: String,
    sha256: String,
}
#[derive(serde::Deserialize)]
struct Case {
    name: String,
    steps: Vec<Step>,
}
#[derive(serde::Deserialize)]
struct Step {
    bytes: String,
    state: Expected,
    responses: Vec<String>,
    modified_enter: bool,
}
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Expected {
    kitty_keyboard_flags: u32,
    win32_input_mode: bool,
    application_cursor_keys: bool,
    bracketed_paste: bool,
}
fn state(enabled: bool) -> ModeState {
    if enabled {
        ModeState::Enabled
    } else {
        ModeState::Disabled
    }
}
fn screen(duty: ReplyDuty) -> Screen {
    Screen::with_replies(Size::new(80, 24).unwrap(), ScrollbackLines::default(), duty).unwrap()
}
#[test]
fn actual_reference_tracker_matches_all_239_whole_and_split_steps() {
    let corpus: Corpus = serde_json::from_str(include_str!("paseo-modes.json")).unwrap();
    assert_eq!(
        corpus.source.path,
        "Paseo packages/protocol/src/terminal-input-mode.ts"
    );
    assert_eq!(
        corpus.source.sha256,
        "7e8bc73aad25e31bd43043abf3119445e7abeecc90c79ead1429c9fac65b707c"
    );
    let mut count = 0;
    for case in corpus.cases {
        let mut engine = screen(ReplyDuty::NodeAuthority);
        for step in case.steps {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(step.bytes)
                .unwrap();
            let result = engine.feed(&bytes);
            assert_eq!(result.consumed.get(), bytes.len());
            assert_eq!(
                result.replies.unwrap().as_bytes(),
                step.responses.concat().as_bytes(),
                "{}",
                case.name
            );
            assert_eq!(
                engine.input_modes(),
                InputModes {
                    kitty: KittyFlags(step.state.kitty_keyboard_flags),
                    win32: state(step.state.win32_input_mode),
                    application_cursor: state(step.state.application_cursor_keys),
                    bracketed_paste: state(step.state.bracketed_paste),
                },
                "{}",
                case.name
            );
            assert_eq!(
                engine.input_modes().modified_enter(),
                state(step.modified_enter)
            );
            count += 1;
        }
    }
    assert_eq!(count, 239);
}
#[test]
fn silent_observer_tracks_modes_and_snapshot_cut_without_answering() {
    let mut engine = screen(ReplyDuty::Silent);
    assert!(
        engine
            .feed(b"\x1b[>7u\x1b[?1;2004;9001h\x1b[?u")
            .replies
            .unwrap()
            .as_bytes()
            .is_empty()
    );
    let retained = engine.snapshot();
    assert_eq!(retained.input_modes(), engine.input_modes());
    assert_eq!(
        engine
            .feed(b"\x1b[<u\x1b[?1;2004;9001l")
            .replies
            .unwrap()
            .as_bytes(),
        b""
    );
    assert_eq!(engine.input_modes(), InputModes::default());
    assert_eq!(retained.input_modes().kitty.get(), 7);
    assert_eq!(retained.input_modes().bracketed_paste, ModeState::Enabled);
}
#[test]
fn typed_raw_flags_are_not_silently_clamped_to_the_decoder_u16() {
    let mut engine = screen(ReplyDuty::NodeAuthority);
    assert_eq!(
        engine
            .feed(b"\x1b[>65536u\x1b[?u")
            .replies
            .unwrap()
            .as_bytes(),
        b"\x1b[?65536u"
    );
    assert_eq!(
        engine
            .feed(b"\x1b[>4294967295u\x1b[?u")
            .replies
            .unwrap()
            .as_bytes(),
        b"\x1b[?4294967295u"
    );
    assert_eq!(
        engine.feed(b"\x1b[>4294967296u").replies,
        Err(ScreenError::InputModeUnavailable)
    );
    assert_eq!(engine.input_modes().kitty.get(), u32::MAX);
}
#[test]
fn stack_capacity_refuses_without_replacing_the_saved_modes() {
    let mut engine = screen(ReplyDuty::NodeAuthority);
    for _ in 0..MAX_KEYBOARD_STACK {
        assert_eq!(engine.feed(b"\x1b[>7u").replies.unwrap().as_bytes(), b"");
    }
    assert_eq!(
        engine.feed(b"\x1b[>0u").replies,
        Err(ScreenError::InputModeUnavailable)
    );
    assert_eq!(engine.input_modes().kitty.get(), 7);
    assert_eq!(engine.feed(b"\x1b[<1024u").replies.unwrap().as_bytes(), b"");
    assert_eq!(engine.input_modes().kitty.get(), 0);
}
#[test]
fn mode_like_osc_data_and_encoded_input_are_not_negotiation() {
    let mut engine = screen(ReplyDuty::NodeAuthority);
    engine.feed(b"\x1b]2;title \x1b[>7u").replies.unwrap();
    // ESC cancels OSC and really starts a CSI: that valid command is decoded once.
    assert_eq!(engine.input_modes().kitty.get(), 7);
    engine
        .feed(b"\x1b[<u\x1bPq\x1b[>9u\x1b\\\x1b[13;2u")
        .replies
        .unwrap();
    // DCS also ends at ESC, so the embedded command is a real parser command here.
    assert_eq!(engine.input_modes().kitty.get(), 9);
    let result = engine.feed(b"\x1b[>1:2u\x1b[13;2u");
    assert!(result.replies.unwrap().as_bytes().is_empty());
    assert_eq!(engine.input_modes().kitty.get(), 9);
}

#[test]
fn authority_reset_is_new_construction_not_an_in_band_ris_tracker_reset() {
    let mut engine = screen(ReplyDuty::Silent);
    assert!(
        engine
            .feed(b"\x1b[>7u\x1b[?9001h\x1bc")
            .replies
            .unwrap()
            .as_bytes()
            .is_empty()
    );
    assert_eq!(engine.input_modes().kitty.get(), 7);
    assert_eq!(engine.input_modes().win32, ModeState::Enabled);
    assert_eq!(
        screen(ReplyDuty::Silent).input_modes(),
        InputModes::default()
    );
}
