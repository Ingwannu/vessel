//! A lost acknowledgement is settled by discovery, never by repeating removal.
use super::*;

#[test]
fn uncertain_result_retains_facts_and_selection_but_requires_successful_discovery() {
    let mut pages = pages();
    pages.select("one", Some(agent()));
    let retained = Confirmation::new(&pages, machine(), &agent()).unwrap();
    let request = Confirmation::new(&pages, machine(), &agent())
        .unwrap()
        .confirm(&machine(), &mut pages)
        .unwrap();
    assert!(pages.removal_uncertain(&machine(), request.ticket, "lost acknowledgement"));
    let page = pages.get("one").unwrap();
    assert_eq!(page.removal_freshness(), RemovalFreshness::RefreshRequired);
    assert_eq!(page.selected(), Some(&agent()));
    assert_eq!(page.inventory().unwrap().agents.len(), 1);
    assert!(page.pending().is_none());
    assert_eq!(page.issue().unwrap().source, "lost acknowledgement");
    assert!(matches!(
        Confirmation::new(&pages, machine(), &agent()),
        Err(Refused::RefreshRequired)
    ));
    assert!(matches!(
        retained.confirm(&machine(), &mut pages),
        Err(Refused::RefreshRequired)
    ));
    let ticket = pages.begin(machine(), Operation::Discovery).unwrap();
    pages.failed(&machine(), ticket, "read failed");
    assert!(matches!(
        Confirmation::new(&pages, machine(), &agent()),
        Err(Refused::RefreshRequired)
    ));
    let ticket = pages.begin(machine(), Operation::Check(agent())).unwrap();
    pages.checked(&machine(), ticket, Vec::new());
    assert!(matches!(
        Confirmation::new(&pages, machine(), &agent()),
        Err(Refused::RefreshRequired)
    ));
    let ticket = pages.begin(machine(), Operation::Discovery).unwrap();
    assert!(!pages.loaded(&machine(), request.ticket, inventory()));
    assert_eq!(
        pages.get("one").unwrap().removal_freshness(),
        RemovalFreshness::RefreshRequired
    );
    assert!(pages.loaded(&machine(), ticket, inventory()));
    assert_eq!(
        pages.get("one").unwrap().removal_freshness(),
        RemovalFreshness::Current
    );
    assert!(Confirmation::new(&pages, machine(), &agent()).is_ok());
}

#[test]
fn a_successful_refresh_that_confirms_absence_cannot_mint_another_removal() {
    let mut pages = pages();
    pages.select("one", Some(agent()));
    let request = Confirmation::new(&pages, machine(), &agent())
        .unwrap()
        .confirm(&machine(), &mut pages)
        .unwrap();
    assert!(pages.removal_uncertain(&machine(), request.ticket, "timeout"));
    let mut absent = inventory();
    absent.agents.clear();
    absent.harnesses.clear();
    absent.readiness.clear();
    load(&mut pages, &machine(), absent);
    assert!(pages.get("one").unwrap().selected().is_none());
    assert!(matches!(
        Confirmation::new(&pages, machine(), &agent()),
        Err(Refused::Unavailable)
    ));
    assert!(!pages.removed(&machine(), request.ticket, request.node, &request.name));
}

#[test]
fn stale_or_unrelated_uncertainty_cannot_block_another_request() {
    let mut pages = pages();
    let ticket = pages.begin(machine(), Operation::Check(agent())).unwrap();
    assert!(!pages.removal_uncertain(&machine(), ticket, "wrong operation"));
    pages.checked(&machine(), ticket, Vec::new());
    let request = Confirmation::new(&pages, machine(), &agent())
        .unwrap()
        .confirm(&machine(), &mut pages)
        .unwrap();
    assert!(!pages.removal_uncertain(
        &MachineKey::from_book("other"),
        request.ticket,
        "other machine"
    ));
    assert!(!pages.removal_uncertain(&machine(), ticket, "stale"));
    assert_eq!(
        pages.get("one").unwrap().removal_freshness(),
        RemovalFreshness::Current
    );
    assert!(pages.removed(&machine(), request.ticket, request.node, &request.name));
}
