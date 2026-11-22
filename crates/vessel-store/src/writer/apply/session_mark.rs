//! A person's Archive and the exact Keep release share the session savepoint.

use rusqlite::Transaction;
use vessel_proto::KeptMarks;

use super::marks_answer;
use crate::automation::binding;
use crate::registry::RegistryChange;
use crate::writer::{Answer, Done, Effects, SessionMark};
use crate::{SessionCommit, marks};

pub(super) fn apply(
    transaction: &mut Transaction<'_>,
    effects: &mut Effects,
    write: SessionMark,
    done: Done<SessionCommit<KeptMarks>>,
) -> Answer {
    let mut wake = false;
    let answer = marks_answer(transaction, effects, write.session, done, |connection| {
        let kept = marks::mark(
            connection,
            write.session,
            write.mark,
            write.at,
            write.residence,
        )?;
        if write.release_keep {
            wake = binding::request_keep_release(connection, write.session)?;
        }
        Ok(kept)
    });
    if wake && matches!(answer, Answer::Marked(_, Ok(_))) {
        effects.registry = RegistryChange::Changed;
    }
    answer
}
