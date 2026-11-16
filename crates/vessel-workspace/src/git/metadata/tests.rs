//! Reference rule fixtures, namespace boundaries and typed malformed-name failure.
use super::*;
use vessel_proto::workspace::{TextKind, TextProblem};

fn named(namespace: Namespace, reference: &str) -> Effect {
    Effect::Ref {
        namespace,
        reference: Reference::Named(reference.parse().unwrap()),
    }
}

#[test]
fn reference_events_preserve_owner_common_and_ref_scope() {
    let fixtures = [
        (Scope::Worktree, "index", Effect::Owner(BaseRefresh::Keep)),
        (Scope::Worktree, "HEAD", Effect::Owner(BaseRefresh::Refresh)),
        (
            Scope::Worktree,
            "config.worktree",
            Effect::Owner(BaseRefresh::Refresh),
        ),
        (
            Scope::Worktree,
            "paseo/worktree.json",
            Effect::Owner(BaseRefresh::Refresh),
        ),
        (Scope::Common, "index", Effect::Owner(BaseRefresh::Keep)),
        (Scope::Common, "HEAD", Effect::Owner(BaseRefresh::Refresh)),
        (
            Scope::Common,
            "refs/heads/feature",
            named(Namespace::Local, "feature"),
        ),
        (
            Scope::Common,
            "refs/remotes/origin/main",
            named(Namespace::Remote, "origin/main"),
        ),
        (
            Scope::Common,
            "packed-refs",
            Effect::All(BaseRefresh::Refresh),
        ),
        (
            Scope::Common,
            "reftable/tables.list",
            Effect::All(BaseRefresh::Refresh),
        ),
        (Scope::Common, "config", Effect::All(BaseRefresh::Refresh)),
    ];
    for (scope, path, expected) in fixtures {
        assert_eq!(classify(scope, path), Ok(expected), "{scope:?}: {path}");
    }
    for (scope, path) in [
        (Scope::Worktree, "COMMIT_EDITMSG"),
        (Scope::Worktree, "ORIG_HEAD"),
        (Scope::Worktree, "config"),
        (Scope::Worktree, "refs/heads/main"),
        (Scope::Common, "config.worktree"),
        (Scope::Common, "paseo/worktree.json"),
        (Scope::Common, "worktrees/new/index"),
        (Scope::Common, "hooks/pre-commit"),
        (Scope::Common, "logs/refs/heads/main"),
        (Scope::Common, "objects/ab/cdef"),
        (Scope::Common, "refs/tags/v1"),
        (Scope::Common, "FETCH_HEAD"),
        (Scope::Common, "HEAD/child"),
        (Scope::Common, "refs/headsish/topic"),
    ] {
        assert_eq!(
            classify(scope, path),
            Ok(Effect::Ignore),
            "{scope:?}: {path}"
        );
    }
}

#[test]
fn lock_precedence_and_pruning_come_from_the_same_ordered_inventory() {
    for scope in [Scope::Common, Scope::Worktree] {
        for path in [
            "index.lock",
            "HEAD.lock",
            "packed-refs.lock",
            "refs/heads/topic.lock",
            "refs/remotes/origin/topic.lock",
            "reftable/tables.list.lock",
        ] {
            assert_eq!(
                classify(scope, path),
                Ok(Effect::Ignore),
                "{scope:?}: {path}"
            );
        }
    }
    assert_eq!(
        pruned_directories(Scope::Common).collect::<Vec<_>>(),
        ["hooks", "logs", "objects"]
    );
    assert!(pruned_directories(Scope::Worktree).next().is_none());
    for directory in pruned_directories(Scope::Common) {
        assert_eq!(classify(Scope::Common, directory), Ok(Effect::Ignore));
        assert_eq!(
            classify(Scope::Common, &format!("{directory}/child")),
            Ok(Effect::Ignore)
        );
    }
}

#[test]
fn namespace_directory_events_never_invent_a_named_branch() {
    for (prefix, namespace) in [
        ("refs/heads", Namespace::Local),
        ("refs/remotes", Namespace::Remote),
    ] {
        for path in [prefix.to_owned(), format!("{prefix}/")] {
            assert_eq!(
                classify(Scope::Common, &path),
                Ok(Effect::Ref {
                    namespace,
                    reference: Reference::Namespace,
                })
            );
        }
    }
    assert_eq!(
        classify(Scope::Common, "reftable"),
        Ok(Effect::All(BaseRefresh::Refresh))
    );
}

#[test]
fn native_separator_normalization_keeps_validated_unicode_and_remote_identity() {
    assert_eq!(
        classify(Scope::Common, "refs\\heads\\기능\\새기능"),
        Ok(named(Namespace::Local, "기능/새기능"))
    );
    assert_eq!(
        classify(Scope::Common, "refs\\remotes\\upstream\\topic"),
        Ok(named(Namespace::Remote, "upstream/topic"))
    );
    assert_eq!(
        classify(Scope::Worktree, "paseo\\worktree.json"),
        Ok(Effect::Owner(BaseRefresh::Refresh))
    );
    assert_eq!(
        classify(Scope::Common, "refs\\heads\\topic.lock"),
        Ok(Effect::Ignore)
    );
}

#[test]
fn malformed_ref_paths_keep_the_original_typed_error_for_structural_recovery() {
    assert_eq!(
        classify(Scope::Common, "refs/heads/bad?name"),
        Err(InvalidText {
            kind: TextKind::Ref,
            problem: TextProblem::Character { character: '?' },
        })
    );
    assert_eq!(
        classify(Scope::Common, "refs/remotes/origin/bad name"),
        Err(InvalidText {
            kind: TextKind::Ref,
            problem: TextProblem::Character { character: ' ' },
        })
    );
}
