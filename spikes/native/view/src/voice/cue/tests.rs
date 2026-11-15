use super::*;
use crate::voice::{Applied, Availability, Target, VoiceEvent};

fn waiting() -> (State, Generation, Cue) {
    let mut state = State::default();
    let generation = state
        .start(
            Target {
                node: vessel_proto::NodeId::random(),
                session: vessel_proto::SessionId::random(),
            },
            Availability::Ready,
        )
        .unwrap();
    state.node_ready(generation).unwrap();
    state.capture_ready(generation).unwrap();
    state.observe(generation, VoiceEvent::TurnStarted).unwrap();
    (state, generation, Cue::new(generation))
}
fn at(ms: u64) -> Duration {
    Duration::from_millis(ms)
}

#[test]
fn cue_waits_out_segment_gaps_and_repeats_only_after_its_own_settled_playback() {
    let (state, _generation, mut cue) = waiting();
    assert_eq!(
        cue.sync(&state, SpeechActivity::Quiet, at(0)),
        Ok(CueAction::WaitUntil(at(1500)))
    );
    assert_eq!(
        cue.sync(&state, SpeechActivity::Quiet, at(1499)),
        Ok(CueAction::WaitUntil(at(1500)))
    );
    let CueAction::Play(playback) = cue.sync(&state, SpeechActivity::Quiet, at(1500)).unwrap()
    else {
        panic!("cue due")
    };
    assert_eq!(
        cue.sync(&state, SpeechActivity::Quiet, at(5000)),
        Ok(CueAction::None)
    );
    assert_eq!(
        cue.finished(&state, SpeechActivity::Quiet, playback, at(6000)),
        Ok(CueAction::WaitUntil(at(9330)))
    );
    assert_eq!(
        cue.finished(&state, SpeechActivity::Quiet, playback, at(7000)),
        Ok(CueAction::None)
    );
    assert_eq!(
        cue.sync(&state, SpeechActivity::Quiet, at(9329)),
        Ok(CueAction::WaitUntil(at(9330)))
    );
    let CueAction::Play(next) = cue.sync(&state, SpeechActivity::Quiet, at(9330)).unwrap() else {
        panic!("next cue due")
    };
    assert_ne!(playback, next);
}

#[test]
fn speech_or_real_output_cancels_the_cue_and_late_completion_cannot_restart_it() {
    let (mut state, generation, mut cue) = waiting();
    cue.sync(&state, SpeechActivity::Quiet, at(0)).unwrap();
    let CueAction::Play(playback) = cue.sync(&state, SpeechActivity::Quiet, at(1500)).unwrap()
    else {
        panic!("cue due")
    };
    assert_eq!(
        cue.sync(&state, SpeechActivity::Speaking, at(1501)),
        Ok(CueAction::Cancel)
    );
    assert_eq!(
        cue.finished(&state, SpeechActivity::Quiet, playback, at(6000)),
        Ok(CueAction::None)
    );
    state
        .observe(generation, VoiceEvent::PlaybackStarted)
        .unwrap();
    assert_eq!(
        cue.sync(&state, SpeechActivity::Quiet, at(6000)),
        Ok(CueAction::None)
    );
    state
        .observe(generation, VoiceEvent::PlaybackFinished)
        .unwrap();
    assert_eq!(state.phase(), Phase::Waiting);
    assert_eq!(
        cue.sync(&state, SpeechActivity::Quiet, at(6000)),
        Ok(CueAction::WaitUntil(at(7500)))
    );
    state.stop().unwrap();
    assert_eq!(
        cue.sync(&state, SpeechActivity::Quiet, at(7000)),
        Ok(CueAction::Cancel)
    );
    assert_eq!(
        cue.finished(&state, SpeechActivity::Quiet, playback, at(8000)),
        Ok(CueAction::None)
    );
}

#[test]
fn a_previous_generation_and_invalid_clock_cannot_schedule_new_cue_audio() {
    let (mut state, generation, mut cue) = waiting();
    state.toggle_mute().unwrap();
    assert_eq!(
        cue.sync(&state, SpeechActivity::Speaking, at(100)),
        Ok(CueAction::WaitUntil(at(1600)))
    );
    assert_eq!(
        cue.sync(&state, SpeechActivity::Quiet, at(99)),
        Err(CueError::Clock)
    );
    state.stop().unwrap();
    assert_eq!(state.stopped(generation).unwrap(), Applied::Current);
    let next = state
        .start(
            Target {
                node: vessel_proto::NodeId::random(),
                session: vessel_proto::SessionId::random(),
            },
            Availability::Ready,
        )
        .unwrap();
    state.node_ready(next).unwrap();
    state.capture_ready(next).unwrap();
    state.observe(next, VoiceEvent::TurnStarted).unwrap();
    assert_eq!(
        cue.sync(&state, SpeechActivity::Quiet, at(1600)),
        Ok(CueAction::Cancel)
    );
    assert_eq!(
        cue.sync(&state, SpeechActivity::Quiet, at(2000)),
        Ok(CueAction::None)
    );
}
