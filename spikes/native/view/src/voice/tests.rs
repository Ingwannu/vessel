use super::*;

fn target() -> Target {
    Target {
        node: vessel_proto::NodeId::random(),
        session: vessel_proto::SessionId::random(),
    }
}

#[test]
fn every_readiness_refusal_leaves_ownership_empty() {
    for reason in [
        Availability::Disabled,
        Availability::Unsupported,
        Availability::NotReady,
        Availability::Offline,
        Availability::AgentRunning,
        Availability::CaptureBusy,
    ] {
        let mut state = State::default();
        assert_eq!(
            state.start(target(), reason),
            Err(TransitionError::Unavailable(reason))
        );
        assert_eq!(state.phase(), Phase::Disabled);
        assert_eq!(state.target(), None);
    }
}

#[test]
fn node_enable_must_precede_capture_and_mute_suppresses_only_upload() {
    let mut state = State::default();
    let target = target();
    let ticket = state.start(target, Availability::Ready).unwrap();
    assert_eq!(state.phase(), Phase::Starting);
    assert!(!state.can_upload(ticket));
    assert_eq!(state.capture_ready(ticket), Err(TransitionError::Order));
    assert_eq!(state.node_ready(ticket), Ok(Applied::Current));
    assert_eq!(state.capture_ready(ticket), Ok(Applied::Current));
    assert_eq!(state.phase(), Phase::Listening);
    assert!(state.can_upload(ticket));
    assert_eq!(state.toggle_mute(), Ok(Mute::Muted));
    assert!(!state.can_upload(ticket));
    assert_eq!(state.target(), Some(target));
    assert_eq!(state.toggle_mute(), Ok(Mute::Open));
    assert!(state.can_upload(ticket));
}

#[test]
fn cleanup_and_generation_fences_prevent_audio_retargeting() {
    let mut state = State::default();
    let old = state.start(target(), Availability::Ready).unwrap();
    assert_eq!(state.stop(), Ok(old));
    assert!(!state.can_upload(old));
    assert_eq!(state.node_ready(old), Ok(Applied::Stale));
    assert_eq!(
        state.start(target(), Availability::Ready),
        Err(TransitionError::Busy)
    );
    assert_eq!(state.phase(), Phase::Stopping);
    assert_eq!(state.stopped(old), Ok(Applied::Current));
    let new_target = target();
    let new = state.start(new_target, Availability::Ready).unwrap();
    assert_ne!(old, new);
    assert_eq!(state.node_ready(old), Ok(Applied::Stale));
    assert_eq!(state.capture_ready(old), Ok(Applied::Stale));
    assert_eq!(state.stopped(old), Ok(Applied::Stale));
    assert_eq!(state.target(), Some(new_target));
    assert_eq!(state.node_ready(new), Ok(Applied::Current));
    assert_eq!(state.capture_ready(new), Ok(Applied::Current));
    assert!(!state.can_upload(old));
    assert!(state.can_upload(new));
}

#[test]
fn disabled_controls_cannot_invent_a_live_microphone() {
    let mut state = State::default();
    assert_eq!(state.stop(), Err(TransitionError::Inactive));
    assert_eq!(state.toggle_mute(), Err(TransitionError::Inactive));
}

#[test]
fn actual_playback_outlives_turn_end_and_barge_in_resets_it() {
    let mut state = State::default();
    let ticket = state.start(target(), Availability::Ready).unwrap();
    assert_eq!(state.node_ready(ticket), Ok(Applied::Current));
    assert_eq!(state.capture_ready(ticket), Ok(Applied::Current));
    for event in [
        VoiceEvent::SpeechStopped,
        VoiceEvent::TranscriptReady,
        VoiceEvent::TurnStarted,
        VoiceEvent::PlaybackStarted,
        VoiceEvent::TurnEnded,
    ] {
        assert_eq!(state.observe(ticket, event), Ok(Applied::Current));
    }
    assert_eq!(state.phase(), Phase::Playing);
    assert_eq!(
        state.observe(ticket, VoiceEvent::PlaybackFinished),
        Ok(Applied::Current)
    );
    assert_eq!(state.phase(), Phase::Listening);
    assert_eq!(
        state.observe(ticket, VoiceEvent::PlaybackStarted),
        Ok(Applied::Current)
    );
    assert_eq!(
        state.observe(ticket, VoiceEvent::SpeechStarted),
        Ok(Applied::Current)
    );
    assert_eq!(state.phase(), Phase::Listening);
    assert_eq!(
        state.observe(ticket, VoiceEvent::PlaybackFinished),
        Ok(Applied::Stale)
    );
    assert_eq!(state.phase(), Phase::Listening);
}

#[test]
fn rejected_transcript_does_not_start_a_turn() {
    let mut state = State::default();
    let ticket = state.start(target(), Availability::Ready).unwrap();
    assert_eq!(state.node_ready(ticket), Ok(Applied::Current));
    assert_eq!(state.capture_ready(ticket), Ok(Applied::Current));
    assert_eq!(
        state.observe(ticket, VoiceEvent::SpeechStopped),
        Ok(Applied::Current)
    );
    assert_eq!(state.phase(), Phase::Submitting);
    assert_eq!(
        state.observe(ticket, VoiceEvent::TranscriptEmpty),
        Ok(Applied::Current)
    );
    assert_eq!(state.phase(), Phase::Listening);
}

#[test]
fn overlay_disables_both_actions_through_switching_and_keeps_the_last_mute_icon() {
    let mut state = State::default();
    assert_eq!(state.overlay(), Overlay::Absent);
    let generation = state.start(target(), Availability::Ready).unwrap();
    assert_eq!(state.overlay(), Overlay::Switching(Mute::Open));
    assert_eq!(state.node_ready(generation).unwrap(), Applied::Current);
    assert_eq!(state.capture_ready(generation).unwrap(), Applied::Current);
    assert_eq!(state.overlay(), Overlay::Active(Mute::Open));
    assert_eq!(state.toggle_mute().unwrap(), Mute::Muted);
    assert_eq!(state.stop().unwrap(), generation);
    assert_eq!(state.overlay(), Overlay::Switching(Mute::Muted));
    assert_eq!(state.toggle_mute(), Err(TransitionError::Inactive));
    assert!(!state.can_upload(generation));
    assert_eq!(state.stopped(generation).unwrap(), Applied::Current);
    assert_eq!(state.overlay(), Overlay::Absent);
}
