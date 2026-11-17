//! Paseo's import sheet through the production shell: host choice, parallel
//! provider reads, one-press import, the filter, failures and stale controls.
use super::intent::{Action, listener};
use super::*;
use gpui::{Modifiers, TestAppContext, VisualTestContext, px, size};
use tokio::sync::mpsc::Receiver;
use vessel_app_kit::{Scheme, SchemeChoice};
use vessel_app_link::{Command, Link};
use vessel_app_view::connection::{ConnectionContext, ConnectionEpochs};
use vessel_app_view::devices::MachineKey;
use vessel_app_view::history::{Ask, Entry, Filter, Listed, Outcome, Reply};
use vessel_app_view::{Agent, Fleet, Machine, Reach};
use vessel_proto::{HarnessKind, HarnessRef, HarnessSessionId, NodeId, SessionId, SessionInfo};

mod geometry;

fn codex() -> HarnessRef {
    HarnessKind::Codex.into()
}

fn claude() -> HarnessRef {
    HarnessKind::ClaudeCode.into()
}

fn machine(name: &str, harnesses: &[HarnessRef]) -> Machine {
    let context = ConnectionEpochs::default()
        .admit(MachineKey::from_book(name), NodeId::random())
        .unwrap();
    Machine {
        connection_epoch: Some(context.tag().epoch()),
        node: Some(context.node()),
        name: name.to_owned(),
        reach: Reach::Online,
        sessions: Vec::new(),
        agents: harnesses
            .iter()
            .map(|harness| Agent {
                harness: harness.clone(),
                name: vessel_app_view::agent_name(harness),
                unavailable: None,
                not_installed: false,
            })
            .collect(),
        protocol: Some(vessel_proto::PROTOCOL_VERSION),
        version: None,
    }
}

fn entry(id: &str, minute: i64) -> Entry {
    Entry {
        id: HarnessSessionId::new(id),
        cwd: "/work/project".into(),
        title: Some(id.parse().unwrap()),
        updated_at: chrono::DateTime::from_timestamp(minute.saturating_mul(60), 0),
        in_vessel: None,
    }
}

/// A shell in a 1280 window with `machines` online, its sheet not open yet.
fn shell(
    cx: &mut TestAppContext,
    machines: Vec<Machine>,
) -> (
    gpui::Entity<Shell>,
    &mut VisualTestContext,
    Receiver<Command>,
) {
    cx.update(|cx| {
        vessel_app_kit::init(cx, SchemeChoice::Fixed(Scheme::Dark)).unwrap();
    });
    let (link, commands) = Link::detached(32).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| {
        let mut shell = Shell::with_test_link(window, cx, link);
        shell.fleet = Fleet {
            read_at: None,
            machines,
        };
        shell
    });
    cx.simulate_resize(size(px(1280.0), px(900.0)));
    cx.run_until_parked();
    (shell, cx, commands)
}

/// Every request the shell sent since the last call.
fn sent(commands: &mut Receiver<Command>) -> Vec<Ask> {
    let mut asks = Vec::new();
    while let Ok(command) = commands.try_recv() {
        if let Command::History(ask) = command {
            asks.push(ask);
        }
    }
    asks
}

/// Answer each listing with `rows(harness)`, as the node would.
fn answer(
    shell: &gpui::Entity<Shell>,
    cx: &mut VisualTestContext,
    asks: Vec<Ask>,
    rows: impl Fn(&HarnessRef) -> Vec<Entry>,
) {
    shell.update(cx, |shell, cx| {
        for ask in asks {
            let Ask::List { scope, ticket, .. } = ask else {
                panic!("a listing");
            };
            let node = scope.context.node();
            let entries = rows(&scope.harness);
            shell.history_apply_reply(
                Reply::Listed {
                    scope,
                    ticket,
                    result: Ok(Listed {
                        entries,
                        next: None,
                        node,
                    }),
                },
                cx,
            );
        }
    });
    cx.run_until_parked();
}

fn open(shell: &gpui::Entity<Shell>, cx: &mut VisualTestContext) {
    shell.update_in(cx, |shell, window, cx| shell.open_import(window, cx));
    cx.run_until_parked();
}

fn click(cx: &mut VisualTestContext, selector: &'static str) {
    let at = cx
        .debug_bounds(selector)
        .unwrap_or_else(|| panic!("{selector} is drawn"))
        .center();
    cx.simulate_click(at, Modifiers::default());
    cx.run_until_parked();
}

fn context_of(shell: &gpui::Entity<Shell>, cx: &mut VisualTestContext) -> ConnectionContext {
    shell.read_with(cx, |shell, _| {
        shell.areas.import.model.opening().unwrap().context.clone()
    })
}

