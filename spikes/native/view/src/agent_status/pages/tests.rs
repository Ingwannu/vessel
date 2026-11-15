//! Adversarial request order and machine switches, without a UI or node.

use super::*;
use vessel_proto::agents::{InstallPart, InstallState};
use vessel_proto::{HarnessKind, TerminalBytes};

pub(super) fn key(name: &'static str) -> MachineKey {
    MachineKey::from_book(name)
}
pub(super) fn inventory(administration: Administration) -> Inventory {
    Inventory {
        node: "00000000-0000-4000-8000-000000000001".parse().unwrap(),
        command_shell: super::super::command::CommandShell::Unix,
        harnesses: Vec::new(),
        agents: Vec::new(),
        readiness: Vec::new(),
        administration,
    }
}
pub(super) fn install() -> InstallAgent {
    InstallAgent {
        agent: HarnessKind::Codex,
        part: InstallPart::Adapter,
    }
}
pub(super) fn job() -> InstallJob {
    InstallJob {
        id: "00000000-0000-4000-8000-000000000001".parse().unwrap(),
        agent: HarnessKind::Codex,
        part: InstallPart::Adapter,
        command: "fake-npm".to_owned(),
        started_at: "2026-11-13T00:00:00Z".parse().unwrap(),
    }
}
pub(super) fn output() -> InstallOutput {
    InstallOutput {
        from: 0,
        bytes: TerminalBytes::new("output".into()),
        next: 6,
        truncated: false,
        state: InstallState::Running,
    }
}

#[test]
fn replies_cannot_cross_machines_or_replace_a_newer_request() {
    let mut pages = Pages::<&str>::default();
    let a = key("one");
    let b = key("two");
    let first = pages.begin(a.clone(), Operation::Discovery).unwrap();
    let other = pages.begin(b.clone(), Operation::Discovery).unwrap();
    assert!(pages.loaded(&b, other, inventory(Administration::OlderNode)));
    assert!(pages.get("one").unwrap().inventory().is_none());
    pages.failed(&a, first, "disconnected");
    let next = pages.begin(a.clone(), Operation::Discovery).unwrap();
    assert!(!pages.loaded(&a, first, inventory(Administration::OlderNode)));
    assert!(pages.get("one").unwrap().inventory().is_none());
    assert_eq!(pages.get("one").unwrap().pending().unwrap().id, next);
    assert!(pages.loaded(&a, next, inventory(Administration::Supported)));
    assert_eq!(
        pages
            .get("one")
            .unwrap()
            .inventory()
            .unwrap()
            .administration,
        Administration::Supported
    );
    assert_eq!(
        pages
            .get("two")
            .unwrap()
            .inventory()
            .unwrap()
            .administration,
        Administration::OlderNode
    );
}

#[test]
fn switching_machines_does_not_lose_a_job_or_its_later_failure_output() {
    let mut pages = Pages::<&str>::default();
    let a = key("one");
    let b = key("two");
    let active = pages
        .begin(a.clone(), Operation::Install(install()))
        .unwrap();
    pages.installing(&a, active, job());
    let other = pages.begin(b.clone(), Operation::Discovery).unwrap();
    assert!(pages.loaded(&b, other, inventory(Administration::Supported)));
    pages.output(&a, active, &output());
    assert!(pages.finished(&a, active, Completion::Failed(Some(7))));
    assert!(!pages.finished(&a, active, Completion::Succeeded));
    let InstallProgress::Finished { log, completion } = pages.get("one").unwrap().install() else {
        panic!("finished on its own machine")
    };
    assert_eq!(*completion, Completion::Failed(Some(7)));
    assert_eq!(log.transcript.text(), "output");
    assert!(pages.get("two").unwrap().inventory().is_some());
    assert!(pages.get("one").unwrap().pending().is_none());
    // A subsequent metadata failure must not delete an already completed log.
    let refresh = pages.begin(a.clone(), Operation::Discovery).unwrap();
    pages.failed(&a, refresh, "offline");
    assert!(matches!(
        pages.get("one").unwrap().install(),
        InstallProgress::Finished { .. }
    ));
}

#[test]
fn an_output_read_error_is_not_a_failed_installer_exit_and_stale_pages_are_ignored() {
    let mut pages = Pages::<&str>::default();
    let a = key("one");
    let active = pages
        .begin(a.clone(), Operation::Install(install()))
        .unwrap();
    pages.installing(&a, active, job());
    assert_eq!(
        super::super::notices::installation(pages.get("one").unwrap().install()),
        Some(crate::alert::Variant::Info)
    );
    pages.output(&a, active, &output());
    pages.failed(&a, active, "output connection lost");
    let page = pages.get("one").unwrap();
    assert!(matches!(page.install(), InstallProgress::Unavailable(_)));
    assert_eq!(
        super::super::notices::installation(page.install()),
        Some(crate::alert::Variant::Warning)
    );
    assert_eq!(page.issue().unwrap().source, "output connection lost");
    let new = pages
        .begin(a.clone(), Operation::Install(install()))
        .unwrap();
    pages.installing(&a, new, job());
    pages.output(&a, active, &output());
    let InstallProgress::Following(log) = pages.get("one").unwrap().install() else {
        panic!("new job")
    };
    assert!(log.transcript.is_empty());
    pages.failed(&a, active, "stale failure");
    assert!(pages.get("one").unwrap().issue().is_none());
}

