//! Real guarded Devices replies prove the paired fixture, not A3 UI authority.
use super::*;
use crate::devices::{Answer, Ask, Outcome as DeviceOutcome, Received};
use vessel_app_view::devices::Tickets;
use vessel_proto::{DeviceSelf, Outcome, Reply, Scope};

fn pair(nodes: PeerNodes) -> (Link, Scenario) {
    Scenario::start_pair(
        &Name::parse("fixture-first").unwrap(),
        &Name::parse("fixture-second").unwrap(),
        nodes,
    )
    .unwrap()
}
fn begin(link: &Link, scene: &Scenario, peer: PeerSlot, tickets: &mut Tickets) {
    link.submit(Command::Devices {
        context: scene.context_of(peer).unwrap(),
        ticket: tickets.next_ticket().unwrap(),
        ask: Ask::Authority,
    })
    .unwrap();
}
fn next_answer(link: &Link, scene: &mut Scenario) -> Answer {
    const MAX_EVENTS: usize = 32;
    for _ in 0..MAX_EVENTS {
        if let Event::Devices(answer) = scene.event(link).unwrap() {
            return answer;
        }
    }
    panic!("expected a bounded genuine Devices answer");
}
fn reported(link: &Link, scene: &mut Scenario, peer: PeerSlot) -> Answer {
    assert!(matches!(
        scene.request_from(link, peer).unwrap(),
        Call::GetDeviceSelf(_)
    ));
    scene
        .reply_to(
            link,
            peer,
            Outcome::Ok(Reply::GetDeviceSelf(DeviceSelf {
                device: None,
                scopes: vec![Scope::Admin],
            })),
        )
        .unwrap();
    let answer = next_answer(link, scene);
    assert_eq!(answer.context(), &scene.context_of(peer).unwrap());
    answer
}
fn completed(answer: Answer) {
    let Received::Completed { outcome, .. } = answer.receive() else {
        panic!("the independent physical admission remains current");
    };
    assert!(
        matches!(outcome, DeviceOutcome::Authority(_, Ok(DeviceSelf { device: None, scopes })) if scopes == [Scope::Admin])
    );
}

#[test]
fn paired_same_node_retirement_refuses_only_its_original_queued_reply() {
    let (link, mut scene) = pair(PeerNodes::SameNode);
    let first = scene.context();
    let second = scene.context_of(PeerSlot::Secondary).unwrap();
    assert_eq!(first.node(), second.node());
    assert_ne!(first.machine(), second.machine());
    assert_ne!(
        first.epoch(),
        second.epoch(),
        "one State allocates both actual admissions"
    );
    let mut tickets = Tickets::default();
    begin(&link, &scene, PeerSlot::Primary, &mut tickets);
    begin(&link, &scene, PeerSlot::Secondary, &mut tickets);
    // Reversing peer reads/replies must not consume the other's physical call.
    let second_reply = reported(&link, &mut scene, PeerSlot::Secondary);
    let first_reply = reported(&link, &mut scene, PeerSlot::Primary);
    scene.retire_peer(&link, PeerSlot::Primary).unwrap();
    assert!(matches!(first_reply.receive(), Received::Retired { context, .. } if context == first));
    completed(second_reply);
    assert_eq!(scene.context_of(PeerSlot::Secondary), Some(second));
    assert_eq!(
        scene.transcript_of(&link, PeerSlot::Primary).unwrap().len(),
        1
    );
    assert_eq!(
        scene
            .transcript_of(&link, PeerSlot::Secondary)
            .unwrap()
            .len(),
        1
    );
    scene.close(&link).unwrap();
}

#[test]
fn paired_physical_close_never_retargets_the_other_peer_even_with_the_same_node() {
    let (link, mut scene) = pair(PeerNodes::SameNode);
    let mut tickets = Tickets::default();
    begin(&link, &scene, PeerSlot::Primary, &mut tickets);
    begin(&link, &scene, PeerSlot::Secondary, &mut tickets);
    let first = reported(&link, &mut scene, PeerSlot::Primary);
    let second = reported(&link, &mut scene, PeerSlot::Secondary);
    scene.close_peer_at(&link, PeerSlot::Primary).unwrap();
    // A real refusal acknowledges that the client observed physical EOF; no
    // scheduling sleep or successful synchronous guard is used as that barrier.
    begin(&link, &scene, PeerSlot::Primary, &mut tickets);
    assert!(matches!(
        next_answer(&link, &mut scene).receive(),
        Received::Retired { .. } | Received::Refused { .. }
    ));
    assert!(matches!(first.receive(), Received::Retired { .. }));
    completed(second);
    assert_eq!(
        scene.transcript_of(&link, PeerSlot::Primary).unwrap().len(),
        1
    );
    assert_eq!(
        scene
            .transcript_of(&link, PeerSlot::Secondary)
            .unwrap()
            .len(),
        1
    );
    scene.close(&link).unwrap();
}

#[test]
fn reconnecting_one_paired_peer_keeps_the_other_original_lease_and_context() {
    let (link, mut scene) = pair(PeerNodes::DifferentNodes);
    let first = scene.context();
    let second = scene.context_of(PeerSlot::Secondary).unwrap();
    assert_ne!(first.node(), second.node());
    let mut tickets = Tickets::default();
    begin(&link, &scene, PeerSlot::Primary, &mut tickets);
    begin(&link, &scene, PeerSlot::Secondary, &mut tickets);
    let old = reported(&link, &mut scene, PeerSlot::Primary);
    let other = reported(&link, &mut scene, PeerSlot::Secondary);
    let next = scene
        .reconnect_peer(&link, PeerSlot::Primary, Reconnect::SameNode)
        .unwrap();
    assert_eq!(next.node(), first.node());
    assert_eq!(next.machine(), first.machine());
    assert_ne!(next.epoch(), first.epoch());
    assert_ne!(next.epoch(), second.epoch());
    assert!(matches!(old.receive(), Received::Retired { context, .. } if context == first));
    completed(other);
    assert_eq!(scene.context_of(PeerSlot::Secondary), Some(second));
    begin(&link, &scene, PeerSlot::Primary, &mut tickets);
    completed(reported(&link, &mut scene, PeerSlot::Primary));
    assert_eq!(
        scene
            .transcript_of(&link, PeerSlot::Secondary)
            .unwrap()
            .len(),
        1
    );
    scene.close(&link).unwrap();
}

#[test]
fn paired_slots_reject_duplicate_names_missing_peers_and_wrong_link_owners() {
    let name = Name::parse("fixture").unwrap();
    assert!(matches!(
        Scenario::start_pair(&name, &name, PeerNodes::SameNode),
        Err(FixtureError::DuplicateName)
    ));
    let (first, mut one) = Scenario::start(&name).unwrap();
    let (second, mut two) = pair(PeerNodes::DifferentNodes);
    assert_eq!(one.context_of(PeerSlot::Secondary), None);
    assert!(matches!(
        one.request_from(&first, PeerSlot::Secondary),
        Err(FixtureError::Protocol)
    ));
    assert!(matches!(
        two.request_from(&first, PeerSlot::Primary),
        Err(FixtureError::WrongLink)
    ));
    assert_eq!(one.transcript(&first).unwrap().len(), 0);
    assert_eq!(
        two.transcript_of(&second, PeerSlot::Primary).unwrap().len(),
        0
    );
    assert_eq!(
        two.transcript_of(&second, PeerSlot::Secondary)
            .unwrap()
            .len(),
        0
    );
    one.close(&first).unwrap();
    two.close(&second).unwrap();
}
