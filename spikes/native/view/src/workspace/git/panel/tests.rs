//! Cases from Paseo's presentation, comparison and preference tests, plus
//! incomplete native replies: absence is never proof of an empty comparison.

use super::*;
use crate::workspace::git::fixtures::status;
use crate::workspace::problem::Problem;
use vessel_proto::workspace::{DiffScope, DiffSummary, Distance, WorktreeBase};

fn summary() -> DiffSummary {
    DiffSummary {
        from: None,
        files: Vec::new(),
        more: false,
    }
}

#[test]
fn default_preferences_match_the_reference_for_every_field() {
    assert_eq!(
        DisplayPreferences::default(),
        DisplayPreferences {
            layout: Layout::Unified,
            tree: TreeVisibility::Hidden,
            wrap: Wrap::NoWrap,
            whitespace: Whitespace::Show,
            inline: InlineDiff::Separate,
            commits: Fold::Closed,
        }
    );
    assert_eq!(Comparison::default().mode(), Some(ComparisonMode::All));
    assert_eq!(PanelState::default().commits, Fold::Closed);
}

#[test]
fn preference_round_trip_does_not_change_the_selected_values() {
    let preferences = DisplayPreferences {
        layout: Layout::Split,
        tree: TreeVisibility::Visible,
        wrap: Wrap::Wrap,
        whitespace: Whitespace::Ignore,
        inline: InlineDiff::Inline,
        commits: Fold::Open,
    };
    let saved = serde_json::to_string(&preferences).unwrap();
    assert_eq!(
        serde_json::from_str::<DisplayPreferences>(&saved).unwrap(),
        preferences
    );
    assert_eq!(
        serde_json::from_str::<DisplayPreferences>("{}").unwrap(),
        DisplayPreferences::default()
    );
}

#[test]
fn presentation_projects_the_target_and_capabilities_without_changing_preferences() {
    for (form, inline, expected) in [
        (
            FormFactor::Desktop,
            InlineDiff::Separate,
            Presentation::Tree,
        ),
        (
            FormFactor::Desktop,
            InlineDiff::Inline,
            Presentation::Combined,
        ),
        (
            FormFactor::Compact,
            InlineDiff::Separate,
            Presentation::Combined,
        ),
        (
            FormFactor::Compact,
            InlineDiff::Inline,
            Presentation::Combined,
        ),
    ] {
        assert_eq!(
            Presentation::for_target(&PanelTarget::Changes, inline, form),
            expected
        );
        assert_eq!(
            Presentation::for_target(&PanelTarget::WorkingDiff, inline, form),
            Presentation::Diff
        );
        let commit = PanelTarget::CommitDiff("a".repeat(40).parse().unwrap());
        assert_eq!(
            Presentation::for_target(&commit, inline, form),
            Presentation::Diff
        );
    }
    assert_eq!(
        Layout::Split.effective(FormFactor::Compact),
        Layout::Unified
    );
    assert_eq!(Layout::Split.effective(FormFactor::Desktop), Layout::Split);
}

#[test]
fn committed_comparison_is_never_sent_as_the_legacy_all_changes_scope() {
    let base = WorktreeBase::Ref {
        name: "origin/main".parse().unwrap(),
    };
    let committed = Comparison::new(
        ComparisonMode::Committed,
        Some(base.clone()),
        Whitespace::Show,
    );
    assert_eq!(committed.base(), Some(&base));
    assert_eq!(committed.legacy_scope(), None);
    assert_eq!(
        Comparison::new(ComparisonMode::All, Some(base.clone()), Whitespace::Show).legacy_scope(),
        Some(DiffScope::Base {
            base: Some(base.clone())
        })
    );
    let uncommitted = Comparison::new(ComparisonMode::Uncommitted, Some(base), Whitespace::Ignore);
    assert_eq!(uncommitted.base(), None);
    assert_eq!(uncommitted.legacy_scope(), Some(DiffScope::Uncommitted));
    assert!(uncommitted.whitespace().ignored());
}

#[test]
fn body_precedence_keeps_connection_and_repository_failures_above_diff_results() {
    let problem = ReadFailure::Call(Problem::Unsent);
    let comparison = Comparison::default();
    let repository = status(Vec::new());
    let diff = summary();
    for (demand, read, expected) in [
        (
            Demand::Inactive,
            RepositoryRead::Ready(&repository),
            Body::Inactive,
        ),
        (
            Demand::Disconnected,
            RepositoryRead::Failed(&problem),
            Body::Connecting,
        ),
        (
            Demand::Active,
            RepositoryRead::Waiting,
            Body::CheckingRepository,
        ),
        (
            Demand::Active,
            RepositoryRead::Failed(&problem),
            Body::RepositoryFailed(&problem),
        ),
        (Demand::Active, RepositoryRead::NotGit, Body::NotRepository),
    ] {
        assert_eq!(
            body(demand, read, DiffRead::Ready(&diff), &comparison),
            expected
        );
    }
    for (read, expected) in [
        (DiffRead::Waiting, Body::ReadingDiff),
        (DiffRead::TooLarge, Body::DiffTooLarge),
        (DiffRead::Failed(&problem), Body::DiffFailed(&problem)),
        (DiffRead::Ready(&diff), Body::Empty(None)),
    ] {
        assert_eq!(
            body(
                Demand::Active,
                RepositoryRead::Ready(&repository),
                read,
                &comparison
            ),
            expected
        );
    }
}

#[test]
fn an_incomplete_diff_without_files_is_not_an_empty_diff() {
    let repository = status(Vec::new());
    let diff = DiffSummary {
        more: true,
        ..summary()
    };
    assert_eq!(
        body(
            Demand::Active,
            RepositoryRead::Ready(&repository),
            DiffRead::Ready(&diff),
            &Comparison::default()
        ),
        Body::Files(&diff)
    );
}

#[test]
fn empty_actions_follow_comparison_and_known_facts_and_respect_whitespace_filtering() {
    let mut repository = status(Vec::new());
    repository.more = true;
    let committed = Comparison::new(ComparisonMode::Committed, None, Whitespace::Show);
    assert_eq!(
        empty_action(&committed, &repository),
        Some(EmptyAction::Uncommitted)
    );
    repository.more = false;
    assert_eq!(empty_action(&committed, &repository), None);
    repository.base = Some(Distance {
        name: "origin/main".parse().unwrap(),
        ahead: 1,
        behind: 0,
    });
    let uncommitted = Comparison::new(ComparisonMode::Uncommitted, None, Whitespace::Show);
    assert_eq!(
        empty_action(&uncommitted, &repository),
        Some(EmptyAction::Committed)
    );
    for mode in [
        ComparisonMode::Committed,
        ComparisonMode::Uncommitted,
        ComparisonMode::All,
    ] {
        assert_eq!(
            empty_action(
                &Comparison::new(mode, None, Whitespace::Ignore),
                &repository
            ),
            None
        );
    }
    assert_eq!(empty_action(&Comparison::default(), &repository), None);
}
