//! A sidebar row's bucket and a project's aggregate (TRACKER 662/663):
//! Paseo's five in their urgency order, a project's the most urgent of its
//! listed sessions.

use vessel_proto::{ProjectId, SessionStatus};

use super::fixture::{fleet, machine, session};
use crate::attention::Bucket;
use crate::board::{Attention, Phrase, RunsIn};
use crate::prefs::SidebarView;
use crate::sidebar::status::WorkspaceKind;
use crate::{Card, Fleet};

fn book() -> Fleet {
    fleet(vec![machine(
        "laptop",
        true,
        &[
            session("aaaa", "2026-11-19T08:00:00Z", SessionStatus::Running),
            session("bbbb", "2026-11-19T09:00:00Z", SessionStatus::Running),
            session("cccc", "2026-11-19T10:00:00Z", SessionStatus::Running),
        ],
    )])
}

fn cards(fleet: &mut Fleet) -> &mut [Card] {
    fleet.machines.first_mut().unwrap().sessions.as_mut_slice()
}

fn first(fleet: &mut Fleet) -> &mut Card {
    cards(fleet).first_mut().unwrap()
}

#[test]
fn a_session_says_the_most_urgent_of_what_its_card_knows() {
    let mut fleet = book();
    let card = first(&mut fleet);
    assert_eq!(card.bucket(), Bucket::Done);
    card.detail.marks.unread = true;
    assert_eq!(card.bucket(), Bucket::Review);
    card.attention = Attention::Working;
    assert_eq!(card.bucket(), Bucket::Working);
    card.attention = Attention::NeedsYou;
    assert_eq!(card.bucket(), Bucket::NeedsInput);
    // A process that failed, whatever else is marked.
    card.attention = Attention::AtRest;
    card.live = false;
    card.detail.failure = Some("handshake".to_owned());
    assert_eq!(card.bucket(), Bucket::Failed);
    // Starting is busy as working is: one ring.
    card.detail.failure = None;
    card.detail.marks.unread = false;
    card.live = true;
    card.detail.process = Phrase::Starting;
    assert_eq!(card.bucket(), Bucket::Working);
}

#[test]
fn a_project_says_its_most_urgent_listed_session() {
    let project = ProjectId::random();
    let other = ProjectId::random();
    let mut fleet = book();
    let view = SidebarView::default();
    // No session of its: done, which draws no badge.
    assert_eq!(
        fleet.project_bucket("laptop", project, view),
        Some(Bucket::Done)
    );
    assert_eq!(fleet.project_bucket("gpu-box", project, view), None);
    {
        let [unread, working, needing] = cards(&mut fleet) else {
            panic!("three sessions");
        };
        unread.detail.runs_in = Some(RunsIn::Checkout(project));
        unread.detail.marks.unread = true;
        working.detail.runs_in = Some(RunsIn::Worktree(project));
        working.attention = Attention::Working;
        // Another project's need is not this one's.
        needing.detail.runs_in = Some(RunsIn::Checkout(other));
        needing.attention = Attention::NeedsYou;
    }
    assert_eq!(
        fleet.project_bucket("laptop", project, view),
        Some(Bucket::Working)
    );
    assert_eq!(
        fleet.project_bucket("laptop", other, view),
        Some(Bucket::NeedsInput)
    );
    // An archived session is never counted.
    let working = cards(&mut fleet)
        .iter_mut()
        .find(|card| card.attention == Attention::Working)
        .unwrap();
    working.detail.marks.archived_at = Some(chrono::Utc::now());
    assert_eq!(
        fleet.project_bucket("laptop", project, view),
        Some(Bucket::Review)
    );
}

#[test]
fn what_a_session_runs_in_names_its_glyph() {
    let mut fleet = book();
    let card = first(&mut fleet);
    assert_eq!(card.workspace_kind(), WorkspaceKind::Folder);
    let project = ProjectId::random();
    card.detail.runs_in = Some(RunsIn::Checkout(project));
    assert_eq!(card.workspace_kind(), WorkspaceKind::Checkout);
    card.detail.runs_in = Some(RunsIn::Worktree(project));
    assert_eq!(card.workspace_kind(), WorkspaceKind::Worktree);
}
