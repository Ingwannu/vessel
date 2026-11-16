//! Cases from checkout-diff-cache.test.ts, with explicit bounded admission and
//! deterministic monotonic instants. No sleeps, subprocesses or fake clock atomics.

use super::*;

use vessel_proto::ProjectId;
use vessel_proto::workspace::GitWhitespace;

struct Fixture {
    _temp: tempfile::TempDir,
    root: Root,
    project: ProjectId,
}

impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = Root::open(temp.path()).unwrap();
        Self {
            _temp: temp,
            root,
            project: ProjectId::random(),
        }
    }

    fn key(&self, comparison: GitComparison) -> Key {
        Key::new(
            self.project,
            self.root.clone(),
            CompareGit {
                root: WorkspaceRoot::Project {
                    project: self.project,
                },
                comparison,
                whitespace: GitWhitespace::Show,
                reason: vessel_proto::workspace::GitReadReason::View,
            },
            None,
            Subject::Summary,
        )
    }
}

fn start<V>(decision: &Decision<V>) -> BuildId {
    assert_eq!(decision.cancel, None);
    let Read::Start(id) = &decision.read else {
        panic!("expected a new build");
    };
    *id
}

#[test]
fn forced_readers_wait_for_one_fresh_build_after_the_old_owned_job_finishes() {
    let fixture = Fixture::new();
    let key = fixture.key(GitComparison::Uncommitted);
    let now = Instant::now();
    let mut cache = Cache::default();
    let old = start(&cache.read(key.clone(), ReadReason::View, now).unwrap());
    let first = cache
        .read(key.clone(), ReadReason::Force(ForceReason::Mutation), now)
        .unwrap();
    assert!(matches!(first.read, Read::After(id) if id == old));
    assert_eq!(first.cancel, Some(old));
    let another = cache
        .read(key.clone(), ReadReason::Force(ForceReason::Mutation), now)
        .unwrap();
    assert!(matches!(another.read, Read::After(id) if id == old));
    assert_eq!(another.cancel, None);
    assert!(matches!(
        cache.finish(old, ResultValue::Value(Arc::new("old")), now),
        Completion::Retired
    ));
    let fresh = start(&cache.read(key.clone(), ReadReason::View, now).unwrap());
    assert!(
        matches!(cache.read(key.clone(), ReadReason::View, now).unwrap().read, Read::Join(id) if id == fresh)
    );
    let value = Arc::new("fresh");
    assert!(matches!(
        cache.finish(fresh, ResultValue::Value(Arc::clone(&value)), now),
        Completion::Published
    ));
    let Read::Hit(hit) = cache.read(key, ReadReason::View, now).unwrap().read else {
        panic!();
    };
    assert!(Arc::ptr_eq(&hit, &value));
}

#[test]
fn invalidated_reads_cannot_refill_the_cache_and_duplicate_completions_are_inert() {
    let fixture = Fixture::new();
    let key = fixture.key(GitComparison::All { base: None });
    let mut cache = Cache::default();
    let now = Instant::now();
    let id = start(&cache.read(key.clone(), ReadReason::View, now).unwrap());
    assert_eq!(
        cache.invalidate(key.query.root, key.project, Change::Working),
        [id]
    );
    assert!(
        cache
            .invalidate(key.query.root, key.project, Change::Working)
            .is_empty()
    );
    assert!(
        matches!(cache.read(key.clone(), ReadReason::View, now).unwrap().read, Read::After(waiting) if waiting == id)
    );
    assert!(matches!(
        cache.finish(id, ResultValue::Value(Arc::new("snapshot")), now),
        Completion::Retired
    ));
    assert!(matches!(
        cache.finish(id, ResultValue::Value(Arc::new("duplicate")), now),
        Completion::Unknown
    ));
    assert_ne!(start(&cache.read(key, ReadReason::View, now).unwrap()), id);
}

