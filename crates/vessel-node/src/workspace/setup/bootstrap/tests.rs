//! No PTYs, process spawns, real registry or filesystem inference in these tests.
use std::cell::{Cell, RefCell};
use std::future::pending;

use super::*;

#[derive(Debug, Clone, Copy)]
enum Mode {
    Output,
    Silent,
    SpawnFailure,
    Revoked,
    CancelReady,
    InputFailure,
    Refuse(Failure),
}

#[derive(Debug, PartialEq, Eq)]
enum Event {
    Claim(Scope),
    Open(usize, Option<String>),
    Input(usize, Vec<u8>),
}

struct Fake {
    modes: Vec<Mode>,
    next: Cell<usize>,
    events: RefCell<Vec<Event>>,
    terminals: Vec<TerminalId>,
    cancel: CancellationToken,
}

impl Fake {
    fn new(modes: Vec<Mode>) -> Self {
        Self {
            terminals: modes.iter().map(|_| TerminalId::random()).collect(),
            modes,
            next: Cell::new(0),
            events: RefCell::new(Vec::new()),
            cancel: CancellationToken::new(),
        }
    }

    fn index(&self, terminal: TerminalId) -> usize {
        self.terminals
            .iter()
            .position(|id| *id == terminal)
            .unwrap()
    }
}

impl Backend for Fake {
    type Lease = usize;

    async fn claim(&self, scope: Scope) -> Result<usize, Failure> {
        self.events.borrow_mut().push(Event::Claim(scope));
        let index = self.next.get();
        self.next.set(index.checked_add(1).unwrap());
        match self.modes[index] {
            Mode::Refuse(failure) => Err(failure),
            Mode::Output
            | Mode::Silent
            | Mode::SpawnFailure
            | Mode::Revoked
            | Mode::CancelReady
            | Mode::InputFailure => Ok(index),
        }
    }

    async fn open(
        &self,
        lease: &usize,
        name: Option<&SessionTitle>,
    ) -> Result<TerminalId, Failure> {
        self.events
            .borrow_mut()
            .push(Event::Open(*lease, name.map(ToString::to_string)));
        if matches!(self.modes[*lease], Mode::SpawnFailure) {
            Err(Failure::Spawn)
        } else {
            Ok(self.terminals[*lease])
        }
    }

    async fn first_output(&self, terminal: TerminalId) {
        match self.modes[self.index(terminal)] {
            Mode::Silent => pending().await,
            Mode::CancelReady => {
                self.cancel.cancel();
                pending::<()>().await;
            }
            Mode::Output
            | Mode::SpawnFailure
            | Mode::Revoked
            | Mode::InputFailure
            | Mode::Refuse(_) => {}
        }
    }

    async fn input(
        &self,
        lease: &usize,
        terminal: TerminalId,
        bytes: TerminalBytes,
    ) -> Result<(), Failure> {
        assert_eq!(self.terminals[*lease], terminal);
        match self.modes[*lease] {
            Mode::Revoked => Err(Failure::Unknown),
            Mode::InputFailure => Err(Failure::Input),
            Mode::Output
            | Mode::Silent
            | Mode::SpawnFailure
            | Mode::CancelReady
            | Mode::Refuse(_) => {
                self.events
                    .borrow_mut()
                    .push(Event::Input(*lease, bytes.bytes().to_vec()));
                Ok(())
            }
        }
    }
}

fn commands(approved: bool, count: usize) -> ProjectCommands {
    ProjectCommands::Listed {
        setup: Vec::new(),
        teardown: Vec::new(),
        scripts: Vec::new(),
        digest: "a".repeat(64).parse().unwrap(),
        approved,
        last_approved: None,
        service_ports: None,
        terminals: (0..count)
            .map(|index| FileTerminal {
                name: Some(format!("terminal {index}").parse().unwrap()),
                // Shell syntax must be retained as interactive shell input.
                command: format!("printf '{index}'; echo $SHELL").parse().unwrap(),
            })
            .collect(),
    }
}

