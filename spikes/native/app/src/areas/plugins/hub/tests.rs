use super::*;
use vessel_proto::plugins::ClientCatalog;

fn ticket(command: Command) -> CatalogTicket {
    let Command::ReadCatalog(ticket) = command;
    ticket
}
fn reply(ticket: CatalogTicket) -> Event {
    Event::CatalogRead {
        ticket,
        result: Ok(ClientCatalog::new(ticket.node(), vec![]).expect("canonical catalog")),
    }
}
fn complete(hub: &mut Hub, ticket: CatalogTicket) -> Option<Change> {
    match hub.accept(reply(ticket)) {
        Acceptance::Settled(change) => change,
        Acceptance::Deferred(_) => panic!("no registry reader in this fixture"),
    }
}

#[test]
fn repeated_invalidations_coalesce_and_an_offline_host_cannot_rearm_itself() {
    let mut hub = Hub::new();
    let node = NodeId::random();
    assert_eq!(
        hub.connected(node, HostSupport::Supported),
        Ok(Change::Eligibility(node))
    );
    for _ in 0..8 {
        hub.invalidate(node)
            .expect("queued read observes latest facts");
    }
    assert_eq!(hub.queued.len(), 1);
    assert!(hub.dirty.is_empty());
    let first = ticket(hub.take_reads().pop().expect("one command"));
    for _ in 0..8 {
        hub.invalidate(node).expect("coalesced active invalidation");
    }
    assert_eq!(hub.dirty.len(), 1);
    assert_eq!(complete(&mut hub, first), Some(Change::Catalog(node)));
    assert_eq!(hub.queued.len(), 1);
    assert!(hub.dirty.is_empty());
    let second = ticket(hub.take_reads().pop().expect("one refresh"));
    assert_eq!(
        complete(&mut hub, first),
        None,
        "duplicate settlement is inert"
    );
    assert_eq!(hub.active, vec![second]);
    assert_eq!(complete(&mut hub, second), Some(Change::Catalog(node)));
    assert_eq!(hub.offline(node), Ok(Change::Eligibility(node)));
    assert_eq!(hub.invalidate(node), Err(Problem::Offline));
    assert!(hub.take_reads().is_empty());
    assert!(
        hub.registry()
            .expect("read-only registry")
            .installed(node)
            .is_some()
    );
    assert_eq!(hub.forget(node), Ok(Change::Eligibility(node)));
    assert_eq!(hub.invalidate(node), Err(Problem::Stale));
    assert!(
        hub.registry()
            .expect("read-only registry")
            .installed(node)
            .is_none()
    );
}

#[test]
fn reconnect_and_forget_retain_active_admission_until_actual_settlement() {
    let mut hub = Hub::new();
    let nodes: Vec<_> = (0..MAX_CONCURRENT_CATALOG_READS.saturating_add(1))
        .map(|_| NodeId::random())
        .collect();
    for node in &nodes {
        assert_eq!(
            hub.connected(*node, HostSupport::Supported),
            Ok(Change::Eligibility(*node))
        );
    }
    let commands = hub.take_reads();
    assert_eq!(commands.len(), MAX_CONCURRENT_CATALOG_READS);
    assert_eq!(hub.queued.len(), 1);
    let old = ticket(commands[0]);
    assert_eq!(
        hub.connected(old.node(), HostSupport::Supported),
        Ok(Change::Eligibility(old.node()))
    );
    assert!(
        hub.take_reads().is_empty(),
        "reconnect does not release old active work"
    );
    assert_eq!(hub.active.len(), MAX_CONCURRENT_CATALOG_READS);
    assert_eq!(
        complete(&mut hub, old),
        None,
        "old connection's publication is stale"
    );
    let next = ticket(hub.take_reads().pop().expect("one freed slot"));
    assert_eq!(next.node(), *nodes.last().expect("queued node"));
    assert_eq!(
        hub.forget(next.node()),
        Ok(Change::Eligibility(next.node()))
    );
    assert!(
        hub.take_reads().is_empty(),
        "forget does not release a live source slot"
    );
    assert_eq!(complete(&mut hub, next), None);
    let fresh = ticket(hub.take_reads().pop().expect("new connection now admitted"));
    assert_eq!(fresh.node(), old.node());
    assert_ne!(fresh, old);
    assert_eq!(complete(&mut hub, fresh), Some(Change::Catalog(old.node())));
}

#[test]
fn a_busy_registry_returns_the_owned_completion_without_retiring_admission() {
    let mut hub = Hub::new();
    let node = NodeId::random();
    assert_eq!(
        hub.connected(node, HostSupport::Supported),
        Ok(Change::Eligibility(node))
    );
    let pending = ticket(hub.take_reads().pop().expect("one read"));
    let registry = Rc::clone(&hub.registry);
    let guard = registry.borrow();
    let Acceptance::Deferred(event) = hub.accept(reply(pending)) else {
        panic!("completion must remain owned by the caller");
    };
    assert_eq!(hub.active, vec![pending]);
    drop(guard);
    assert!(
        matches!(hub.accept(event), Acceptance::Settled(Some(Change::Catalog(owner))) if owner == node)
    );
    assert!(hub.active.is_empty());
}

#[tokio::test]
async fn the_honest_unavailable_source_uses_the_same_canonical_link_completion() {
    let mut hub = Hub::new();
    let node = NodeId::random();
    assert_eq!(
        hub.connected(node, HostSupport::Supported),
        Ok(Change::Eligibility(node))
    );
    let command = hub.take_reads().pop().expect("one read");
    let event = command
        .run(&vessel_app_link::plugins::UnavailableNode)
        .await;
    assert!(
        matches!(hub.accept(event), Acceptance::Settled(Some(Change::Failure { node: owner, problem: Problem::Unsupported })) if owner == node)
    );
    assert!(hub.active.is_empty());
    assert_eq!(
        hub.registry().expect("registry").failure(node),
        Some(Problem::Unsupported)
    );
    assert_eq!(
        hub.connected(node, HostSupport::Unsupported),
        Ok(Change::Failure {
            node,
            problem: Problem::Unsupported
        })
    );
    assert!(hub.registry().expect("registry").installed(node).is_none());
}
