//! Transition regressions, independent of a UI executor or an invented node.

use super::*;

#[test]
fn manager_back_preserves_search_and_cannot_abandon_a_pending_write() {
    let mut manager = Manager::default();
    manager.search("ready");
    manager.edit(label("Ready", Color::Violet)).unwrap();
    manager.back().unwrap();
    assert_eq!(manager.query(), "ready");
    manager.edit(label("Ready", Color::Violet)).unwrap();
    manager.draft_mut().unwrap().set_color(Color::Sky);
    let request = ticket(&mut Tickets::default());
    assert_eq!(
        manager.save(HostState::Online, &ready_catalog(), request),
        Ok(label("Ready", Color::Sky))
    );
    assert_eq!(manager.back(), Err(Failure::Pending));
    assert!(matches!(manager.state(), ManagerState::Saving { ticket, .. } if *ticket == request));
}

fn name(raw: &str) -> Name {
    raw.parse::<Name>().unwrap()
}
fn key(raw: &str) -> HostKey {
    let id = match raw {
        "a" => "00000000-0000-4000-8000-000000000001",
        "b" => "00000000-0000-4000-8000-000000000002",
        "offline" => "00000000-0000-4000-8000-000000000003",
        "failed" => "00000000-0000-4000-8000-000000000004",
        other => panic!("unexpected fixture host: {other}"),
    };
    id.parse().unwrap()
}
fn label(raw: &str, color: Color) -> Label {
    Label {
        name: name(raw),
        color,
    }
}
fn ticket(source: &mut Tickets) -> Ticket {
    source.issue().unwrap()
}

fn ready_catalog() -> Catalog {
    Catalog::snapshot([label("Ready", Color::Violet)])
}

#[test]
fn identity_collapses_whitespace_without_changing_host_spelling() {
    let normalized = name("  In\tProgress\n ");
    assert_eq!(normalized.as_str(), "In Progress");
    assert_eq!(normalized.key(), name("in  progress").key());
    assert_ne!(normalized, name("in progress"));
    assert!(" \t\n".parse::<Name>().is_err());
    assert_eq!(name(&"가".repeat(2048)).as_str().chars().count(), 2048);
    assert_eq!(Color::default(), Color::Violet);
    assert_eq!(name("\u{feff}Ready\u{feff}").as_str(), "Ready");
    assert_eq!(name("a\u{0085}b").as_str(), "a\u{0085}b");
}

#[test]
fn create_and_assign_is_one_request_and_a_failed_request_keeps_the_form() {
    let mut page = PickerPage::default();
    page.create().unwrap();
    assert_eq!(page.draft_mut().unwrap().color(), Color::Violet);
    let mut tickets = Tickets::default();
    let first = ticket(&mut tickets);
    assert_eq!(
        page.submit(HostState::Online, first),
        Err(Failure::NoChange)
    );
    let draft = page.draft_mut().unwrap();
    draft.set_text("  New  Label ");
    draft.set_color(Color::Orange);
    assert_eq!(
        page.submit(HostState::Offline, first),
        Err(Failure::Offline)
    );
    assert_eq!(
        page.submit(HostState::Online, first),
        Ok(label("New Label", Color::Orange))
    );
    assert_eq!(page.create(), Err(Failure::Pending));
    assert!(page.draft_mut().is_none());
    assert!(!page.completed(ticket(&mut tickets), Ok(())));
    assert!(page.completed(first, Err(Failure::Update)));
    assert_eq!(page.draft_mut().unwrap().text(), "  New  Label ");
    assert_eq!(page.draft_mut().unwrap().error(), Some(Failure::Update));
    let second = ticket(&mut tickets);
    assert_eq!(
        page.submit(HostState::Online, second),
        Ok(label("New Label", Color::Orange))
    );
    assert!(page.completed(second, Ok(())));
    assert_eq!(page, PickerPage::Listing);
}