#[test]
fn active_work_survives_completed_eviction_and_a_failed_job_can_retry() {
    let fixture = Fixture::new();
    let mut cache = Cache::default();
    let now = Instant::now();
    let active = fixture.key(GitComparison::Uncommitted);
    let id = start(&cache.read(active.clone(), ReadReason::View, now).unwrap());
    for _ in 0..70 {
        let mut key = fixture.key(GitComparison::Uncommitted);
        key.query.root = WorkspaceRoot::Project {
            project: ProjectId::random(),
        };
        let completed = start(&cache.read(key, ReadReason::View, now).unwrap());
        assert!(matches!(
            cache.finish(completed, ResultValue::Value(Arc::new("completed")), now),
            Completion::Published
        ));
    }
    assert_eq!(cache.completed.len(), MAX_COMPLETED);
    assert!(
        matches!(cache.read(active.clone(), ReadReason::View, now).unwrap().read, Read::Join(found) if found == id)
    );
    assert!(matches!(
        cache.finish(id, ResultValue::Failed, now),
        Completion::Failed
    ));
    assert_ne!(
        start(&cache.read(active, ReadReason::View, now).unwrap()),
        id
    );
}

#[test]
fn working_and_history_changes_invalidate_all_for_each_base_but_not_other_roots() {
    for change in [Change::Working, Change::History] {
        let fixture = Fixture::new();
        let mut cache = Cache::default();
        let now = Instant::now();
        let mut keys = Vec::new();
        for base in ["main", "next"] {
            let key = fixture.key(GitComparison::All {
                base: Some(WorktreeBase::Ref {
                    name: base.parse().unwrap(),
                }),
            });
            let id = start(&cache.read(key.clone(), ReadReason::View, now).unwrap());
            assert!(matches!(
                cache.finish(id, ResultValue::Value(Arc::new(base)), now),
                Completion::Published
            ));
            keys.push(key);
        }
        let mut other = fixture.key(GitComparison::Uncommitted);
        other.project = ProjectId::random();
        other.query.root = WorkspaceRoot::Project {
            project: other.project,
        };
        let id = start(&cache.read(other.clone(), ReadReason::View, now).unwrap());
        assert!(matches!(
            cache.finish(id, ResultValue::Value(Arc::new("other")), now),
            Completion::Published
        ));
        assert!(
            cache
                .invalidate(
                    WorkspaceRoot::Project {
                        project: fixture.project
                    },
                    fixture.project,
                    change
                )
                .is_empty()
        );
        for key in keys {
            let _id = start(&cache.read(key, ReadReason::View, now).unwrap());
        }
        assert!(matches!(
            cache.read(other, ReadReason::View, now).unwrap().read,
            Read::Hit(_)
        ));
    }
}

#[test]
fn freshness_and_two_second_suppression_match_the_reference_boundaries() {
    let fixture = Fixture::new();
    let key = fixture.key(GitComparison::Uncommitted);
    let mut cache = Cache::default();
    let now = Instant::now();
    let id = start(&cache.read(key.clone(), ReadReason::View, now).unwrap());
    assert!(matches!(
        cache.finish(id, ResultValue::Value(Arc::new("cached")), now),
        Completion::Published
    ));
    let boundary = now.checked_add(FRESH_FOR).unwrap();
    assert!(matches!(
        cache
            .read(key.clone(), ReadReason::View, boundary)
            .unwrap()
            .read,
        Read::Hit(_)
    ));
    let expired = boundary.checked_add(Duration::from_millis(1)).unwrap();
    let refreshing = start(&cache.read(key.clone(), ReadReason::View, expired).unwrap());
    let before_gap = expired
        .checked_add(MIN_READ_GAP.checked_sub(Duration::from_millis(1)).unwrap())
        .unwrap();
    assert!(matches!(
        cache
            .read(key.clone(), ReadReason::View, before_gap)
            .unwrap()
            .read,
        Read::Hit(_)
    ));
    let at_gap = expired.checked_add(MIN_READ_GAP).unwrap();
    assert!(
        matches!(cache.read(key, ReadReason::View, at_gap).unwrap().read, Read::Join(found) if found == refreshing)
    );
}

#[test]
fn key_normalization_uses_defaults_only_for_comparisons_that_need_a_base() {
    let fixture = Fixture::new();
    let first = fixture.key(GitComparison::Uncommitted);
    let default = Some(WorktreeBase::Ref {
        name: "main".parse().unwrap(),
    });
    assert_eq!(
        Key::new(
            fixture.project,
            fixture.root.clone(),
            first.query.clone(),
            default.clone(),
            Subject::Summary
        ),
        first
    );
    let mut committed = fixture.key(GitComparison::Committed { base: None });
    committed.query.comparison = GitComparison::Committed {
        base: default.clone(),
    };
    assert_eq!(
        Key::new(
            fixture.project,
            fixture.root.clone(),
            fixture.key(GitComparison::Committed { base: None }).query,
            default,
            Subject::Summary
        ),
        committed
    );
}

