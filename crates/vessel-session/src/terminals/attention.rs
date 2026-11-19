//! Pure semantic activity reducer (ADR 0089); output bytes never enter here.
use chrono::{DateTime, Utc};
use vessel_proto::notifications::{
    TerminalActivity, TerminalActivitySignal, TerminalAttention, TerminalAttentionReason,
    TerminalAttentionRevision,
};

/// A semantic mutation refused without changing current attention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AttentionError {
    /// An ended process cannot begin new semantic work.
    #[error("the terminal has ended; open a new terminal to report activity")]
    Ended,
    /// A revision cannot advance without reusing an identity.
    #[error("terminal attention revision exhausted; reopen the terminal")]
    Exhausted,
    /// A stale clear or producer report cannot change newer attention.
    #[error("terminal attention changed; refresh before clearing it")]
    Stale,
}

pub(super) fn signal(
    current: TerminalAttention,
    signal: TerminalActivitySignal,
    now: DateTime<Utc>,
) -> Result<TerminalAttention, AttentionError> {
    let activity = next_activity(current.activity, signal);
    replace(current, activity, now)
}

pub(super) fn clear(
    current: TerminalAttention,
    revision: TerminalAttentionRevision,
    now: DateTime<Utc>,
) -> Result<TerminalAttention, AttentionError> {
    if current.revision != revision {
        return Err(AttentionError::Stale);
    }
    match current.activity {
        TerminalActivity::Attention { .. } => replace(current, TerminalActivity::Idle, now),
        TerminalActivity::Untracked
        | TerminalActivity::Unknown
        | TerminalActivity::Idle
        | TerminalActivity::Working => Ok(current),
    }
}

fn replace(
    current: TerminalAttention,
    activity: TerminalActivity,
    now: DateTime<Utc>,
) -> Result<TerminalAttention, AttentionError> {
    if current.activity == activity {
        return Ok(current);
    }
    let revision = current.revision.next().ok_or(AttentionError::Exhausted)?;
    Ok(TerminalAttention {
        activity,
        revision,
        changed_at: now,
        ..current
    })
}

fn next_activity(current: TerminalActivity, signal: TerminalActivitySignal) -> TerminalActivity {
    match signal {
        TerminalActivitySignal::Working => TerminalActivity::Working,
        TerminalActivitySignal::NeedsInput => TerminalActivity::Attention {
            reason: TerminalAttentionReason::NeedsInput,
        },
        TerminalActivitySignal::Clear => TerminalActivity::Unknown,
        TerminalActivitySignal::Interrupt => {
            if current == TerminalActivity::Working {
                TerminalActivity::Unknown
            } else {
                current
            }
        }
        TerminalActivitySignal::Idle => match current {
            TerminalActivity::Working => TerminalActivity::Attention {
                reason: TerminalAttentionReason::Finished,
            },
            TerminalActivity::Attention {
                reason: TerminalAttentionReason::Finished,
            } => current,
            TerminalActivity::Untracked
            | TerminalActivity::Unknown
            | TerminalActivity::Idle
            | TerminalActivity::Attention {
                reason: TerminalAttentionReason::NeedsInput | TerminalAttentionReason::Unknown,
            } => TerminalActivity::Idle,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn initial() -> TerminalAttention {
        TerminalAttention {
            terminal: vessel_proto::TerminalId::random(),
            revision: TerminalAttentionRevision::default(),
            activity: TerminalActivity::Unknown,
            changed_at: DateTime::UNIX_EPOCH,
        }
    }
    #[test]
    fn known_work_finishes_but_plain_shell_unknown_is_never_inferred_as_work() {
        let unknown = initial();
        let idle = signal(unknown, TerminalActivitySignal::Idle, DateTime::UNIX_EPOCH).unwrap();
        assert_eq!(idle.activity, TerminalActivity::Idle);
        let working = signal(idle, TerminalActivitySignal::Working, DateTime::UNIX_EPOCH).unwrap();
        let finished = signal(working, TerminalActivitySignal::Idle, DateTime::UNIX_EPOCH).unwrap();
        assert_eq!(
            finished.activity,
            TerminalActivity::Attention {
                reason: TerminalAttentionReason::Finished
            }
        );
        assert_eq!(
            signal(finished, TerminalActivitySignal::Idle, DateTime::UNIX_EPOCH).unwrap(),
            finished
        );
        assert_eq!(
            clear(finished, finished.revision, DateTime::UNIX_EPOCH)
                .unwrap()
                .activity,
            TerminalActivity::Idle
        );
    }
    #[test]
    fn clear_and_interrupt_do_not_create_finished_attention() {
        let current = initial();
        let working = signal(
            current,
            TerminalActivitySignal::Working,
            DateTime::UNIX_EPOCH,
        )
        .unwrap();
        assert_eq!(
            signal(
                working,
                TerminalActivitySignal::Interrupt,
                DateTime::UNIX_EPOCH
            )
            .unwrap()
            .activity,
            TerminalActivity::Unknown
        );
        assert_eq!(
            signal(working, TerminalActivitySignal::Clear, DateTime::UNIX_EPOCH)
                .unwrap()
                .activity,
            TerminalActivity::Unknown
        );
        let waiting = signal(
            working,
            TerminalActivitySignal::NeedsInput,
            DateTime::UNIX_EPOCH,
        )
        .unwrap();
        assert_eq!(
            signal(
                waiting,
                TerminalActivitySignal::Interrupt,
                DateTime::UNIX_EPOCH
            )
            .unwrap(),
            waiting
        );
    }
    #[test]
    fn an_old_clear_cannot_consume_new_attention_and_revision_exhaustion_does_not_wrap() {
        let current = initial();
        let waiting = signal(
            current,
            TerminalActivitySignal::NeedsInput,
            DateTime::UNIX_EPOCH,
        )
        .unwrap();
        assert_eq!(
            clear(waiting, current.revision, DateTime::UNIX_EPOCH),
            Err(AttentionError::Stale)
        );
        let exhausted = TerminalAttention {
            revision: TerminalAttentionRevision::new(u64::MAX),
            ..waiting
        };
        assert_eq!(
            signal(
                exhausted,
                TerminalActivitySignal::Working,
                DateTime::UNIX_EPOCH
            ),
            Err(AttentionError::Exhausted)
        );
    }
}

#[cfg(test)]
mod untracked_tests {
    use super::*;
    #[test]
    fn untracked_is_never_finished_without_explicit_work_and_clear_loses_tracking() {
        let initial = TerminalAttention {
            terminal: vessel_proto::TerminalId::random(),
            revision: TerminalAttentionRevision::default(),
            activity: TerminalActivity::Untracked,
            changed_at: DateTime::UNIX_EPOCH,
        };
        assert_eq!(
            clear(initial, initial.revision, DateTime::UNIX_EPOCH).unwrap(),
            initial
        );
        let idle = signal(initial, TerminalActivitySignal::Idle, DateTime::UNIX_EPOCH).unwrap();
        assert_eq!(idle.activity, TerminalActivity::Idle);
        let missing = signal(initial, TerminalActivitySignal::Clear, DateTime::UNIX_EPOCH).unwrap();
        assert_eq!(missing.activity, TerminalActivity::Unknown);
        assert_ne!(missing.revision, initial.revision);
    }
}