#[test]
fn projections_choose_target_then_host_order_and_retain_connected_failed_caches() {
    let hosts = vec![
        Host {
            key: key("b"),
            route: crate::machine_book::Name::parse("b").unwrap(),
            connection: None,
            state: HostState::Online,
            catalog: Catalog::snapshot([label("ready", Color::Sky)]),
        },
        Host {
            key: key("a"),
            route: crate::machine_book::Name::parse("a").unwrap(),
            connection: None,
            state: HostState::Online,
            catalog: Catalog::snapshot([label("Ready", Color::Pink), label("alpha", Color::Blue)]),
        },
        Host {
            key: key("offline"),
            route: crate::machine_book::Name::parse("offline").unwrap(),
            connection: None,
            state: HostState::Offline,
            catalog: Catalog::snapshot([label("hidden", Color::Red)]),
        },
        Host {
            key: key("failed"),
            route: crate::machine_book::Name::parse("failed").unwrap(),
            connection: None,
            state: HostState::Failed(Failure::Load),
            catalog: Catalog::snapshot([label("failed", Color::Red)]),
        },
    ];
    assert_eq!(
        project(&hosts, None),
        vec![
            label("alpha", Color::Blue),
            label("failed", Color::Red),
            label("Ready", Color::Pink)
        ]
    );
    assert_eq!(
        project(&hosts, Some(&key("b"))),
        vec![
            label("alpha", Color::Blue),
            label("failed", Color::Red),
            label("ready", Color::Sky)
        ]
    );
    let offline = hosts
        .iter()
        .find(|host| host.key == key("offline"))
        .unwrap();
    assert!(offline.catalog.get(&name("hidden")).is_some());
}

#[test]
fn committed_catalog_changes_remove_previous_names_and_snapshot_removes_absent() {
    let mut catalog = Catalog::snapshot([label("Old", Color::Red)]);
    catalog.upsert(Some(&name("old")), label("New", Color::Teal));
    assert!(catalog.get(&name("old")).is_none());
    assert_eq!(catalog.get(&name("NEW")), Some(&label("New", Color::Teal)));
    catalog.remove(&name("missing"));
    catalog.remove(&name("new"));
    assert_eq!(catalog.labels().count(), 0);
    let replacement = Catalog::snapshot([label("only", Color::Sky)]);
    assert!(replacement.get(&name("New")).is_none());
}

#[test]
fn picker_has_one_pending_ticket_per_name_and_does_not_optimistically_assign() {
    let mut picker = Picker::default();
    let mut tickets = Tickets::default();
    let first = ticket(&mut tickets);
    let second = ticket(&mut tickets);
    let catalog = Catalog::snapshot([label("Ready", Color::Sky), label("Other", Color::Red)]);
    picker
        .begin(HostState::Online, &name("READY"), first)
        .unwrap();
    assert_eq!(
        picker.begin(HostState::Online, &name(" ready "), second),
        Err(Failure::Pending)
    );
    picker
        .begin(HostState::Online, &name("other"), second)
        .unwrap();
    let rows = picker.rows(&catalog, &[name("ready")], "READY");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows.first().unwrap().assignment, Assignment::Assigned);
    assert_eq!(rows.first().unwrap().pending, Some(first));
    assert!(!picker.finish(&name("ready"), second, Ok(())));
    assert!(picker.finish(&name("ready"), first, Err(Failure::Update)));
    assert_eq!(picker.error(), Some(Failure::Update));
    assert!(picker.finish(&name("other"), second, Ok(())));
    assert_eq!(picker.error(), Some(Failure::Update));
    assert_eq!(
        picker
            .rows(&catalog, &[name("ready")], "")
            .last()
            .unwrap()
            .assignment,
        Assignment::Assigned
    );
}

#[test]
fn every_unavailable_host_refuses_picker_before_pending() {
    for (state, failure) in [
        (HostState::Offline, Failure::Offline),
        (HostState::Loading, Failure::Load),
        (HostState::Unsupported, Failure::Unsupported),
    ] {
        let mut picker = Picker::default();
        let first = ticket(&mut Tickets::default());
        assert_eq!(picker.begin(state, &name("Ready"), first), Err(failure));
        assert!(!picker.finish(&name("Ready"), first, Ok(())));
    }
}

#[test]
fn a_catalog_load_failure_does_not_revoke_a_connected_hosts_mutation_capability() {
    let mut picker = Picker::default();
    let request = ticket(&mut Tickets::default());
    picker
        .begin(HostState::Failed(Failure::Load), &name("Ready"), request)
        .unwrap();
    assert!(picker.finish(&name("Ready"), request, Ok(())));
}

