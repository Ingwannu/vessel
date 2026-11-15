use super::*;
use crate::agent_status::{command::CommandShell, pages::Inventory};
use vessel_proto::agents::{AddedAgent, Catalogue, CatalogueRun};

type TestResult<T> = Result<T, Box<dyn std::error::Error>>;
fn setup() -> TestResult<(Pages<&'static str>, MachineKey, NodeId)> {
    let mut pages = Pages::default();
    let machine = MachineKey::from_book("fixture");
    let node = "00000000-0000-4000-8000-000000000001".parse()?;
    let ticket = pages.begin(machine.clone(), Operation::Discovery)?;
    pages.loaded(
        &machine,
        ticket,
        Inventory {
            node,
            command_shell: CommandShell::Unix,
            harnesses: Vec::new(),
            agents: Vec::new(),
            readiness: Vec::new(),
            administration: Administration::Supported,
        },
    );
    Ok((pages, machine, node))
}
fn registry() -> TestResult<CatalogueEntry> {
    Ok(CatalogueEntry {
        id: "provider-with-a-registry-id-that-is-longer-than-the-agent-name-limit-abc".parse()?,
        name: "Provider".into(),
        version: "1.0.0".parse()?,
        description: String::new(),
        website: None,
        repository: None,
        license: None,
        runs: CatalogueRun::Command {
            program: "fake-npm".parse()?,
            args: vec!["exact-package@1.0.0".into()],
            env: Vec::new(),
            runner_found: false,
        },
    })
}
fn set_catalogue(
    pages: &mut Pages<&str>,
    machine: &MachineKey,
    node: NodeId,
    entry: CatalogueEntry,
) -> TestResult<()> {
    let ticket = pages.begin(machine.clone(), Operation::Catalogue { node })?;
    if !pages.catalogue_loaded(
        machine,
        ticket,
        node,
        Catalogue {
            fetched_at: Some(chrono::DateTime::UNIX_EPOCH),
            entries: vec![entry],
            ..Catalogue::default()
        },
    ) {
        return Err(std::io::Error::other("fixture catalogue was not accepted").into());
    }
    Ok(())
}
fn ack(call: &AddAgent) -> TestResult<AgentInfo> {
    let (program, args, source) = match &call.agent {
        AgentToAdd::Command { definition } => (
            definition.program.clone(),
            definition.args.clone(),
            AgentSource::Command,
        ),
        AgentToAdd::Registry { id, version, .. } => (
            "fake-npm".parse()?,
            Vec::new(),
            AgentSource::Registry {
                id: id.clone(),
                version: version.clone(),
            },
        ),
    };
    Ok(AgentInfo {
        agent: HarnessRef::Added(call.name.clone()),
        detail: AgentDetail::Added {
            added: AddedAgent {
                label: None,
                program,
                args,
                env: Vec::new(),
                secrets: Vec::new(),
                inherit: Vec::new(),
                mcp: Vec::new(),
                source,
            },
        },
    })
}
#[test]
fn command_arguments_are_literal_and_the_call_never_replaces() {
    let (mut pages, machine, node) = setup().unwrap();
    let intent = Intent::command(&pages, machine.clone()).unwrap();
    let request = intent
        .submit(
            &machine,
            &mut pages,
            "custom-agent",
            "fake-acp",
            "--name\na b\n$(never-run)\n\"quoted\"",
        )
        .unwrap();
    let (route, expected, call, ticket) = request.into_parts();
    assert_eq!((route, expected), (machine.clone(), node));
    assert!(!call.replace);
    let AgentToAdd::Command { definition } = &call.agent else {
        panic!("expected command")
    };
    assert_eq!(
        definition.args,
        ["--name", "a b", "$(never-run)", "\"quoted\""]
    );
    assert!(
        definition.env.is_empty() && definition.inherit.is_empty() && definition.mcp.is_empty()
    );
    assert!(matches!(
        intent.submit(&machine, &mut pages, "another", "fake", ""),
        Err(Refused::Request(StartProblem::Busy))
    ));
    assert!(pages.added(&machine, ticket, node, ack(&call).unwrap()));
    let inventory = pages.get(machine.as_str()).unwrap().inventory().unwrap();
    assert!(inventory.harnesses.is_empty() && inventory.readiness.is_empty());
    assert!(pages.get(machine.as_str()).unwrap().pending().is_none());
    assert_eq!(inventory.agents.len(), 1);
    assert!(matches!(
        intent.submit(&machine, &mut pages, "CUSTOM-AGENT", "fake", ""),
        Err(Refused::Taken)
    ));
}
#[test]
fn invalid_fields_reserve_no_ticket_and_do_not_change_the_page() {
    let (mut pages, machine, _) = setup().unwrap();
    let intent = Intent::command(&pages, machine.clone()).unwrap();
    for name in ["", "has spaces", "codex", &"a".repeat(65)] {
        assert!(matches!(
            intent.submit(&machine, &mut pages, name, "fake", ""),
            Err(Refused::Name(_))
        ));
        assert!(pages.get(machine.as_str()).unwrap().pending().is_none());
    }
    assert!(matches!(
        intent.submit(&machine, &mut pages, "good", "", ""),
        Err(Refused::Program(_))
    ));
    assert!(
        pages
            .get(machine.as_str())
            .unwrap()
            .inventory()
            .unwrap()
            .agents
            .is_empty()
    );
}
#[test]
fn registry_pin_and_editable_name_are_independent_of_runner_availability() {
    let (mut pages, machine, node) = setup().unwrap();
    let entry = registry().unwrap();
    set_catalogue(&mut pages, &machine, node, entry.clone()).unwrap();
    let intent = Intent::registry(&pages, machine.clone(), &entry).unwrap();
    assert!(matches!(
        intent.submit(&machine, &mut pages, entry.id.as_str(), "", ""),
        Err(Refused::Name(_))
    ));
    let (_, _, call, _) = intent
        .submit(&machine, &mut pages, "my-provider", "ignored", "ignored")
        .unwrap()
        .into_parts();
    assert_eq!(
        call.agent,
        AgentToAdd::Registry {
            id: entry.id,
            version: entry.version,
            mcp: Vec::new()
        }
    );
    assert!(!call.replace);
}
#[test]
fn changed_machine_node_and_registry_entry_refuse_original_intent() {
    let (mut pages, machine, node) = setup().unwrap();
    let entry = registry().unwrap();
    set_catalogue(&mut pages, &machine, node, entry.clone()).unwrap();
    let intent = Intent::registry(&pages, machine.clone(), &entry).unwrap();
    assert!(matches!(
        intent.submit(
            &MachineKey::from_book("other"),
            &mut pages,
            "custom",
            "",
            ""
        ),
        Err(Refused::MachineChanged)
    ));
    let mut newer = entry;
    newer.version = "2.0.0".parse().unwrap();
    set_catalogue(&mut pages, &machine, node, newer).unwrap();
    assert!(matches!(
        intent.submit(&machine, &mut pages, "custom", "", ""),
        Err(Refused::Changed)
    ));
    let original = Intent::command(&pages, machine.clone()).unwrap();
    let other = "00000000-0000-4000-8000-000000000002".parse().unwrap();
    assert!(pages.connected(&machine, other));
    let ticket = pages.begin(machine.clone(), Operation::Discovery).unwrap();
    pages.loaded(
        &machine,
        ticket,
        Inventory {
            node: other,
            command_shell: CommandShell::Unix,
            harnesses: Vec::new(),
            agents: Vec::new(),
            readiness: Vec::new(),
            administration: Administration::Supported,
        },
    );
    assert!(matches!(
        original.submit(&machine, &mut pages, "custom", "fake", ""),
        Err(Refused::Changed)
    ));
}
#[test]
fn ack_requires_the_original_book_ticket_node_name_and_source() {
    let (mut pages, machine, node) = setup().unwrap();
    let entry = registry().unwrap();
    set_catalogue(&mut pages, &machine, node, entry.clone()).unwrap();
    let intent = Intent::registry(&pages, machine.clone(), &entry).unwrap();
    let (_, _, call, ticket) = intent
        .submit(&machine, &mut pages, "custom", "", "")
        .unwrap()
        .into_parts();
    let info = ack(&call).unwrap();
    let other = "00000000-0000-4000-8000-000000000002".parse().unwrap();
    assert!(!pages.added(&MachineKey::from_book("other"), ticket, node, info.clone()));
    assert!(!pages.added(&machine, ticket.next().unwrap(), node, info.clone()));
    assert!(!pages.added(&machine, ticket, other, info.clone()));
    let mut wrong = info.clone();
    wrong.agent = HarnessRef::Added("wrong".parse().unwrap());
    assert!(!pages.added(&machine, ticket, node, wrong));
    let mut wrong = info.clone();
    let AgentDetail::Added { added } = &mut wrong.detail else {
        panic!("added")
    };
    added.source = AgentSource::Command;
    assert!(!pages.added(&machine, ticket, node, wrong));
    assert!(pages.added(&machine, ticket, node, info.clone()));
    assert!(!pages.added(&machine, ticket, node, info));
    assert!(matches!(
        catalogue::offer(
            &entry,
            &pages
                .get(machine.as_str())
                .unwrap()
                .inventory()
                .unwrap()
                .agents
        ),
        Offer::Recorded(_)
    ));
}
#[test]
fn uncertain_add_preserves_facts_but_requires_discovery_before_retry() {
    let (mut pages, machine, _) = setup().unwrap();
    let intent = Intent::command(&pages, machine.clone()).unwrap();
    let (_, _, _, ticket) = intent
        .submit(&machine, &mut pages, "custom", "fake", "")
        .unwrap()
        .into_parts();
    assert!(pages.addition_uncertain(&machine, ticket, "lost ACK"));
    assert!(
        pages
            .get(machine.as_str())
            .unwrap()
            .inventory()
            .unwrap()
            .agents
            .is_empty()
    );
    assert!(matches!(
        Intent::command(&pages, machine.clone()),
        Err(Refused::Unavailable)
    ));
    assert_eq!(
        pages.get(machine.as_str()).unwrap().issue().unwrap().source,
        "lost ACK"
    );
}