#[tokio::test(start_paused = true)]
async fn silent_first_terminal_observes_the_deadline_before_second_start() {
    let fake = Fake::new(vec![Mode::Silent, Mode::Output]);
    let scope = Scope {
        workspace: WorkspaceId::random(),
        run: SetupRunId::random(),
    };
    let began = tokio::time::Instant::now();
    let results = run(
        &fake,
        scope,
        &commands(true, 2),
        SetupRunState::Succeeded,
        &fake.cancel,
    )
    .await
    .unwrap();
    assert_eq!(began.elapsed(), READY_LIMIT);
    assert_eq!(
        results,
        fake.terminals
            .iter()
            .copied()
            .map(ResultState::Sent)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        *fake.events.borrow(),
        vec![
            Event::Claim(scope),
            Event::Open(0, Some("terminal 0".to_owned())),
            Event::Input(0, b"printf '0'; echo $SHELL\r".to_vec()),
            Event::Claim(scope),
            Event::Open(1, Some("terminal 1".to_owned())),
            Event::Input(1, b"printf '1'; echo $SHELL\r".to_vec()),
        ]
    );
}

#[tokio::test]
async fn terminal_only_files_need_approval_and_successful_setup_before_any_claim() {
    let fake = Fake::new(vec![]);
    let scope = Scope {
        workspace: WorkspaceId::random(),
        run: SetupRunId::random(),
    };
    assert!(
        run(
            &fake,
            scope,
            &commands(false, 1),
            SetupRunState::Succeeded,
            &fake.cancel
        )
        .await
        .is_none()
    );
    for state in [
        SetupRunState::Running,
        SetupRunState::Failed,
        SetupRunState::Cancelled,
        SetupRunState::Unknown,
    ] {
        assert!(
            run(&fake, scope, &commands(true, 1), state, &fake.cancel)
                .await
                .is_none()
        );
    }
    assert!(fake.events.borrow().is_empty());
}

#[tokio::test]
async fn cancellation_while_readiness_waits_sends_no_input_and_starts_no_next_terminal() {
    let fake = Fake::new(vec![Mode::CancelReady, Mode::Output]);
    let result = run(
        &fake,
        Scope {
            workspace: WorkspaceId::random(),
            run: SetupRunId::random(),
        },
        &commands(true, 2),
        SetupRunState::Succeeded,
        &fake.cancel,
    )
    .await;
    assert_eq!(result, Some(vec![ResultState::Cancelled]));
    assert_eq!(fake.next.get(), 1);
    assert!(
        !fake
            .events
            .borrow()
            .iter()
            .any(|event| matches!(event, Event::Input(_, _)))
    );
}

#[tokio::test]
async fn every_start_rechecks_exact_workspace_even_after_an_earlier_terminal_succeeded() {
    let scope = Scope {
        workspace: WorkspaceId::random(),
        run: SetupRunId::random(),
    };
    let fake = Fake::new(vec![Mode::Output, Mode::Refuse(Failure::Blocked)]);
    assert_eq!(
        run(
            &fake,
            scope,
            &commands(true, 2),
            SetupRunState::Succeeded,
            &fake.cancel
        )
        .await,
        Some(vec![
            ResultState::Sent(fake.terminals[0]),
            ResultState::Failed(Failure::Blocked)
        ])
    );
    assert_eq!(
        fake.events
            .borrow()
            .iter()
            .filter(|event| **event == Event::Claim(scope))
            .count(),
        2
    );
    assert!(
        !fake
            .events
            .borrow()
            .iter()
            .any(|event| matches!(event, Event::Open(1, _)))
    );
}

#[tokio::test]
async fn revocation_after_shell_output_refuses_input_without_removing_the_worktree() {
    let fake = Fake::new(vec![Mode::Revoked]);
    assert_eq!(
        run(
            &fake,
            Scope {
                workspace: WorkspaceId::random(),
                run: SetupRunId::random()
            },
            &commands(true, 1),
            SetupRunState::Succeeded,
            &fake.cancel
        )
        .await,
        Some(vec![ResultState::Failed(Failure::Unknown)])
    );
    assert!(
        !fake
            .events
            .borrow()
            .iter()
            .any(|event| matches!(event, Event::Input(_, _)))
    );
}

#[tokio::test]
async fn independent_failures_report_each_attempt_without_stopping_later_specs() {
    let fake = Fake::new(vec![
        Mode::SpawnFailure,
        Mode::InputFailure,
        Mode::Refuse(Failure::Unavailable),
        Mode::Output,
    ]);
    assert_eq!(
        run(
            &fake,
            Scope {
                workspace: WorkspaceId::random(),
                run: SetupRunId::random()
            },
            &commands(true, 4),
            SetupRunState::Succeeded,
            &fake.cancel
        )
        .await,
        Some(vec![
            ResultState::Failed(Failure::Spawn),
            ResultState::Failed(Failure::Input),
            ResultState::Failed(Failure::Unavailable),
            ResultState::Sent(fake.terminals[3])
        ])
    );
}
