use super::{
    DistanceName, HeadActivity, HeadName, HeadState, Reading, RequestKeys, Writing, fixtures,
    git_head, head_activity,
};
use vessel_proto::workspace::{Distance, GitOperation};

fn distance(name: &str) -> Distance {
    Distance {
        name: name.parse().unwrap(),
        ahead: 0,
        behind: 0,
    }
}

#[test]
fn a_mirrored_upstream_does_not_repeat_the_branch_and_a_self_base_is_omitted() {
    let mut status = fixtures::status(vec![]);
    status.head = Some("1111111111111111111111111111111111111111".parse().unwrap());
    status.branch = Some("feature/long-navigation-name".parse().unwrap());
    status.upstream = Some(distance("origin/feature/long-navigation-name"));
    status.base = Some(distance("feature/long-navigation-name"));
    let head = git_head(&status, HeadActivity::Idle);
    assert_eq!(head.name, HeadName::Branch(status.branch.as_ref().unwrap()));
    assert_eq!(
        head.state,
        HeadState::Upstream {
            distance: status.upstream.as_ref().unwrap(),
            name: DistanceName::ImplicitUpstream
        }
    );
    assert_eq!(head.base, None);
    status.base = Some(distance("origin/feature/long-navigation-name"));
    assert_eq!(git_head(&status, HeadActivity::Idle).base, None);
    status.base = Some(distance("main"));
    assert_eq!(
        git_head(&status, HeadActivity::Idle).base,
        status.base.as_ref()
    );
    status.upstream = Some(distance("origin/main"));
    assert_eq!(
        git_head(&status, HeadActivity::Idle).state,
        HeadState::Upstream {
            distance: status.upstream.as_ref().unwrap(),
            name: DistanceName::Explicit
        }
    );
}

#[test]
fn activity_incomplete_reads_and_branch_states_have_explicit_precedence() {
    let mut status = fixtures::status(vec![]);
    assert_eq!(
        git_head(&status, HeadActivity::Idle).state,
        HeadState::NoCommits
    );
    status.head = Some("1111111111111111111111111111111111111111".parse().unwrap());
    assert_eq!(
        git_head(&status, HeadActivity::Idle).state,
        HeadState::NoUpstream
    );
    status.branch = None;
    assert_eq!(
        git_head(&status, HeadActivity::Idle).name,
        HeadName::Detached
    );
    assert_eq!(
        git_head(&status, HeadActivity::Idle).state,
        HeadState::Detached(status.head.as_ref().unwrap())
    );
    status.more = true;
    assert_eq!(
        git_head(&status, HeadActivity::Idle).state,
        HeadState::Incomplete
    );
    status.operation = Some(GitOperation::Merge);
    assert_eq!(
        git_head(&status, HeadActivity::Idle).state,
        HeadState::Operation(GitOperation::Merge)
    );
    status.base = Some(distance("main"));
    assert_eq!(
        git_head(&status, HeadActivity::Reading).state,
        HeadState::Refreshing
    );
    assert_eq!(
        git_head(&status, HeadActivity::Writing).state,
        HeadState::Working
    );
    assert_eq!(git_head(&status, HeadActivity::Writing).base, None);
    status.head = None;
    assert_eq!(
        git_head(&status, HeadActivity::Idle).name,
        HeadName::Missing
    );
}

#[test]
fn a_pending_write_wins_over_a_pending_read() {
    let status = fixtures::status(vec![]);
    let mut reading = Reading::Ready(status);
    let mut writing = Writing::default();
    assert_eq!(head_activity(&reading, &writing), HeadActivity::Idle);
    let mut keys = RequestKeys::default();
    let read = keys.issue().unwrap();
    reading.start(read);
    assert_eq!(head_activity(&reading, &writing), HeadActivity::Reading);
    assert!(writing.begin(keys.issue().unwrap()));
    assert_eq!(head_activity(&reading, &writing), HeadActivity::Writing);
}

#[test]
fn another_branch_with_the_same_tail_is_not_mistaken_for_the_current_branch() {
    let mut status = fixtures::status(vec![]);
    status.head = Some("1111111111111111111111111111111111111111".parse().unwrap());
    status.branch = Some("feature".parse().unwrap());
    status.upstream = Some(distance("origin/other/feature"));
    assert_eq!(
        git_head(&status, HeadActivity::Idle).state,
        HeadState::Upstream {
            distance: status.upstream.as_ref().unwrap(),
            name: DistanceName::Explicit,
        }
    );
    assert_eq!(
        crate::Lang::En
            .git_header(&status, HeadActivity::Idle)
            .state,
        "Up to date with origin/other/feature"
    );
    assert_eq!(
        crate::Lang::Ko
            .git_header(&status, HeadActivity::Idle)
            .state,
        "origin/other/feature 기준 최신 상태"
    );
}
