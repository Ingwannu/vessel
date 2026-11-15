//! Removal intent after Paseo's `providers-section.tsx:410–440`.
//! Its section test currently covers row open/toggle, not removal races;
//! these cases also exercise Vessel's typed machine and reply boundaries.

use super::*;
use crate::agent_status::command::CommandShell;
use crate::agent_status::row::Activity;
use vessel_proto::agents::{AgentInfo, AgentSource, Installed, SignedIn, UnknownReason};
use vessel_proto::{Harness, HarnessKind, StructuredSupport, TerminalSupport};

mod uncertainty;

fn agent() -> HarnessRef {
    HarnessRef::Added("example".parse().unwrap())
}
fn machine() -> MachineKey {
    MachineKey::from_book("one")
}
fn inventory() -> Inventory {
    let target = agent();
    Inventory {
        node: "00000000-0000-4000-8000-000000000001".parse().unwrap(),
        command_shell: CommandShell::Unix,
        harnesses: vec![Harness {
            kind: target.clone(),
            path: "/opt/example".into(),
            version: None,
            structured: StructuredSupport::Available,
            terminal: TerminalSupport::Available,
        }],
        agents: vec![AgentInfo {
            agent: target.clone(),
            detail: AgentDetail::Added {
                added: AddedAgent {
                    label: Some("Example".parse().unwrap()),
                    program: "example".parse().unwrap(),
                    args: vec!["--stdio".into()],
                    env: Vec::new(),
                    secrets: Vec::new(),
                    inherit: Vec::new(),
                    mcp: Vec::new(),
                    source: AgentSource::Command,
                },
            },
        }],
        readiness: vec![vessel_proto::agents::AgentReadiness {
            agent: target,
            profile: "example".parse().unwrap(),
            installed: Installed::Unknown {
                reason: UnknownReason::NotChecked,
            },
            signed_in: SignedIn::NotRequired,
            checked_at: "2026-11-13T00:00:00Z".parse().unwrap(),
        }],
        administration: Administration::Supported,
    }
}
fn load(pages: &mut Pages<&'static str>, key: &MachineKey, snapshot: Inventory) {
    let ticket = pages.begin(key.clone(), Operation::Discovery).unwrap();
    assert!(pages.loaded(key, ticket, snapshot));
}
fn pages() -> Pages<&'static str> {
    let mut pages = Pages::default();
    load(&mut pages, &machine(), inventory());
    pages
}

#[test]
fn only_explicitly_listed_added_definitions_have_a_remove_action() {
    let mut snapshot = inventory();
    assert!(can_remove(&agent(), &snapshot));
    for builtin in HarnessKind::ALL {
        assert!(!can_remove(&HarnessRef::Known(builtin), &snapshot));
    }
    assert!(!can_remove(
        &HarnessRef::Added("absent".parse().unwrap()),
        &snapshot
    ));
    snapshot.administration = Administration::OlderNode;
    assert!(!can_remove(&agent(), &snapshot));
    snapshot.administration = Administration::Supported;
    // Discovery alone does not prove the definition is owned by the node.
    snapshot.agents.clear();
    assert!(!can_remove(&agent(), &snapshot));
    let pages = Pages::<&str>::default();
    assert!(matches!(
        Confirmation::new(&pages, machine(), &agent()),
        Err(Refused::Unavailable)
    ));
}

#[test]
fn registry_and_missing_program_are_still_added_definitions() {
    let mut snapshot = inventory();
    let AgentDetail::Added { added } = &mut snapshot.agents[0].detail else {
        panic!("added fixture")
    };
    added.source = AgentSource::Registry {
        id: "example-registry-entry".parse().unwrap(),
        version: "1.0".parse().unwrap(),
    };
    snapshot.harnesses.clear();
    snapshot.readiness[0].installed = Installed::Missing {
        program: "missing-program".parse().unwrap(),
    };
    assert!(can_remove(&agent(), &snapshot));
    let mut pages = Pages::default();
    load(&mut pages, &machine(), snapshot);
    assert!(Confirmation::new(&pages, machine(), &agent()).is_ok());
}