#[gpui::test]
fn one_connected_host_skips_the_chooser_and_reads_every_agent_in_parallel(cx: &mut TestAppContext) {
    let (shell, cx, mut commands) = shell(cx, vec![machine("laptop", &[codex(), claude()])]);
    open(&shell, cx);
    let asks = sent(&mut commands);
    let harnesses: Vec<&HarnessRef> = asks.iter().map(|ask| &ask.scope().harness).collect();
    assert_eq!(harnesses, [&codex(), &claude()]);
    assert!(asks.iter().all(|ask| ask.scope().cwd.is_none()));
    shell.read_with(cx, |shell, _| {
        assert_eq!(shell.areas.import.page, Page::Sessions);
        assert!(!shell.areas.import.model.scoped());
    });
    assert!(cx.debug_bounds("history-loading").is_some());
    assert!(cx.debug_bounds("history-scope").is_some());
    answer(&shell, cx, asks, |harness| {
        if harness == &codex() {
            vec![entry("codex-a", 10)]
        } else {
            vec![entry("claude-a", 20)]
        }
    });
    assert!(cx.debug_bounds("history-loading").is_none());
    assert!(cx.debug_bounds("history-row-codex-a").is_some());
    assert!(cx.debug_bounds("history-row-claude-a").is_some());
    // Two providers: Paseo's filter pill.
    assert!(cx.debug_bounds("history-filter").is_some());
}

#[gpui::test]
fn several_hosts_without_one_in_context_open_import_from_host_first(cx: &mut TestAppContext) {
    let (shell, cx, mut commands) = shell(
        cx,
        vec![machine("alpha", &[codex()]), machine("beta", &[claude()])],
    );
    open(&shell, cx);
    assert!(
        sent(&mut commands).is_empty(),
        "nothing is read before a host"
    );
    shell.read_with(cx, |shell, _| {
        assert_eq!(shell.areas.import.page, Page::Hosts);
    });
    assert!(cx.debug_bounds("history-host-alpha").is_some());
    click(cx, "history-host-beta");
    let asks = sent(&mut commands);
    assert_eq!(asks.len(), 1);
    assert!(
        asks.iter()
            .all(|ask| ask.scope().machine.as_str() == "beta" && ask.scope().harness == claude())
    );
    shell.read_with(cx, |shell, _| {
        assert_eq!(shell.areas.import.page, Page::Sessions);
    });
    // One provider: no pill.
    answer(&shell, cx, asks, |_| vec![entry("claude-a", 1)]);
    assert!(cx.debug_bounds("history-filter").is_none());
}

#[gpui::test]
fn a_row_press_imports_that_session_then_closes_the_sheet_and_opens_it(cx: &mut TestAppContext) {
    let (shell, cx, mut commands) = shell(cx, vec![machine("laptop", &[codex()])]);
    open(&shell, cx);
    let asks = sent(&mut commands);
    answer(&shell, cx, asks, |_| vec![entry("native", 1)]);
    click(cx, "history-row-native");
    let Ok([Ask::Import(batch)]) = <[Ask; 1]>::try_from(sent(&mut commands)) else {
        panic!("one import");
    };
    assert_eq!(batch.entries.len(), 1);
    shell.read_with(cx, |shell, _| {
        assert_eq!(
            shell
                .areas
                .import
                .model
                .importing()
                .map(HarnessSessionId::as_str),
            Some("native")
        );
    });
    // A second press while it runs sends nothing.
    click(cx, "history-row-native");
    assert!(sent(&mut commands).is_empty());
    let node = context_of(&shell, cx).node();
    let created = SessionId::random();
    let entry = batch.entries.first().unwrap().clone();
    shell.update(cx, |shell, cx| {
        shell.history_apply_reply(
            Reply::Imported {
                outcomes: vec![Outcome {
                    id: entry.id.clone(),
                    result: Ok(Box::new(SessionInfo::new(
                        created,
                        entry.spec(&batch.scope),
                        chrono::DateTime::UNIX_EPOCH,
                    ))),
                }],
                scope: batch.scope.clone(),
                ticket: batch.ticket,
            },
            cx,
        );
    });
    cx.run_until_parked();
    shell.read_with(cx, |shell, _| {
        assert!(!shell.areas.import.open, "the sheet closed");
        let open = shell
            .areas
            .session
            .open
            .as_ref()
            .expect("the session opened");
        assert_eq!(open.session, created);
        assert_eq!(
            open.binding.as_ref().map(|binding| binding.address.node),
            Some(node),
            "the node the import went to"
        );
    });
}

#[gpui::test]
fn a_receipt_after_the_sheet_closed_opens_nothing(cx: &mut TestAppContext) {
    let (shell, cx, mut commands) = shell(cx, vec![machine("laptop", &[codex()])]);
    open(&shell, cx);
    let asks = sent(&mut commands);
    answer(&shell, cx, asks, |_| vec![entry("native", 1)]);
    click(cx, "history-row-native");
    let Ok([Ask::Import(batch)]) = <[Ask; 1]>::try_from(sent(&mut commands)) else {
        panic!("one import");
    };
    click(cx, "dialog-close");
    let entry = batch.entries.first().unwrap().clone();
    shell.update(cx, |shell, cx| {
        shell.history_apply_reply(
            Reply::Imported {
                outcomes: vec![Outcome {
                    id: entry.id.clone(),
                    result: Ok(Box::new(SessionInfo::new(
                        SessionId::random(),
                        entry.spec(&batch.scope),
                        chrono::DateTime::UNIX_EPOCH,
                    ))),
                }],
                scope: batch.scope.clone(),
                ticket: batch.ticket,
            },
            cx,
        );
    });
    cx.run_until_parked();
    shell.read_with(cx, |shell, _| {
        assert!(!shell.areas.import.open);
        assert!(shell.areas.import.opened.is_none());
        assert!(shell.areas.session.open.is_none());
    });
}

