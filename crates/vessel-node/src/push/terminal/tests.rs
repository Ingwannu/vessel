//! Pure source watermarks; no protocol number, process or transport is invented.
use super::*;
use vessel_proto::terminals::{TerminalInfo, TerminalKind, TerminalPlace};
fn info() -> TerminalInfo {
    let mut info = TerminalInfo::new(
        TerminalId::random(),
        TerminalPlace::Session {
            session: vessel_proto::SessionId::random(),
        },
        std::path::PathBuf::from("/fixture"),
        TerminalKind::Unknown,
        chrono::DateTime::UNIX_EPOCH,
    );
    let state = info.attention.as_mut().unwrap();
    state.revision = TerminalAttentionRevision::new(1);
    state.activity = TerminalActivity::Attention {
        reason: TerminalAttentionReason::NeedsInput,
    };
    info
}
#[test]
fn exact_revision_once_and_regressions_never_replay_attention() {
    let mut row = info();
    let state = row.attention.unwrap();
    let mut seen = HashMap::new();
    assert_eq!(fresh(&mut seen, vec![row.clone()]), vec![state]);
    assert!(fresh(&mut seen, vec![row.clone()]).is_empty());
    row.attention.as_mut().unwrap().revision = TerminalAttentionRevision::new(0);
    assert!(fresh(&mut seen, vec![row.clone()]).is_empty());
    row.attention = Some(state);
    assert!(fresh(&mut seen, vec![row.clone()]).is_empty());
    row.attention.as_mut().unwrap().revision = state.revision.next().unwrap();
    assert_eq!(fresh(&mut seen, vec![row]).len(), 1);
}
#[test]
fn baseline_unknown_clear_failed_removed_and_foreign_states_cannot_push() {
    let mut row = info();
    let state = row.attention.unwrap();
    let mut seen = HashMap::from([(row.id, state.revision)]);
    assert!(fresh(&mut seen, vec![row.clone()]).is_empty());
    let mut foreign = state;
    foreign.terminal = TerminalId::random();
    assert!(!current(&row, foreign));
    row.attention.as_mut().unwrap().revision = state.revision.next().unwrap();
    row.attention.as_mut().unwrap().activity = TerminalActivity::Idle;
    assert!(fresh(&mut seen, vec![row.clone()]).is_empty());
    assert!(!current(&row, state));
    row.attention.as_mut().unwrap().revision = row.attention.unwrap().revision.next().unwrap();
    row.attention.as_mut().unwrap().activity = TerminalActivity::Unknown;
    assert!(fresh(&mut seen, vec![row.clone()]).is_empty());
    row.status = SessionStatus::failed("fixture");
    row.attention.as_mut().unwrap().revision = row.attention.unwrap().revision.next().unwrap();
    row.attention.as_mut().unwrap().activity = state.activity;
    assert!(fresh(&mut seen, vec![row]).is_empty());
    assert!(fresh(&mut seen, Vec::new()).is_empty());
    assert!(seen.is_empty());
}