#[test]
fn cancel_starts_nothing_and_confirmation_keeps_the_original_machine() {
    let mut pages = pages();
    let cancel = Confirmation::new(&pages, machine(), &agent()).unwrap();
    drop(cancel);
    assert!(pages.get("one").unwrap().pending().is_none());
    let confirm = Confirmation::new(&pages, machine(), &agent()).unwrap();
    assert!(matches!(
        confirm.confirm(&MachineKey::from_book("two"), &mut pages),
        Err(Refused::MachineChanged)
    ));
    assert!(pages.get("one").unwrap().pending().is_none());
    assert!(pages.get("two").is_none());
}

#[test]
fn refreshed_or_removed_definitions_need_a_new_confirmation() {
    let mut pages = pages();
    let confirm = Confirmation::new(&pages, machine(), &agent()).unwrap();
    let mut changed = inventory();
    let AgentDetail::Added { added } = &mut changed.agents[0].detail else {
        panic!("added fixture")
    };
    added.args.push("--other-account".into());
    load(&mut pages, &machine(), changed);
    assert!(matches!(
        confirm.confirm(&machine(), &mut pages),
        Err(Refused::DefinitionChanged)
    ));
    assert!(pages.get("one").unwrap().pending().is_none());
    let confirm = Confirmation::new(&pages, machine(), &agent()).unwrap();
    let mut gone = inventory();
    gone.agents.clear();
    load(&mut pages, &machine(), gone);
    assert!(matches!(
        confirm.confirm(&machine(), &mut pages),
        Err(Refused::Unavailable)
    ));
}

#[test]
fn in_flight_work_is_not_overwritten_by_a_second_confirmation() {
    let mut pages = pages();
    let confirm = Confirmation::new(&pages, machine(), &agent()).unwrap();
    let refresh = pages.begin(machine(), Operation::Discovery).unwrap();
    assert!(matches!(
        Confirmation::new(&pages, machine(), &agent()),
        Err(Refused::Request(StartProblem::Busy))
    ));
    assert!(matches!(
        confirm.confirm(&machine(), &mut pages),
        Err(Refused::Request(StartProblem::Busy))
    ));
    assert_eq!(pages.get("one").unwrap().pending().unwrap().id, refresh);
}

#[test]
fn acknowledgement_is_scoped_to_ticket_name_and_operation() {
    let mut pages = pages();
    load(&mut pages, &MachineKey::from_book("two"), inventory());
    pages.select("one", Some(agent()));
    let request = Confirmation::new(&pages, machine(), &agent())
        .unwrap()
        .confirm(&machine(), &mut pages)
        .unwrap();
    assert!(!pages.removed(
        &MachineKey::from_book("two"),
        request.ticket,
        request.node,
        &request.name
    ));
    assert!(!pages.removed(
        &machine(),
        request.ticket,
        request.node,
        &"other".parse().unwrap()
    ));
    assert!(!pages.removed(&machine(), RequestId::ZERO, request.node, &request.name));
    assert_eq!(
        Activity::for_agent(
            &agent(),
            Some(&Operation::Remove {
                node: request.node,
                name: request.name.clone()
            })
        ),
        Activity::Idle
    );
    assert!(pages.removed(
        &request.machine,
        request.ticket,
        request.node,
        &request.name
    ));
    assert!(!pages.removed(
        &request.machine,
        request.ticket,
        request.node,
        &request.name
    ));
    let page = pages.get("one").unwrap();
    let snapshot = page.inventory().unwrap();
    assert!(snapshot.agents.is_empty());
    assert!(snapshot.harnesses.is_empty());
    assert!(snapshot.readiness.is_empty());
    assert!(page.selected().is_none());
    assert!(page.pending().is_none());
    assert!(can_remove(
        &agent(),
        pages.get("two").unwrap().inventory().unwrap()
    ));
    let refresh = pages.begin(machine(), Operation::Discovery).unwrap();
    assert!(!pages.removed(&machine(), refresh, request.node, &request.name));
    assert_eq!(pages.get("one").unwrap().pending().unwrap().id, refresh);
}