#[test]
fn reasons_are_not_query_identity_and_every_forced_reason_bypasses_freshness() {
    use vessel_proto::workspace::GitReadReason;
    let fixture = Fixture::new();
    let now = Instant::now();
    for reason in [
        GitReadReason::Refresh,
        GitReadReason::Mutation,
        GitReadReason::Recovery,
    ] {
        let key = fixture.key(GitComparison::Uncommitted);
        let mut query = key.query.clone();
        query.reason = reason;
        assert_eq!(
            Key::new(
                fixture.project,
                fixture.root.clone(),
                query,
                None,
                Subject::Summary
            ),
            key
        );
        let mut cache = Cache::default();
        let id = start(&cache.read(key.clone(), ReadReason::View, now).unwrap());
        let _done = cache.finish(id, ResultValue::Value(Arc::new("old")), now);
        assert_ne!(start(&cache.read(key, reason.into(), now).unwrap()), id);
    }
}

#[test]
fn repository_invalidation_retires_immutable_bodies_but_working_and_history_do_not() {
    let fixture = Fixture::new();
    let now = Instant::now();
    let key = fixture.key(GitComparison::Commit {
        id: "a".repeat(40).parse().unwrap(),
    });
    let mut cache = Cache::<&str>::default();
    let id = start(&cache.read(key.clone(), ReadReason::View, now).unwrap());
    assert!(
        cache
            .invalidate(key.query.root, key.project, Change::Working)
            .is_empty()
    );
    assert!(
        cache
            .invalidate(key.query.root, key.project, Change::History)
            .is_empty()
    );
    assert_eq!(
        cache.invalidate(key.query.root, key.project, Change::Repository),
        [id]
    );
}

#[test]
fn active_admission_and_identifier_exhaustion_never_start_unowned_work() {
    let fixture = Fixture::new();
    let now = Instant::now();
    let mut cache = Cache::<&str>::default();
    for _ in 0..MAX_ACTIVE {
        let mut key = fixture.key(GitComparison::Uncommitted);
        key.query.root = WorkspaceRoot::Project {
            project: ProjectId::random(),
        };
        let _id = start(&cache.read(key, ReadReason::View, now).unwrap());
    }
    assert!(matches!(
        cache.read(
            fixture.key(GitComparison::Uncommitted),
            ReadReason::View,
            now
        ),
        Err(Refusal::Busy)
    ));
    let mut exhausted = Cache::<&str> {
        issued: u64::MAX,
        ..Cache::default()
    };
    assert!(matches!(
        exhausted.read(
            fixture.key(GitComparison::Uncommitted),
            ReadReason::View,
            now
        ),
        Err(Refusal::Exhausted)
    ));
    assert!(exhausted.active.is_empty());
}

#[test]
fn a_forced_summary_invalidates_its_body_pages_but_not_another_comparison() {
    let fixture = Fixture::new();
    let now = Instant::now();
    let summary = fixture.key(GitComparison::Uncommitted);
    let mut file = summary.clone();
    file.subject = Subject::File {
        path: "a.rs".parse().unwrap(),
        offset: GitHunkOffset::default(),
    };
    let other = fixture.key(GitComparison::Committed { base: None });
    let mut cache = Cache::<&str>::default();
    let body = start(&cache.read(file.clone(), ReadReason::View, now).unwrap());
    let unrelated = start(&cache.read(other.clone(), ReadReason::View, now).unwrap());
    assert_eq!(cache.invalidate_siblings(&summary), [body]);
    assert!(
        matches!(cache.read(file, ReadReason::View, now).unwrap().read, Read::After(id) if id == body)
    );
    assert!(
        matches!(cache.read(other, ReadReason::View, now).unwrap().read, Read::Join(id) if id == unrelated)
    );
}

