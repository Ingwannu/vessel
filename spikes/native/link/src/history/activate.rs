//! Reactivation preserves the existing session id and the selected account.

use super::{CREATE_DEADLINE, READ_DEADLINE, create_failure, read_failure};
use crate::connections::SelectedConnection;
use vessel_app_view::history::{Entry, Problem, Scope};
use vessel_proto::{
    GetSession, HarnessKind, HarnessRef, ProfileName, SessionId, SessionInfo,
    conversation::ResumeSession,
};

pub(super) async fn existing(
    client: &SelectedConnection,
    scope: &Scope,
    entry: &Entry,
    session: SessionId,
) -> Result<Box<SessionInfo>, Problem> {
    let info = tokio::time::timeout(READ_DEADLINE, client.call(GetSession { session }))
        .await
        .map_or(Err(Problem::Timeout), |reply| reply.map_err(read_failure))?;
    validate(&info, scope, entry, session)?;
    if info.status.is_live() {
        return Ok(Box::new(info));
    }
    // session.resume reopens the same row and removes its archive mark.
    // A lost result is ambiguous, so the model requires a history refresh;
    // it must never fall back to creating another imported session.
    let resumed = tokio::time::timeout(CREATE_DEADLINE, client.call(ResumeSession { session }))
        .await
        .map_or(Err(Problem::Uncertain), |reply| {
            reply.map_err(create_failure)
        })?;
    validate(&resumed, scope, entry, session)?;
    Ok(Box::new(resumed))
}

fn validate(
    info: &SessionInfo,
    scope: &Scope,
    entry: &Entry,
    expected: SessionId,
) -> Result<(), Problem> {
    if info.id != expected
        || info.spec.harness != scope.harness
        || info.harness_session.as_ref() != Some(&entry.id)
        || !same_account(
            &scope.harness,
            scope.profile.as_ref(),
            info.profile.as_ref(),
        )
    {
        return Err(Problem::InvalidReply);
    }
    Ok(())
}

fn same_account(
    harness: &HarnessRef,
    expected: Option<&ProfileName>,
    actual: Option<&ProfileName>,
) -> bool {
    if expected == actual {
        return true;
    }
    let implicit = |name: Option<&ProfileName>| {
        name.is_none_or(|name| match harness {
            HarnessRef::Known(kind) => name
                .as_str()
                .parse::<HarnessKind>()
                .is_ok_and(|found| found == *kind),
            HarnessRef::Added(agent) => name == agent.profile(),
        })
    };
    implicit(expected) && implicit(actual)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canonical_default_and_explicit_aliases_match_but_other_accounts_do_not() {
        let harness = HarnessKind::ClaudeCode.into();
        let canonical = "claude".parse().unwrap();
        let alias = "claude-code".parse().unwrap();
        let other = "work-account".parse().unwrap();
        assert!(same_account(&harness, None, Some(&canonical)));
        assert!(same_account(&harness, Some(&alias), Some(&canonical)));
        assert!(!same_account(&harness, None, Some(&other)));
        assert!(!same_account(&harness, Some(&other), Some(&canonical)));
    }
}

/// A mismatched write receipt is uncertain: creation may already have committed.
pub(super) fn created(info: &SessionInfo, scope: &Scope, entry: &Entry) -> Result<(), Problem> {
    if info.spec.harness != scope.harness
        || info.spec.cwd.as_ref() != Some(&entry.cwd)
        || !matches!(&info.spec.mode, vessel_proto::SessionMode::Structured {resume:Some(id),..} if id == &entry.id)
        || !same_account(
            &scope.harness,
            scope.profile.as_ref(),
            info.profile.as_ref(),
        )
    {
        return Err(Problem::Uncertain);
    }
    Ok(())
}