#[test]
fn refusal_keeps_rows_selection_and_retry_while_late_success_cannot_delete_them() {
    let mut pages = pages();
    pages.select("one", Some(agent()));
    let request = Confirmation::new(&pages, machine(), &agent())
        .unwrap()
        .confirm(&machine(), &mut pages)
        .unwrap();
    pages.failed(&request.machine, request.ticket, "forbidden");
    let page = pages.get("one").unwrap();
    assert!(page.pending().is_none());
    assert_eq!(page.selected(), Some(&agent()));
    assert!(can_remove(&agent(), page.inventory().unwrap()));
    assert_eq!(page.issue().unwrap().source, "forbidden");
    assert_eq!(
        page.issue().unwrap().operation,
        Operation::Remove {
            node: request.node,
            name: request.name.clone()
        }
    );
    let retry = Confirmation::new(&pages, machine(), &agent())
        .unwrap()
        .confirm(&machine(), &mut pages)
        .unwrap();
    assert_ne!(retry.ticket, request.ticket);
    assert!(!pages.removed(
        &request.machine,
        request.ticket,
        request.node,
        &request.name
    ));
    assert!(can_remove(
        &agent(),
        pages.get("one").unwrap().inventory().unwrap()
    ));
}

#[test]
fn removing_one_definition_preserves_siblings_and_an_unrelated_detail() {
    let mut snapshot = inventory();
    let sibling = HarnessRef::Added("sibling".parse().unwrap());
    let mut info = snapshot.agents[0].clone();
    info.agent = sibling.clone();
    snapshot.agents.push(info);
    let mut harness = snapshot.harnesses[0].clone();
    harness.kind = sibling.clone();
    snapshot.harnesses.push(harness);
    let mut ready = snapshot.readiness[0].clone();
    ready.agent = sibling.clone();
    snapshot.readiness.push(ready);
    let mut pages = Pages::default();
    load(&mut pages, &machine(), snapshot);
    pages.select("one", Some(sibling.clone()));
    let request = Confirmation::new(&pages, machine(), &agent())
        .unwrap()
        .confirm(&machine(), &mut pages)
        .unwrap();
    assert!(pages.removed(&machine(), request.ticket, request.node, &request.name));
    let page = pages.get("one").unwrap();
    let kept = page.inventory().unwrap();
    assert_eq!(page.selected(), Some(&sibling));
    assert_eq!(kept.agents.len(), 1);
    assert_eq!(kept.agents[0].agent, sibling);
    assert_eq!(kept.harnesses.len(), 1);
    assert_eq!(kept.harnesses[0].kind, sibling);
    assert_eq!(kept.readiness.len(), 1);
    assert_eq!(kept.readiness[0].agent, sibling);
}

#[test]
fn a_reused_book_name_does_not_redirect_confirmation_or_acknowledgement() {
    let mut pages = pages();
    let confirm = Confirmation::new(&pages, machine(), &agent()).unwrap();
    let mut replaced = inventory();
    replaced.node = "00000000-0000-4000-8000-000000000002".parse().unwrap();
    load(&mut pages, &machine(), replaced);
    assert!(matches!(
        confirm.confirm(&machine(), &mut pages),
        Err(Refused::NodeChanged)
    ));
    let request = Confirmation::new(&pages, machine(), &agent())
        .unwrap()
        .confirm(&machine(), &mut pages)
        .unwrap();
    let other_node = "00000000-0000-4000-8000-000000000001".parse().unwrap();
    assert!(!pages.removed(&machine(), request.ticket, other_node, &request.name));
    assert!(can_remove(
        &agent(),
        pages.get("one").unwrap().inventory().unwrap()
    ));
    assert!(pages.removed(&machine(), request.ticket, request.node, &request.name));
}

#[test]
fn a_removal_only_disables_its_exact_added_agent_action() {
    let node = inventory().node;
    let target = agent();
    let operation = Operation::Remove {
        node,
        name: "custom".parse().unwrap(),
    };
    assert!(!is_removing(&target, None));
    assert!(!is_removing(&target, Some(&Operation::Discovery)));
    assert!(is_removing(
        &HarnessRef::Added("custom".parse().unwrap()),
        Some(&operation)
    ));
    assert!(!is_removing(
        &HarnessRef::Added("sibling".parse().unwrap()),
        Some(&operation)
    ));
    assert!(!is_removing(
        &HarnessRef::Known(HarnessKind::Codex),
        Some(&operation)
    ));
}