#[test]
fn an_unfetched_copy_does_not_authorize_a_registry_addition() {
    let (mut pages, machine, node) = setup().unwrap();
    let entry = registry().unwrap();
    let ticket = pages
        .begin(machine.clone(), Operation::Catalogue { node })
        .unwrap();
    assert!(pages.catalogue_loaded(
        &machine,
        ticket,
        node,
        Catalogue {
            fetched_at: None,
            entries: vec![entry.clone()],
            ..Catalogue::default()
        }
    ));
    assert!(matches!(
        Intent::registry(&pages, machine.clone(), &entry),
        Err(Refused::Unavailable)
    ));
    assert!(pages.get(machine.as_str()).unwrap().pending().is_none());
}
#[test]
fn a_registry_ack_keeps_the_exact_id_and_version_not_just_the_name() {
    let (mut pages, machine, node) = setup().unwrap();
    let entry = registry().unwrap();
    set_catalogue(&mut pages, &machine, node, entry.clone()).unwrap();
    let intent = Intent::registry(&pages, machine.clone(), &entry).unwrap();
    let (_, _, call, ticket) = intent
        .submit(&machine, &mut pages, "custom", "", "")
        .unwrap()
        .into_parts();
    for source in [
        AgentSource::Registry {
            id: "another-provider".parse().unwrap(),
            version: entry.version.clone(),
        },
        AgentSource::Registry {
            id: entry.id.clone(),
            version: "2.0.0".parse().unwrap(),
        },
    ] {
        let mut info = ack(&call).unwrap();
        let AgentDetail::Added { added } = &mut info.detail else {
            panic!("fixture is an added definition")
        };
        added.source = source;
        assert!(!pages.added(&machine, ticket, node, info));
        assert_eq!(
            pages.get(machine.as_str()).unwrap().pending().unwrap().id,
            ticket
        );
    }
    assert!(pages.added(&machine, ticket, node, ack(&call).unwrap()));
}

#[test]
fn suggested_registry_names_keep_the_whole_valid_id_and_never_truncate() {
    let mut entry = registry().unwrap();
    assert!(suggested_name(&entry).is_none());
    entry.id = "my-provider".parse().unwrap();
    assert_eq!(suggested_name(&entry).unwrap().as_str(), "my-provider");
    for kind in vessel_proto::HarnessKind::ALL {
        entry.id = kind.name().parse().unwrap();
        assert!(suggested_name(&entry).is_none());
    }
}
