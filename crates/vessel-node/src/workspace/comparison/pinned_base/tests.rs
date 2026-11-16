//! Qualified ref identity follows the reference, not normalized-label equality.
use super::*;

fn base(name: &str) -> WorktreeBase {
    WorktreeBase::Ref {
        name: name.parse().unwrap(),
    }
}

#[test]
fn a_project_without_a_pinned_base_keeps_the_complete_requested_comparison() {
    for comparison in [
        GitComparison::All { base: None },
        GitComparison::Committed {
            base: Some(base("refs/heads/other")),
        },
        GitComparison::Uncommitted,
        GitComparison::Commit {
            id: "a".repeat(40).parse().unwrap(),
        },
    ] {
        assert_eq!(admit(None, comparison.clone()), Ok(comparison));
    }
}

#[test]
fn an_exact_ref_or_its_display_alias_keeps_the_recorded_stream_for_both_modes() {
    for (stored, alias) in [
        ("refs/heads/main", "main"),
        ("refs/remotes/upstream/main", "main"),
        ("refs/remotes/team/upstream/main", "upstream/main"),
        ("origin/feature/name", "feature/name"),
    ] {
        let pinned: RefName = stored.parse().unwrap();
        for requested in [None, Some(base(stored)), Some(base(alias))] {
            assert_eq!(
                admit(
                    Some(&pinned),
                    GitComparison::All {
                        base: requested.clone()
                    }
                ),
                Ok(GitComparison::All {
                    base: Some(base(stored))
                })
            );
            assert_eq!(
                admit(Some(&pinned), GitComparison::Committed { base: requested }),
                Ok(GitComparison::Committed {
                    base: Some(base(stored))
                })
            );
        }
    }
}

#[test]
fn another_qualified_stream_is_not_authorized_by_the_same_display_name() {
    let pinned: RefName = "refs/remotes/upstream/main".parse().unwrap();
    for requested in [
        base("refs/heads/main"),
        base("refs/remotes/origin/main"),
        base("other"),
        WorktreeBase::Commit {
            id: "a".repeat(40).parse().unwrap(),
        },
    ] {
        assert_eq!(
            admit(
                Some(&pinned),
                GitComparison::All {
                    base: Some(requested.clone())
                }
            ),
            Err(BaseRefusal::Mismatch {
                stored: pinned.clone(),
                requested
            })
        );
    }
}

#[test]
fn immutable_commits_and_uncommitted_reads_do_not_apply_branch_base_admission() {
    let pinned: RefName = "refs/heads/main".parse().unwrap();
    for comparison in [
        GitComparison::Uncommitted,
        GitComparison::Commit {
            id: "a".repeat(40).parse().unwrap(),
        },
    ] {
        assert_eq!(admit(Some(&pinned), comparison.clone()), Ok(comparison));
    }
}
