//! Hold typed label answers until the existing writer's outer batch commits.
use super::{Answer, Effects, isolated};
use crate::{
    StoreError,
    registry::RegistryChange,
    workspace_labels::{self, Applied, WorkspaceLabelCommit, Write},
};
use rusqlite::Transaction;

pub(super) fn apply(
    transaction: &mut Transaction<'_>,
    write: Write,
    effects: &mut Effects,
) -> Answer {
    match write {
        Write::Assign { request, done } => Answer::LabelAssigned(
            done,
            committed(transaction, effects, |connection| {
                workspace_labels::assign(connection, &request)
            }),
        ),
        Write::Update { request, done } => Answer::LabelUpdated(
            done,
            committed(transaction, effects, |connection| {
                workspace_labels::update(connection, &request)
            }),
        ),
        Write::Delete { request, done } => Answer::LabelDeleted(
            done,
            committed(transaction, effects, |connection| {
                workspace_labels::delete(connection, &request)
            }),
        ),
    }
}
fn committed<T>(
    transaction: &mut Transaction<'_>,
    effects: &mut Effects,
    operation: impl FnOnce(&rusqlite::Connection) -> Result<Applied<T>, StoreError>,
) -> Result<WorkspaceLabelCommit<T>, StoreError> {
    let applied = isolated(transaction, operation)?;
    // No marker escapes a rolled-back savepoint. The existing writer only
    // publishes this coalesced wake-up after the outer commit succeeds.
    if applied.registry == RegistryChange::Changed {
        effects.registry = RegistryChange::Changed;
    }
    Ok(applied.committed)
}