#[test]
fn save_keeps_case_only_rename_atomic_color_and_failed_draft() {
    let mut manager = Manager::default();
    manager.select(key("a")).unwrap();
    manager.search("ready");
    manager.edit(label("Ready", Color::Violet)).unwrap();
    let mut tickets = Tickets::default();
    let first = ticket(&mut tickets);
    assert_eq!(
        manager.save(HostState::Online, &ready_catalog(), first),
        Err(Failure::NoChange)
    );
    let draft = manager.draft_mut().unwrap();
    draft.set_text("  ready ");
    draft.set_color(Color::Teal);
    assert_eq!(
        manager.save(HostState::Online, &ready_catalog(), first),
        Ok(label("ready", Color::Teal))
    );
    assert_eq!(manager.select(key("b")), Err(Failure::Pending));
    assert!(manager.draft_mut().is_none());
    assert!(!manager.completed(ticket(&mut tickets), Ok(())));
    assert!(manager.completed(first, Err(Failure::NameTaken)));
    assert_eq!(manager.state().draft().unwrap().text(), "  ready ");
    assert_eq!(manager.error(), Some(Failure::NameTaken));
    assert_eq!(manager.query(), "ready");
    assert_eq!(
        manager.save(HostState::Online, &ready_catalog(), first),
        Ok(label("ready", Color::Teal))
    );
    assert!(manager.completed(first, Ok(())));
    assert_eq!(manager.state(), &ManagerState::Listing);
    manager.reset().unwrap();
    assert!(manager.query().is_empty());
}

#[test]
fn whitespace_only_draft_never_enables_save() {
    let mut draft = Draft::new(label("Ready", Color::Violet));
    draft.set_text(" \t ");
    draft.set_color(Color::Red);
    assert!(draft.replacement(&label("Ready", Color::Violet)).is_none());
}

#[test]
fn delete_requires_matching_inspection_explicit_confirmation_and_online_host() {
    let mut manager = Manager::default();
    manager.select(key("a")).unwrap();
    manager.edit(label("Ready", Color::Violet)).unwrap();
    manager.draft_mut().unwrap().set_text("Unsaved replacement");
    let mut tickets = Tickets::default();
    let inspect = ticket(&mut tickets);
    let remove = ticket(&mut tickets);
    assert_eq!(
        manager.delete(HostState::Online, remove),
        Err(Failure::NoChange)
    );
    assert_eq!(
        manager.inspect(HostState::Online, &ready_catalog(), inspect),
        Ok(name("Ready"))
    );
    assert_eq!(manager.reset(), Err(Failure::Pending));
    assert!(!manager.inspected(remove, Ok(Count::new(9))));
    assert!(manager.inspected(inspect, Ok(Count::new(2))));
    assert_eq!(
        manager.state().draft().unwrap().text(),
        "Unsaved replacement"
    );
    assert_eq!(
        manager.state().draft().unwrap().original().name,
        name("Ready")
    );
    assert!(matches!(
        manager.state(),
        ManagerState::Confirming { count, .. } if count.get() == 2
    ));
    assert_eq!(
        manager.delete(HostState::Offline, remove),
        Err(Failure::Offline)
    );
    assert!(manager.cancel_delete());
    assert_eq!(
        manager.inspect(HostState::Online, &ready_catalog(), inspect),
        Ok(name("Ready"))
    );
    assert!(manager.inspected(inspect, Ok(Count::new(0))));
    assert_eq!(manager.delete(HostState::Online, remove), Ok(name("Ready")));
    assert!(!manager.completed(inspect, Ok(())));
    assert!(manager.completed(remove, Err(Failure::Update)));
    assert_eq!(
        manager.state().draft().unwrap().original().name,
        name("Ready")
    );
}

#[test]
fn inspection_failure_keeps_draft_and_host_switch_clears_idle_edit() {
    let mut manager = Manager::default();
    manager.select(key("a")).unwrap();
    manager.edit(label("Ready", Color::Sky)).unwrap();
    let inspect = ticket(&mut Tickets::default());
    assert_eq!(
        manager.inspect(HostState::Online, &ready_catalog(), inspect),
        Ok(name("Ready"))
    );
    assert!(manager.inspected(inspect, Err(Failure::Load)));
    assert_eq!(manager.error(), Some(Failure::Load));
    assert!(matches!(manager.state(), ManagerState::Editing(_)));
    manager.select(key("b")).unwrap();
    assert_eq!(manager.state(), &ManagerState::Listing);
    assert_eq!(manager.error(), None);
}

