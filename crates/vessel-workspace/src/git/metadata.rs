//! Decode native Git metadata event paths, after Paseo's ordered event rules.
//!
//! [Decision Log]
//! - Existing assets: validated `RefName` values and the Git owner's common-dir
//!   boundary. File observation has a different, finite lifecycle.
//! - Choice: one ordered inventory drives both routing and watcher pruning.
//!   Parse external names here; never turn event strings into Git arguments.
//! - Difference: a namespace directory event is explicit, and malformed names
//!   keep the existing typed parse error. A future observer must refresh
//!   structurally on either, rather than silently retain a stale snapshot.
//! - This decoder performs no IO, observation, invalidation or authorization.
use std::borrow::Cow;
use vessel_proto::workspace::{InvalidText, RefName};

/// Which authoritative Git metadata directory reported the relative path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// One linked worktree's administrative directory.
    Worktree,
    /// The common directory shared by the repository's worktrees.
    Common,
}
/// Base resolution is independent of who needs a new status snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaseRefresh {
    /// An index update leaves the base identity alone.
    Keep,
    /// HEAD/config/ref structure can change the comparison base.
    Refresh,
}
/// The namespace is retained instead of guessing from a branch's spelling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Namespace {
    /// A branch under `refs/heads`.
    Local,
    /// A tracking branch under `refs/remotes`, including its remote component.
    Remote,
}
/// Watchers can report a directory itself, without naming an individual ref.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reference {
    /// Reconcile this namespace; no branch identity was observed.
    Namespace,
    /// A validated relative ref name, e.g. topic or origin/topic.
    Named(RefName),
}
/// A typed refresh request, not proof that the event changed Git state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    /// Locks, irrelevant metadata and unknown paths do not trigger work.
    Ignore,
    /// Only the owner worktree needs a new snapshot.
    Owner(BaseRefresh),
    /// All workspaces of the authoritative project need a new snapshot.
    All(BaseRefresh),
    /// Reconcile a local or remote ref without losing namespace identity.
    Ref {
        /// Which ref namespace changed.
        namespace: Namespace,
        /// A name, or a directory-level reconciliation request.
        reference: Reference,
    },
}

/// Decode a path relative to the already-authorized metadata directory.
/// Backslash normalization matches the reference on every host. This accepts
/// no checkout/root authority and performs no path lookup or process launch.
///
/// # Errors
/// A matching ref with an invalid name retains the original `InvalidText`. The
/// observer must retain/report that failure and force a structural refresh;
/// it must not discard it as an ignored event or execute the raw name.
pub fn classify(scope: Scope, input: &str) -> Result<Effect, InvalidText> {
    let path = if input.contains('\\') {
        Cow::Owned(input.replace('\\', "/"))
    } else {
        Cow::Borrowed(input)
    };
    RULES
        .iter()
        .find_map(|rule| rule.apply(scope, &path))
        .unwrap_or(Ok(Effect::Ignore))
}

/// Directories a future watcher may prune, derived from the SAME rule inventory.
/// Paths are relative to the authoritative metadata directory, not filesystem
/// roots. No registration or traversal is performed by this iterator.
pub fn pruned_directories(scope: Scope) -> impl Iterator<Item = &'static str> {
    RULES.iter().filter_map(move |rule| rule.pruned(scope))
}

