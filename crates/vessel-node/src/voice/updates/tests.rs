use super::*;
use vessel_proto::voice::{AgentTurn, Playback, PlaybackId, PlaybackPcm, SpeechGroupId};

fn audio() -> VoiceUpdate {
    VoiceUpdate::Audio {
        playback: Playback {
            id: PlaybackId::random(),
            group: SpeechGroupId::random(),
            format: "audio/pcm;rate=24000;bits=16".parse().unwrap(),
            pcm: PlaybackPcm::new(vec![1, 2]).unwrap(),
        },
    }
}
fn control() -> VoiceUpdate {
    VoiceUpdate::Turn {
        state: AgentTurn::Started,
    }
}

#[test]
fn cancelled_waiter_restores_exact_audio_before_another_await() {
    let mut updates = Updates::default();
    let expected = audio();
    let (waiter, response) = oneshot::channel();
    assert_eq!(updates.read(waiter).unwrap(), Delivery::Waiting);
    updates.publish(expected.clone()).unwrap();
    drop(response);
    assert_eq!(updates.deliver(), Delivery::Waiting);
    assert_eq!(updates.audio, 1);
    let (waiter, mut response) = oneshot::channel();
    assert_eq!(updates.read(waiter).unwrap(), Delivery::UpdateReady);
    assert_eq!(response.try_recv().unwrap(), expected);
    assert_eq!(updates.audio, 0);
}

#[test]
fn audio_and_control_limits_are_independent_and_close_does_not_need_queue_room() {
    let mut updates = Updates::default();
    for _ in 0..MAX_AUDIO {
        updates.publish(audio()).unwrap();
    }
    let rejected = audio();
    match updates.publish(rejected.clone()).unwrap_err() {
        PublishRefusal::Capacity(value) => assert_eq!(value, rejected),
        PublishRefusal::Closed(_) | PublishRefusal::Terminal(_) => {
            panic!("expected capacity refusal")
        }
    }
    for _ in 0..MAX_CONTROL {
        updates.publish(control()).unwrap();
    }
    assert!(matches!(
        updates.publish(control()),
        Err(PublishRefusal::Capacity(_))
    ));
    updates.close(Closed::Stopped).unwrap();
    assert_eq!(
        (updates.audio, updates.controls, updates.queue.len()),
        (0, 0, 0)
    );
    let (waiter, mut response) = oneshot::channel();
    assert_eq!(updates.read(waiter).unwrap(), Delivery::TerminalReady);
    assert_eq!(response.try_recv().unwrap(), VoiceUpdate::Stopped);
    let rejected = audio();
    match updates.publish(rejected.clone()).unwrap_err() {
        PublishRefusal::Closed(value) => assert_eq!(value, rejected),
        PublishRefusal::Capacity(_) | PublishRefusal::Terminal(_) => {
            panic!("expected closed refusal")
        }
    }
}

#[test]
fn a_second_wait_is_refused_and_cancelled_terminal_is_not_consumed() {
    let mut updates = Updates::default();
    let (first, first_response) = oneshot::channel();
    assert_eq!(updates.read(first).unwrap(), Delivery::Waiting);
    let (second, _second_response) = oneshot::channel();
    assert_eq!(updates.read(second), Err(ReadRefusal::Waiting));
    updates
        .close(Closed::Failed(VoiceFailure::TimedOut))
        .unwrap();
    drop(first_response);
    assert_eq!(updates.deliver(), Delivery::Waiting);
    assert_eq!(updates.close(Closed::Stopped), Err(ReadRefusal::Gone));
    let (next, mut response) = oneshot::channel();
    assert_eq!(updates.read(next).unwrap(), Delivery::TerminalReady);
    assert_eq!(
        response.try_recv().unwrap(),
        VoiceUpdate::Failed {
            reason: VoiceFailure::TimedOut
        }
    );
    let (late, _response) = oneshot::channel();
    assert_eq!(updates.read(late), Err(ReadRefusal::Gone));
    let mut fresh = Updates::default();
    match fresh.publish(VoiceUpdate::Stopped).unwrap_err() {
        PublishRefusal::Terminal(value) => assert_eq!(value, VoiceUpdate::Stopped),
        PublishRefusal::Capacity(_) | PublishRefusal::Closed(_) => {
            panic!("expected terminal refusal")
        }
    }
}