#[gpui::test]
fn the_provider_filter_narrows_the_merged_rows_on_the_client(cx: &mut TestAppContext) {
    let (shell, cx, mut commands) = shell(cx, vec![machine("laptop", &[codex(), claude()])]);
    open(&shell, cx);
    let asks = sent(&mut commands);
    answer(&shell, cx, asks, |harness| {
        if harness == &codex() {
            vec![entry("codex-a", 10)]
        } else {
            vec![entry("claude-a", 20)]
        }
    });
    click(cx, "history-filter");
    shell.read_with(cx, |shell, _| assert!(shell.areas.import.filter.is_open()));
    shell.update_in(cx, |shell, window, cx| {
        listener(shell, Action::Filter(Filter::Provider(claude())))(shell, window, cx);
    });
    cx.run_until_parked();
    assert!(sent(&mut commands).is_empty(), "the filter reads nothing");
    assert!(cx.debug_bounds("history-row-claude-a").is_some());
    assert!(cx.debug_bounds("history-row-codex-a").is_none());
    shell.read_with(cx, |shell, _| assert!(!shell.areas.import.filter.is_open()));
}

#[gpui::test]
fn a_failed_provider_keeps_the_others_and_its_retry_reads_it_alone(cx: &mut TestAppContext) {
    let (shell, cx, mut commands) = shell(cx, vec![machine("laptop", &[codex(), claude()])]);
    open(&shell, cx);
    let Ok([first, second]) = <[Ask; 2]>::try_from(sent(&mut commands)) else {
        panic!("two reads");
    };
    shell.update(cx, |shell, cx| {
        shell.history_apply_reply(first.refused(Problem::Timeout), cx);
    });
    answer(&shell, cx, vec![second], |_| vec![entry("claude-a", 1)]);
    assert!(cx.debug_bounds("history-provider-errors").is_some());
    assert!(cx.debug_bounds("history-row-claude-a").is_some());
    click(cx, "history-retry-codex");
    let Ok([retried]) = <[Ask; 1]>::try_from(sent(&mut commands)) else {
        panic!("one read");
    };
    assert_eq!(retried.scope().harness, codex());
}

#[gpui::test]
fn every_listed_session_already_imported_says_so(cx: &mut TestAppContext) {
    let (shell, cx, mut commands) = shell(cx, vec![machine("laptop", &[codex()])]);
    open(&shell, cx);
    let asks = sent(&mut commands);
    answer(&shell, cx, asks, |_| {
        let mut imported = entry("done", 1);
        imported.in_vessel = Some(SessionId::random());
        vec![imported]
    });
    assert!(cx.debug_bounds("history-empty").is_some());
    assert!(cx.debug_bounds("history-row-done").is_none());
    shell.read_with(cx, |shell, _| {
        let area = &shell.areas.import;
        assert_eq!(
            area.model.empty_state(&area.query),
            Some(vessel_app_view::history::Empty::AlreadyImported)
        );
    });
}

#[gpui::test]
fn controls_drawn_for_an_earlier_opening_do_nothing(cx: &mut TestAppContext) {
    let (shell, cx, mut commands) = shell(cx, vec![machine("laptop", &[codex()])]);
    open(&shell, cx);
    let asks = sent(&mut commands);
    answer(&shell, cx, asks, |_| vec![entry("native", 1)]);
    let stale = shell.read_with(cx, |shell, _| {
        (
            listener(
                shell,
                Action::Row {
                    harness: codex(),
                    id: HarnessSessionId::new("native"),
                },
            ),
            listener(shell, Action::Refresh),
            listener(shell, Action::Close),
        )
    });
    // Close and open again: a new opening with the same host and rows.
    shell.update_in(cx, |shell, window, cx| {
        shell.close_import(window, cx);
        shell.open_import(window, cx);
    });
    let asks = sent(&mut commands);
    answer(&shell, cx, asks, |_| vec![entry("native", 1)]);
    shell.update_in(cx, |shell, window, cx| {
        (stale.0)(shell, window, cx);
        (stale.1)(shell, window, cx);
        (stale.2)(shell, window, cx);
    });
    assert!(
        sent(&mut commands).is_empty(),
        "stale intentions send nothing"
    );
    shell.read_with(cx, |shell, _| {
        assert!(
            shell.areas.import.open,
            "a stale close keeps the new opening"
        );
        assert!(shell.areas.import.model.importing().is_none());
    });
    // The current row still imports.
    click(cx, "history-row-native");
    assert!(matches!(
        <[Ask; 1]>::try_from(sent(&mut commands)),
        Ok([Ask::Import(_)])
    ));
}
