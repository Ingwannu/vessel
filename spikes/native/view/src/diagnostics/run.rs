//! The app diagnostic's run (ADR 0111, TRACKER 567), as Paseo's
//! `app-diagnostic-sheet.tsx`: the app's own facts, then each machine in the
//! book asked for its report, a progress row per step while any machine is
//! asked, then the whole text.
//!
//! [Decision Log]
//! - Purpose: Paseo's app diagnostic is one block holding what the client
//!   knows and what each host says of itself. The app's run said only what
//!   it held, so a machine's own state (its process, data directory, agents'
//!   activity, tools) never reached the report.
//! - Before: Run collected instantly and asked no machine, so the dialog had
//!   no progress to show (a stated difference from Paseo's sheet).
//! - Choice:
//!   - Each connected machine that speaks `node.diagnostics` (protocol 24) is
//!     asked once per run.
//!   - A machine that is not connected, or older, is said so in the text
//!     without a call.
//!   - The text exists only once no machine is still being asked, as Paseo
//!     collects every host before it shows its text.
//!   - An answer to an earlier run carries another ticket and is dropped.
//! - Steps: the app first (Paseo's "Client", the coordinator's "App"), then
//!   each machine by its book name. Their words are the screen's; this
//!   module names only the step and its state.

use vessel_proto::ErrorCode;
use vessel_proto::diagnostics::{DIAGNOSTICS_PROTOCOL, DiagnosticReport};

use super::{AppFacts, Problem};
use crate::board::{Machine, Reach};
use crate::devices::{MachineKey, Ticket, Tickets};

/// What a run knows of one machine's own report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodePart {
    /// Not connected: nothing to ask.
    NotConnected,
    /// Older than `node.diagnostics`: not asked.
    Unsupported,
    /// Asked, with this ticket.
    Asking(Ticket),
    /// Its report.
    Answered(Box<DiagnosticReport>),
    /// Why it has none.
    Failed(Problem),
}

/// Which step a progress row is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepOf {
    /// The app's own facts.
    App,
    /// A machine, by its book name.
    Machine(String),
}

/// A progress row's state (Paseo's `formatProgressStatus`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepState {
    /// Still asked.
    Running,
    /// Answered, or nothing to ask.
    Done,
    /// The call failed.
    Failed,
}

/// One progress row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    /// Which.
    pub of: StepOf,
    /// Its state.
    pub state: StepState,
}

/// One run of the app diagnostic.
#[derive(Debug, Clone)]
pub struct Run {
    app: AppFacts,
    machines: Vec<Machine>,
    parts: Vec<NodePart>,
}

impl Run {
    /// A new run over `machines` as they are now, and the calls to make:
    /// each asked machine by its book name, with the run's ticket for it.
    pub fn start(
        app: AppFacts,
        machines: &[Machine],
        tickets: &mut Tickets,
    ) -> (Self, Vec<(MachineKey, Ticket)>) {
        let mut asks = Vec::new();
        let parts = machines
            .iter()
            .map(|machine| {
                if !matches!(machine.reach, Reach::Online) {
                    return NodePart::NotConnected;
                }
                let speaks = machine
                    .protocol
                    .zip(DIAGNOSTICS_PROTOCOL)
                    .is_some_and(|(spoken, since)| spoken >= since);
                if !speaks {
                    return NodePart::Unsupported;
                }
                tickets.next_ticket().map_or(
                    NodePart::Failed(Problem::Refused(ErrorCode::Unavailable)),
                    |ticket| {
                        asks.push((MachineKey::from_book(&machine.name), ticket));
                        NodePart::Asking(ticket)
                    },
                )
            })
            .collect();
        let run = Self {
            app,
            machines: machines.to_vec(),
            parts,
        };
        (run, asks)
    }

    /// Take a machine's answer. False when it is not this run's (an earlier
    /// run's ticket, or a machine this run did not ask).
    pub fn answered(
        &mut self,
        machine: &MachineKey,
        ticket: Ticket,
        result: Result<DiagnosticReport, Problem>,
    ) -> bool {
        let asked = self
            .machines
            .iter()
            .zip(&mut self.parts)
            .find(|(known, part)| {
                known.name == machine.as_str() && **part == NodePart::Asking(ticket)
            });
        let Some((_machine, part)) = asked else {
            return false;
        };
        *part = match result {
            Ok(report) => NodePart::Answered(Box::new(report)),
            Err(problem) => NodePart::Failed(problem),
        };
        true
    }

    /// Whether a machine is still being asked.
    #[must_use]
    pub fn running(&self) -> bool {
        self.parts
            .iter()
            .any(|part| matches!(part, NodePart::Asking(_)))
    }

    /// The progress rows: the app, then each machine.
    #[must_use]
    pub fn steps(&self) -> Vec<Step> {
        let mut steps = vec![Step {
            of: StepOf::App,
            state: StepState::Done,
        }];
        steps.extend(
            self.machines
                .iter()
                .zip(&self.parts)
                .map(|(machine, part)| {
                    let state = match part {
                        NodePart::Asking(_) => StepState::Running,
                        NodePart::Failed(_) => StepState::Failed,
                        NodePart::NotConnected | NodePart::Unsupported | NodePart::Answered(_) => {
                            StepState::Done
                        }
                    };
                    Step {
                        of: StepOf::Machine(machine.name.clone()),
                        state,
                    }
                }),
        );
        steps
    }

    /// The whole text, once no machine is still being asked.
    #[must_use]
    pub fn text(&self) -> Option<String> {
        if self.running() {
            return None;
        }
        let nodes: Vec<Option<String>> = self
            .machines
            .iter()
            .zip(&self.parts)
            .map(|(machine, part)| node_section(&machine.name, part))
            .collect();
        Some(super::compose(&self.app, &self.machines, &nodes))
    }
}

/// What a machine said of itself, below its own section (the page's and
/// Paseo's words for a host that cannot be asked).
fn node_section(name: &str, part: &NodePart) -> Option<String> {
    match part {
        // The machine's section already says it is unreachable, and why.
        NodePart::NotConnected | NodePart::Asking(_) => None,
        NodePart::Unsupported => Some(format!(
            "Node diagnostics: {name}\n  Status: unsupported by this node"
        )),
        NodePart::Failed(problem) => Some(format!(
            "Machine diagnostics: {name}\n  Error: {}",
            problem.report_text()
        )),
        NodePart::Answered(report) => Some(
            vessel_proto::diagnostics::text::report(report)
                .trim_end()
                .to_owned(),
        ),
    }
}

#[cfg(test)]
mod tests;
