use super::*;
#[tokio::test]
async fn mute_and_immediate_unmute_cannot_reopen_old_or_muted_capture() {
    let stopped = Arc::new(Fence::default());
    let gate = Gate::new(Arc::clone(&stopped));
    let old = gate.snapshot();
    let interval = old.interval().unwrap();
    gate.set_mute(Mute::Muted).unwrap();
    let muted = gate.snapshot();
    gate.set_mute(Mute::Open).unwrap();
    assert!(!old.live());
    assert!(!muted.live());
    assert!(gate.snapshot().live());
    interval.closed().await;
    stopped.close();
    assert!(!gate.snapshot().live());
    assert_eq!(gate.set_mute(Mute::Open), Err(MuteError::Stopped));
}
#[test]
fn a_callback_suppresses_a_frame_instead_of_waiting_on_a_control_transition() {
    let gate = Gate::new(Arc::new(Fence::default()));
    let _held = gate.current.lock().unwrap();
    assert!(!gate.snapshot().live());
}
