use super::*;
use crate::devices::{MachineKey, Ticket, Tickets};
use vessel_proto::mcp::{Injection, McpSettings};
use vessel_proto::skills::{
    SkillNames, SkillOperation, SkillSelection, SkillsSaved, SkillsSnapshot, SkillsState,
};
use vessel_proto::{ErrorCode, NodeId};

fn next(tickets: &mut Tickets) -> Ticket {
    tickets.next_ticket().unwrap()
}
fn target(machine: &str, node: NodeId) -> Target {
    crate::connection::ConnectionEpochs::default()
        .admit(MachineKey::from_book(machine), node)
        .unwrap()
}
fn page(tickets: &mut Tickets) -> Page {
    Page::new(Scope {
        consumer: Consumer::Settings,
        target: target("host", NodeId::random()),
        visit: Visit::new(next(tickets)),
    })
}
fn names(items: &[&str]) -> SkillNames {
    SkillNames::new(items.iter().map(|name| name.parse().unwrap()).collect()).unwrap()
}
fn snapshot() -> SkillsSnapshot {
    SkillsSnapshot {
        state: SkillsState::UpToDate,
        ops: vec![],
        available: names(&["vessel", "vessel-advisor"]),
        installed: names(&["vessel", "vessel-advisor"]),
        selection: SkillSelection::All,
    }
}
fn loaded(page: &mut Page, tickets: &mut Tickets, snapshot: SkillsSnapshot) {
    let task = page.read_skills(next(tickets)).unwrap();
    assert!(page.answered(Answer {
        task,
        result: Ok(Output::Skills(snapshot))
    }));
}
#[test]
fn target_visit_consumer_and_attempt_all_fence_replies() {
    let mut tickets = Tickets::default();
    let mut page = page(&mut tickets);
    let first = page.read_tools(next(&mut tickets)).unwrap();
    let mut admissions = crate::connection::ConnectionEpochs::default();
    let _old = admissions
        .admit(MachineKey::from_book("host"), page.scope().target.node())
        .unwrap();
    let reconnected = admissions
        .admit(MachineKey::from_book("host"), page.scope().target.node())
        .unwrap();
    for changed in [
        Scope {
            target: reconnected,
            ..page.scope().clone()
        },
        Scope {
            consumer: Consumer::Onboarding,
            ..page.scope().clone()
        },
        Scope {
            visit: Visit::new(next(&mut tickets)),
            ..page.scope().clone()
        },
        Scope {
            target: target("host", NodeId::random()),
            ..page.scope().clone()
        },
        Scope {
            target: target("other", page.scope().target.node()),
            ..page.scope().clone()
        },
    ] {
        let mut stale = first.clone();
        stale.scope = changed;
        assert!(!page.answered(Answer {
            task: stale,
            result: Ok(Output::Tools(McpSettings::default()))
        }));
        assert!(page.tools.pending());
    }
    assert!(page.answered(Answer {
        task: first.clone(),
        result: Ok(Output::Tools(McpSettings::default()))
    }));
    let second = page.read_tools(next(&mut tickets)).unwrap();
    assert!(!page.answered(Answer {
        task: first,
        result: Err(Problem::Offline)
    }));
    assert!(page.tools.pending());
    assert!(page.answered(Answer {
        task: second,
        result: Err(Problem::TimedOut)
    }));
    assert_eq!(page.tools.value().unwrap().inject, Injection::Off);
    assert_eq!(page.tools.problem(), Some(Problem::TimedOut));
}
#[test]
fn tools_wait_for_actual_read_and_preserve_the_last_confirmed_state() {
    let mut tickets = Tickets::default();
    let mut page = page(&mut tickets);
    assert!(page.toggle_tools(next(&mut tickets)).is_none());
    let read = page.read_tools(next(&mut tickets)).unwrap();
    assert!(page.toggle_tools(next(&mut tickets)).is_none());
    assert!(page.answered(Answer {
        task: read,
        result: Ok(Output::Tools(McpSettings::default()))
    }));
    let change = page.toggle_tools(next(&mut tickets)).unwrap();
    assert_eq!(change.action, Action::Inject(Injection::On));
    assert_eq!(page.tools.value().unwrap().inject, Injection::Off);
    assert!(page.answered(change.refused(Problem::Refused(ErrorCode::Forbidden))));
    assert_eq!(page.tools.value().unwrap().inject, Injection::Off);
}
#[test]
fn picker_seeds_once_all_to_custom_selects_current_names_and_refresh_keeps_edits() {
    let mut tickets = Tickets::default();
    let mut page = page(&mut tickets);
    loaded(&mut page, &mut tickets, snapshot());
    page.choose();
    let picker = page.picker.as_mut().unwrap();
    picker.toggle_all();
    assert_eq!(
        picker.draft,
        SkillSelection::Custom {
            skills: names(&["vessel", "vessel-advisor"])
        }
    );
    picker.toggle(&"vessel-advisor".parse().unwrap());
    let draft = picker.draft.clone();
    loaded(&mut page, &mut tickets, snapshot());
    page.choose();
    assert_eq!(page.picker.as_ref().unwrap().draft, draft);
    page.close_picker();
    page.choose();
    assert_eq!(page.picker.as_ref().unwrap().draft, SkillSelection::All);
}
#[test]
fn save_requires_fresh_exact_removals_and_a_second_changed_plan_stays_open() {
    let mut tickets = Tickets::default();
    let mut page = page(&mut tickets);
    loaded(&mut page, &mut tickets, snapshot());
    page.choose();
    page.picker.as_mut().unwrap().toggle_all();
    page.picker
        .as_mut()
        .unwrap()
        .toggle(&"vessel-advisor".parse().unwrap());
    let first = page.save_picker(next(&mut tickets)).unwrap();
    assert!(
        matches!(&first.action, Action::Save(request) if request.confirmed_removals.is_empty())
    );
    page.close_picker();
    assert!(page.picker.is_some());
    let removals = names(&["vessel-advisor"]);
    let reply = SkillsSaved {
        snapshot: snapshot(),
        confirmation_required: Some(removals.clone()),
    };
    assert!(page.answered(Answer {
        task: first,
        result: Ok(Output::Saved(reply.clone()))
    }));
    assert!(
        matches!(&page.confirmation, Some(Confirmation::Removals { names, .. }) if *names == removals)
    );
    let confirmed = page.confirm(next(&mut tickets)).unwrap();
    assert!(
        matches!(&confirmed.action, Action::Save(request) if request.confirmed_removals == removals)
    );
    assert!(page.answered(Answer {
        task: confirmed,
        result: Ok(Output::Saved(reply))
    }));
    assert_eq!(page.skills.problem(), Some(Problem::Changed));
    assert!(page.picker.is_some());
    assert!(page.confirmation.is_none());
    let retry = page.save_picker(next(&mut tickets)).unwrap();
    assert!(
        matches!(&retry.action, Action::Save(request) if request.confirmed_removals.is_empty())
    );
    assert!(page.answered(Answer {
        task: retry,
        result: Ok(Output::Saved(SkillsSaved {
            snapshot: snapshot(),
            confirmation_required: None
        }))
    }));
    assert!(page.picker.is_none());
}
#[test]
fn uninstall_is_explicit_and_empty_selection_blocks_only_install() {
    let mut tickets = Tickets::default();
    let mut page = page(&mut tickets);
    loaded(&mut page, &mut tickets, snapshot());
    assert!(page.toggle_skills(next(&mut tickets)).is_none());
    assert!(matches!(
        page.confirmation,
        Some(Confirmation::Uninstall(_))
    ));
    page.cancel_confirmation();
    assert!(page.confirm(next(&mut tickets)).is_none());
    assert!(page.toggle_skills(next(&mut tickets)).is_none());
    let uninstall = page.confirm(next(&mut tickets)).unwrap();
    assert_eq!(uninstall.action, Action::Uninstall);
    let mut empty = snapshot();
    empty.state = SkillsState::NotInstalled;
    empty.installed = SkillNames::default();
    empty.selection = SkillSelection::Custom {
        skills: SkillNames::default(),
    };
    assert!(page.answered(Answer {
        task: uninstall,
        result: Ok(Output::Skills(empty))
    }));
    assert!(page.toggle_skills(next(&mut tickets)).is_none());
    page.choose();
    assert!(page.picker.is_some());
}
#[test]
fn update_confirms_only_maintenance_in_stable_order_and_wrong_reply_refuses() {
    let mut tickets = Tickets::default();
    let mut page = page(&mut tickets);
    let mut status = snapshot();
    status.state = SkillsState::Drift;
    status.ops = vec![
        SkillOperation::Delete {
            name: "vessel-advisor".parse().unwrap(),
        },
        SkillOperation::Update {
            name: "vessel".parse().unwrap(),
        },
        SkillOperation::Add {
            name: "vessel-advisor".parse().unwrap(),
        },
    ];
    loaded(&mut page, &mut tickets, status);
    page.ask_update();
    let Some(Confirmation::Update(ops)) = &page.confirmation else {
        panic!("maintenance confirmation missing")
    };
    assert_eq!(ops.len(), 2);
    assert!(matches!(ops.first(), Some(SkillOperation::Add { .. })));
    let task = page.confirm(next(&mut tickets)).unwrap();
    assert_eq!(task.action, Action::Reconcile);
    assert!(page.answered(Answer {
        task,
        result: Ok(Output::Tools(McpSettings::default()))
    }));
    assert_eq!(page.skills.problem(), Some(Problem::InvalidReply));
}

#[test]
fn uncertain_mutation_requires_a_fresh_read_and_keeps_the_picker_draft() {
    for problem in [Problem::TimedOut, Problem::Uncertain] {
        let mut tickets = Tickets::default();
        let mut page = page(&mut tickets);
        loaded(&mut page, &mut tickets, snapshot());
        page.choose();
        page.picker.as_mut().unwrap().toggle_all();
        page.picker
            .as_mut()
            .unwrap()
            .toggle(&"vessel-advisor".parse().unwrap());
        let draft = page.picker.as_ref().unwrap().draft.clone();
        let save = page.save_picker(next(&mut tickets)).unwrap();
        assert!(page.answered(save.refused(problem)));
        assert!(page.skills.requires_read());
        assert!(page.save_picker(next(&mut tickets)).is_none());
        assert!(page.confirm(next(&mut tickets)).is_none());
        let read = page.read_skills(next(&mut tickets)).unwrap();
        assert!(page.answered(Answer {
            task: read,
            result: Ok(Output::Skills(snapshot()))
        }));
        assert_eq!(page.picker.as_ref().unwrap().draft, draft);
        assert!(!page.skills.requires_read());
        assert!(page.save_picker(next(&mut tickets)).is_some());
    }
}
