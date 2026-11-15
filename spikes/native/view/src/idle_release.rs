//! Settings › Agents' "Release idle agents", as Paseo's
//! `IdleRuntimeCleanupCard` (`screens/settings/idle-runtime-cleanup-card.tsx`),
//! over the node's own setting: how long an idle session's agent runs
//! before it is parked (`node.settings`, `node.set_settings`, protocol 12;
//! ADR 0038).
//!
//! [Decision Log]
//! - Purpose: Paseo turns its daemon's idle cleanup on and off from a
//!   host's settings, applied at once; Vessel's node had the setting and
//!   only a command line changed it.
//! - Constraint: Paseo's setting is on or off with a fixed hour; Vessel's
//!   node keeps a number of minutes (1 to 10,080) or never. A switch must
//!   not lose a person's own minutes set on the command line.
//! - Choice: Paseo's switch. On is any number of minutes; off is never.
//!   Turning it on restores Paseo's hour (the node's default, 60
//!   minutes); turning it off sends never. The hint says the minutes in
//!   force, as Paseo's says its hour.
//! - Requests carry a ticket and the machine they are for, so an answer
//!   for another machine, or an older request, changes nothing (the
//!   Devices page's rule, `devices::Ticket`).
//! - Cost: minutes other than 60 are kept only until the switch is turned
//!   off and on again; the command line still sets any number.

use vessel_proto::parking::ParkAfter;
use vessel_proto::settings::NodeSettings;
use vessel_proto::{ErrorCode, ProtocolVersion};

use crate::devices::{MachineKey, Ticket};

/// The protocol that has `node.settings` (12).
pub const PROTOCOL: ProtocolVersion = ProtocolVersion::new(12);

/// Why the setting could not be read or changed. Codes only: a node's
/// own error text is not shown (the Devices page's rule).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Problem {
    /// No live connection to the machine.
    #[error("the machine is not connected")]
    Offline,
    /// The node refused the call, with this code (`forbidden`: this
    /// device may read but not administer).
    #[error("the node refused the call ({0:?})")]
    Refused(ErrorCode),
}

/// What the card shows for the machine chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Card {
    /// Being read (Paseo draws nothing until it has the config).
    Reading,
    /// The machine's node predates the setting: nothing to switch.
    TooOld,
    /// It could not be read.
    Failed(Problem),
    /// The switch.
    Switch(Switch),
}

/// The switch and what it says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Switch {
    /// Whether idle agents are released.
    pub on: bool,
    /// After how many minutes, when on (the hint's number).
    pub minutes: Option<u16>,
    /// Whether a change is being made (the switch waits, as Paseo's
    /// `disabled={mutation.isPending}`).
    pub pending: bool,
    /// The last change the node refused, said under the hint.
    pub refused: Option<Problem>,
}

/// One machine's setting as the page knows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    machine: MachineKey,
    ticket: Ticket,
    read: Option<Result<NodeSettings, Problem>>,
    pending: bool,
    refused: Option<Problem>,
}

impl Page {
    /// A read of `machine`'s setting, sent with `ticket`.
    #[must_use]
    pub const fn reading(machine: MachineKey, ticket: Ticket) -> Self {
        Self {
            machine,
            ticket,
            read: None,
            pending: false,
            refused: None,
        }
    }

    /// The machine it is about.
    #[must_use]
    pub const fn machine(&self) -> &MachineKey {
        &self.machine
    }

    /// An answer arrived: kept only when it is for this machine and its
    /// latest request. Returns whether it was kept.
    pub fn answered(
        &mut self,
        machine: &MachineKey,
        ticket: Ticket,
        result: Result<NodeSettings, Problem>,
    ) -> bool {
        if machine != &self.machine || ticket != self.ticket {
            return false;
        }
        if self.pending {
            // A change: on refusal the switch shows what is still in force.
            self.pending = false;
            match result {
                Ok(settings) => {
                    self.refused = None;
                    self.read = Some(Ok(settings));
                }
                Err(problem) => self.refused = Some(problem),
            }
        } else {
            self.read = Some(result);
        }
        true
    }

    /// The switch was pressed: the change to send with `ticket`, or
    /// `None` when there is nothing to switch (not read yet, a change
    /// already waiting, or a value this app does not know).
    pub fn toggle(&mut self, ticket: Ticket) -> Option<ParkAfter> {
        if self.pending {
            return None;
        }
        let Some(Ok(settings)) = self.read.as_ref() else {
            return None;
        };
        let next = match settings.park_after {
            ParkAfter::Idle { .. } => ParkAfter::Never,
            ParkAfter::Never => ParkAfter::DEFAULT,
            ParkAfter::Unknown => return None,
        };
        self.ticket = ticket;
        self.pending = true;
        Some(next)
    }

    /// What the card shows; `protocol` is the machine's.
    #[must_use]
    pub fn card(&self, protocol: Option<ProtocolVersion>) -> Card {
        if protocol.is_some_and(|protocol| protocol < PROTOCOL) {
            return Card::TooOld;
        }
        match self.read.as_ref() {
            None => Card::Reading,
            Some(Err(problem)) => Card::Failed(*problem),
            Some(Ok(settings)) => match settings.park_after {
                // A newer node's value: shown as unknown rather than
                // guessed (no switch to press).
                ParkAfter::Unknown => Card::Failed(Problem::Refused(ErrorCode::Unsupported)),
                ParkAfter::Never => Card::Switch(Switch {
                    on: false,
                    minutes: None,
                    pending: self.pending,
                    refused: self.refused,
                }),
                ParkAfter::Idle { minutes } => Card::Switch(Switch {
                    on: true,
                    minutes: Some(minutes.get()),
                    pending: self.pending,
                    refused: self.refused,
                }),
            },
        }
    }
}