#[test]
fn edit_dirty_state_follows_current_catalog_and_removed_targets_cannot_be_saved() {
    let mut manager = Manager::default();
    manager.edit(label("Ready", Color::Violet)).unwrap();
    let ticket = ticket(&mut Tickets::default());
    let recolored = Catalog::snapshot([label("Ready", Color::Sky)]);
    assert_eq!(
        manager.save(HostState::Online, &recolored, ticket),
        Ok(label("Ready", Color::Violet))
    );
    assert!(manager.completed(ticket, Err(Failure::Update)));
    assert_eq!(
        manager.save(HostState::Online, &Catalog::default(), ticket),
        Err(Failure::NotFound)
    );
    assert_eq!(
        manager.inspect(HostState::Online, &Catalog::default(), ticket),
        Err(Failure::NotFound)
    );
    assert!(matches!(manager.state(), ManagerState::Editing(_)));
}

#[test]
fn field_policy_is_read_only_for_unavailable_hosts_and_every_async_phase() {
    let mut manager = Manager::default();
    assert_eq!(manager.edit_access(HostState::Online), EditAccess::ReadOnly);
    manager.edit(label("Ready", Color::Violet)).unwrap();
    for host in [
        HostState::Offline,
        HostState::Loading,
        HostState::Unsupported,
    ] {
        assert_eq!(manager.edit_access(host), EditAccess::ReadOnly);
    }
    assert_eq!(manager.edit_access(HostState::Online), EditAccess::Editable);
    assert_eq!(
        manager.edit_access(HostState::Failed(Failure::Load)),
        EditAccess::Editable
    );
    manager.draft_mut().unwrap().set_color(Color::Sky);
    let ticket = ticket(&mut Tickets::default());
    assert_eq!(
        manager.save(HostState::Online, &ready_catalog(), ticket),
        Ok(label("Ready", Color::Sky))
    );
    assert_eq!(manager.edit_access(HostState::Online), EditAccess::ReadOnly);
    assert!(manager.completed(ticket, Err(Failure::Update)));
    assert_eq!(manager.edit_access(HostState::Online), EditAccess::Editable);
    assert_eq!(
        manager.inspect(HostState::Online, &ready_catalog(), ticket),
        Ok(name("Ready"))
    );
    assert_eq!(manager.edit_access(HostState::Online), EditAccess::ReadOnly);
    assert!(manager.inspected(ticket, Ok(Count::new(1))));
    assert_eq!(manager.edit_access(HostState::Online), EditAccess::ReadOnly);
    assert_eq!(manager.delete(HostState::Online, ticket), Ok(name("Ready")));
    assert_eq!(manager.edit_access(HostState::Online), EditAccess::ReadOnly);
    assert!(manager.completed(ticket, Ok(())));
    assert_eq!(manager.edit_access(HostState::Online), EditAccess::ReadOnly);
}

#[test]
fn removed_host_does_not_retarget_a_pending_write_and_idle_sync_preserves_search() {
    let host = |raw| Host {
        key: key(raw),
        route: crate::machine_book::Name::parse(raw).unwrap(),
        connection: None,
        state: HostState::Online,
        catalog: ready_catalog(),
    };
    let mut manager = Manager::default();
    manager.sync_hosts(&[host("a")]);
    manager.search("ready");
    manager.edit(label("Ready", Color::Violet)).unwrap();
    manager.draft_mut().unwrap().set_color(Color::Sky);
    let ticket = ticket(&mut Tickets::default());
    assert_eq!(
        manager.save(HostState::Online, &ready_catalog(), ticket),
        Ok(label("Ready", Color::Sky))
    );
    manager.sync_hosts(&[host("b")]);
    assert_eq!(manager.selected(), Some(&key("a")));
    assert!(matches!(manager.state(), ManagerState::Saving { .. }));
    assert!(manager.completed(ticket, Err(Failure::Offline)));
    manager.sync_hosts(&[host("b")]);
    assert_eq!(manager.selected(), Some(&key("b")));
    assert_eq!(manager.state(), &ManagerState::Listing);
    assert_eq!(manager.query(), "ready");
}