#[derive(Clone, Copy)]
enum Scopes {
    One(Scope),
    Both,
}
impl Scopes {
    fn contains(self, scope: Scope) -> bool {
        match self {
            Self::One(selected) => selected == scope,
            Self::Both => true,
        }
    }
}
#[derive(Clone, Copy)]
enum Pattern {
    Exact(&'static str),
    Prefix(&'static str),
    Suffix(&'static str),
}
impl Pattern {
    fn matches(self, path: &str) -> bool {
        match self {
            Self::Exact(wanted) => path == wanted,
            Self::Prefix(prefix) => {
                path == prefix.trim_end_matches('/') || path.starts_with(prefix)
            }
            Self::Suffix(suffix) => path.ends_with(suffix),
        }
    }
}
#[derive(Clone, Copy)]
enum Pruning {
    Keep,
    Directory,
}
#[derive(Clone, Copy)]
enum Rule {
    Ignore(Scopes, Pattern, Pruning),
    Owner(Scope, &'static str, BaseRefresh),
    All(Scope, Pattern, BaseRefresh),
    Ref(Scope, &'static str, Namespace),
}
impl Rule {
    fn apply(self, scope: Scope, path: &str) -> Option<Result<Effect, InvalidText>> {
        match self {
            Self::Ignore(scopes, pattern, _) => {
                (scopes.contains(scope) && pattern.matches(path)).then_some(Ok(Effect::Ignore))
            }
            Self::Owner(selected, exact, base) => {
                (scope == selected && path == exact).then_some(Ok(Effect::Owner(base)))
            }
            Self::All(selected, pattern, base) => {
                (scope == selected && pattern.matches(path)).then_some(Ok(Effect::All(base)))
            }
            Self::Ref(selected, prefix, namespace) => {
                if scope != selected {
                    return None;
                }
                let name = if path == prefix.trim_end_matches('/') {
                    ""
                } else {
                    path.strip_prefix(prefix)?
                };
                Some(if name.is_empty() {
                    Ok(Effect::Ref {
                        namespace,
                        reference: Reference::Namespace,
                    })
                } else {
                    name.parse().map(|name| Effect::Ref {
                        namespace,
                        reference: Reference::Named(name),
                    })
                })
            }
        }
    }
    fn pruned(self, scope: Scope) -> Option<&'static str> {
        match self {
            Self::Ignore(scopes, pattern, pruning) => {
                if !scopes.contains(scope) {
                    return None;
                }
                match (pattern, pruning) {
                    (Pattern::Prefix(prefix), Pruning::Directory) => {
                        Some(prefix.trim_end_matches('/'))
                    }
                    (Pattern::Exact(_) | Pattern::Suffix(_), Pruning::Directory)
                    | (
                        Pattern::Exact(_) | Pattern::Prefix(_) | Pattern::Suffix(_),
                        Pruning::Keep,
                    ) => None,
                }
            }
            Self::Owner(_, _, _) | Self::All(_, _, _) | Self::Ref(_, _, _) => None,
        }
    }
}

// Order matters: a ref lock is ignored before the broader namespace rule.
static RULES: [Rule; 16] = [
    Rule::Ignore(Scopes::Both, Pattern::Suffix(".lock"), Pruning::Keep),
    Rule::Owner(Scope::Worktree, "index", BaseRefresh::Keep),
    Rule::Owner(Scope::Worktree, "HEAD", BaseRefresh::Refresh),
    Rule::Owner(Scope::Worktree, "config.worktree", BaseRefresh::Refresh),
    Rule::Owner(Scope::Worktree, "paseo/worktree.json", BaseRefresh::Refresh),
    Rule::Owner(Scope::Common, "index", BaseRefresh::Keep),
    Rule::Owner(Scope::Common, "HEAD", BaseRefresh::Refresh),
    Rule::Ref(Scope::Common, "refs/heads/", Namespace::Local),
    Rule::Ref(Scope::Common, "refs/remotes/", Namespace::Remote),
    Rule::All(
        Scope::Common,
        Pattern::Exact("packed-refs"),
        BaseRefresh::Refresh,
    ),
    Rule::All(
        Scope::Common,
        Pattern::Prefix("reftable/"),
        BaseRefresh::Refresh,
    ),
    Rule::All(
        Scope::Common,
        Pattern::Exact("config"),
        BaseRefresh::Refresh,
    ),
    Rule::Ignore(
        Scopes::One(Scope::Common),
        Pattern::Prefix("worktrees/"),
        Pruning::Keep,
    ),
    Rule::Ignore(
        Scopes::One(Scope::Common),
        Pattern::Prefix("hooks/"),
        Pruning::Directory,
    ),
    Rule::Ignore(
        Scopes::One(Scope::Common),
        Pattern::Prefix("logs/"),
        Pruning::Directory,
    ),
    Rule::Ignore(
        Scopes::One(Scope::Common),
        Pattern::Prefix("objects/"),
        Pruning::Directory,
    ),
];

#[cfg(test)]
mod tests;