#[test]
fn recently_used_completed_payloads_survive_eviction_of_an_older_one() {
    let fixture = Fixture::new();
    let now = Instant::now();
    let first = fixture.key(GitComparison::Uncommitted);
    let mut second = first.clone();
    second.query.whitespace = GitWhitespace::Ignore;
    let mut cache = Cache::default();
    for key in [first.clone(), second.clone()] {
        let id = start(&cache.read(key, ReadReason::View, now).unwrap());
        let _done = cache.finish(id, ResultValue::Value(Arc::new("kept")), now);
    }
    for _ in 2..MAX_COMPLETED {
        let mut key = first.clone();
        key.query.root = WorkspaceRoot::Project {
            project: ProjectId::random(),
        };
        let id = start(&cache.read(key, ReadReason::View, now).unwrap());
        let _done = cache.finish(id, ResultValue::Value(Arc::new("other")), now);
    }
    assert!(matches!(
        cache
            .read(first.clone(), ReadReason::View, now)
            .unwrap()
            .read,
        Read::Hit(_)
    ));
    let mut extra = first.clone();
    extra.query.root = WorkspaceRoot::Project {
        project: ProjectId::random(),
    };
    let id = start(&cache.read(extra, ReadReason::View, now).unwrap());
    let _done = cache.finish(id, ResultValue::Value(Arc::new("newest")), now);
    assert!(matches!(
        cache.read(first, ReadReason::View, now).unwrap().read,
        Read::Hit(_)
    ));
    assert!(matches!(
        cache.read(second, ReadReason::View, now).unwrap().read,
        Read::Start(_)
    ));
}

#[test]
fn shared_ref_changes_retire_sibling_project_caches_but_working_changes_stay_local() {
    let fixture = Fixture::new();
    let now = Instant::now();
    for change in [Change::Working, Change::History, Change::Repository] {
        for (comparison, history_affects) in [
            (GitComparison::Uncommitted, false),
            (GitComparison::Committed { base: None }, true),
            (GitComparison::All { base: None }, true),
            (
                GitComparison::Commit {
                    id: "a".repeat(40).parse().unwrap(),
                },
                false,
            ),
        ] {
            let local = fixture.key(comparison);
            let mut sibling = local.clone();
            sibling.query.root = WorkspaceRoot::Worktree {
                worktree: vessel_proto::WorktreeId::random(),
            };
            let mut foreign = sibling.clone();
            foreign.project = ProjectId::random();
            foreign.query.root = WorkspaceRoot::Project {
                project: foreign.project,
            };
            let mut cache = Cache::default();
            let id = start(&cache.read(sibling.clone(), ReadReason::View, now).unwrap());
            let other = start(&cache.read(foreign.clone(), ReadReason::View, now).unwrap());
            let expected = match change {
                Change::Working => false,
                Change::History => history_affects,
                Change::Repository => true,
            };
            let cancelled = cache.invalidate(local.query.root, local.project, change);
            assert_eq!(cancelled, if expected { vec![id] } else { Vec::new() });
            assert_eq!(
                cache.finish(id, ResultValue::Value(Arc::new("sibling")), now),
                if expected {
                    Completion::Retired
                } else {
                    Completion::Published
                }
            );
            assert_eq!(
                cache.finish(other, ResultValue::Value(Arc::new("foreign")), now),
                Completion::Published
            );
            assert!(matches!(
                cache.read(foreign, ReadReason::View, now).unwrap().read,
                Read::Hit(_)
            ));
            let again = cache.read(sibling.clone(), ReadReason::View, now).unwrap();
            if expected {
                let _id = start(&again);
            } else {
                assert!(matches!(again.read, Read::Hit(_)));
            }
            // Completed sibling payloads obey exactly the same ownership rule.
            let mut complete = Cache::default();
            let id = start(
                &complete
                    .read(sibling.clone(), ReadReason::View, now)
                    .unwrap(),
            );
            let _published = complete.finish(id, ResultValue::Value(Arc::new("cached")), now);
            assert!(
                complete
                    .invalidate(local.query.root, local.project, change)
                    .is_empty()
            );
            let after = complete.read(sibling, ReadReason::View, now).unwrap();
            if expected {
                let _id = start(&after);
            } else {
                assert!(matches!(after.read, Read::Hit(_)));
            }
        }
    }
}