#[test]
fn capacity_evicts_only_idle_pages_and_busy_does_not_consume_a_ticket() {
    let mut pages = Pages::<&str>::default();
    let a = key("first");
    let first = pages.begin(a.clone(), Operation::Discovery).unwrap();
    assert_eq!(
        pages.begin(a.clone(), Operation::Discovery),
        Err(StartProblem::Busy)
    );
    for index in 1..MAX_PAGES {
        let _ticket = pages
            .begin(
                MachineKey::from_book(format!("node-{index}")),
                Operation::Discovery,
            )
            .unwrap();
    }
    assert_eq!(pages.next.get(), u64::try_from(MAX_PAGES).unwrap());
    assert_eq!(
        pages.begin(key("extra"), Operation::Discovery),
        Err(StartProblem::Capacity)
    );
    pages.failed(&a, first, "offline");
    let _ticket = pages.begin(key("extra"), Operation::Discovery).unwrap();
    assert_eq!(pages.entries.len(), MAX_PAGES);
    assert!(pages.get("first").is_none());
    assert!(pages.get("node-1").unwrap().pending().is_some());
}

#[test]
fn a_reply_of_the_wrong_operation_cannot_complete_a_check_and_ids_never_wrap() {
    let mut pages = Pages::<&str>::default();
    let a = key("one");
    let agent = HarnessRef::Known(HarnessKind::Codex);
    let request = pages
        .begin(a.clone(), Operation::Check(agent.clone()))
        .unwrap();
    assert!(!pages.loaded(&a, request, inventory(Administration::Supported)));
    assert!(pages.get("one").unwrap().inventory().is_none());
    assert!(pages.get("one").unwrap().pending().is_some());
    pages.select("one", Some(agent.clone()));
    assert_eq!(pages.get("one").unwrap().selected(), Some(&agent));
    pages.checked(&a, request, Vec::new());
    assert!(pages.get("one").unwrap().pending().is_none());
    pages.next = RequestId::new(u64::MAX);
    assert_eq!(
        pages.begin(a, Operation::Discovery),
        Err(StartProblem::Exhausted)
    );
}

#[test]
fn start_refusals_have_complete_bilingual_instructions_and_notice_variants() {
    use crate::{Lang, alert::Variant};
    for (problem, variant, en, ko) in [
        (
            StartProblem::Busy,
            Variant::Warning,
            "Wait for this machine's current operation to finish.",
            "이 머신의 현재 작업이 끝날 때까지 기다리세요.",
        ),
        (
            StartProblem::Capacity,
            Variant::Warning,
            "Settings is still working on other machines. Try again shortly.",
            "다른 머신의 설정 작업이 진행 중입니다. 잠시 후 다시 시도하세요.",
        ),
        (
            StartProblem::Exhausted,
            Variant::Error,
            "Close and reopen Vessel, then try again.",
            "Vessel을 닫고 다시 연 뒤 시도하세요.",
        ),
    ] {
        assert_eq!(problem.variant(), variant);
        assert_eq!(Lang::En.agents().start_problem(problem), en);
        assert_eq!(Lang::Ko.agents().start_problem(problem), ko);
    }
}

#[test]
fn initial_discovery_stops_on_a_failed_reply_and_other_operations_are_not_list_loading() {
    assert!(!Page::<&str>::default().initial_discovery());
    let mut pages = Pages::<&str>::default();
    let machine = key("one");
    let request = pages.begin(machine.clone(), Operation::Discovery).unwrap();
    assert!(pages.get("one").unwrap().initial_discovery());
    pages.failed(&machine, request, "disconnected");
    assert!(!pages.get("one").unwrap().initial_discovery());
    let check = pages
        .begin(
            machine,
            Operation::Check(HarnessRef::Known(HarnessKind::Codex)),
        )
        .unwrap();
    assert_eq!(pages.get("one").unwrap().pending().unwrap().id, check);
    assert!(!pages.get("one").unwrap().initial_discovery());
}
#[test]
fn a_cached_refresh_keeps_the_rows_instead_of_replacing_them_with_placeholders() {
    let mut pages = Pages::<&str>::default();
    let machine = key("one");
    let request = pages.begin(machine.clone(), Operation::Discovery).unwrap();
    assert!(pages.loaded(&machine, request, inventory(Administration::Supported)));
    let refresh = pages.begin(machine, Operation::Discovery).unwrap();
    assert_eq!(pages.get("one").unwrap().pending().unwrap().id, refresh);
    assert!(!pages.get("one").unwrap().initial_discovery());
    assert!(pages.get("one").unwrap().inventory().is_some());
}
