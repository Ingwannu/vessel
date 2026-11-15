//! Worktree base admission after checkout-git.ts:1340–1378. This operates on
//! the pinned fact from the same resolved-root snapshot, never another lookup.
//!
//! [Decision Log]
//! - Purpose: a display alias must not silently redirect a recorded base stream.
//! - Constraint: the existing resolved base also contains a project fallback;
//!   treating that fallback as a pinned ref would reject valid project choices.
//! - Choice: admit against the additional fact on the existing resolved root,
//!   then use the original comparison reader. No new root resolver or I/O.
//! - Alternative: normalize both refs to labels. That conflates local and remote
//!   streams; the reference only normalizes the stored ref for a display alias.
//! - Effect: refusal retains both typed identities, with neither called correct.
use vessel_proto::workspace::{GitComparison, RefName, WorktreeBase};

/// A comparison may not silently choose a different recorded branch stream.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(super) enum BaseRefusal {
    #[error("the requested comparison base differs from the workspace base")]
    Mismatch {
        stored: RefName,
        requested: WorktreeBase,
    },
}

pub(super) fn admit(
    pinned: Option<&RefName>,
    comparison: GitComparison,
) -> Result<GitComparison, BaseRefusal> {
    match comparison {
        GitComparison::Committed { base } => {
            choose(pinned, base).map(|base| GitComparison::Committed { base })
        }
        GitComparison::All { base } => choose(pinned, base).map(|base| GitComparison::All { base }),
        other @ (GitComparison::Uncommitted | GitComparison::Commit { .. }) => Ok(other),
    }
}

fn choose(
    pinned: Option<&RefName>,
    requested: Option<WorktreeBase>,
) -> Result<Option<WorktreeBase>, BaseRefusal> {
    let Some(stored) = pinned else {
        return Ok(requested);
    };
    match requested {
        None => {}
        Some(WorktreeBase::Ref { name })
            if name == *stored || name.as_str() == display_name(stored) => {}
        Some(requested) => {
            return Err(BaseRefusal::Mismatch {
                stored: stored.clone(),
                requested,
            });
        }
    }
    Ok(Some(WorktreeBase::Ref {
        name: stored.clone(),
    }))
}

fn display_name(name: &RefName) -> &str {
    let text = name.as_str();
    if let Some(local) = text.strip_prefix("refs/heads/") {
        return local;
    }
    if let Some(remote) = text.strip_prefix("refs/remotes/") {
        return remote
            .split_once('/')
            .map_or(remote, |(_remote, branch)| branch);
    }
    text.strip_prefix("origin/").unwrap_or(text)
}

#[cfg(test)]
mod tests;
