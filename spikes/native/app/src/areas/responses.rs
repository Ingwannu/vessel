//! A card's reply retains its exact request across navigation and cancellation.
//!
//! [Decision Log]
//! - Purpose: a generic session act cannot distinguish a failed answer from
//!   the next question in the same session.
//! - Constraint: cards and embedded conversations share one window and link.
//! - Alternatives: per-card tasks lose cross-seat admission; matching only the
//!   session lets a late failure decorate a successor request.
//! - Choice: a bounded window registry owns tickets; the originating seat is
//!   retained until the exact reply settles. Check both its shown target and
//!   the still-waiting request before changing any card.
//! - Cost: an uncertain response needs a fresh authoritative read. No answer
//!   is queued offline or retried automatically; the person's draft survives.
use super::conversation::{Answer, Seat};
use crate::shell::Shell;
use std::collections::HashMap;
use vessel_app_link::{
    Act, Command,
    responses::{Answer as Reply, Ask, Response},
};
use vessel_app_view::{
    devices::MachineKey,
    responses::{Pending, Problem, Registry, Request, Target},
};
use vessel_proto::RequestId;

/// Shared admission, with seat routes bounded by the same registry.
#[derive(Default)]
pub(crate) struct Responses {
    registry: Registry,
    seats: HashMap<RequestId, Seat>,
}

impl Shell {
    /// Submit from the exact conversation that rendered this request.
    pub(crate) fn response_at(&mut self, seat: Seat, answer: Answer) {
        let response = match answer {
            Answer::Permit(id, option) => Response::Permission { id, option },
            Answer::Question(id, answer) => Response::Question { id, answer },
        };
        let request = response.request();
        let Some(area) = (seat.get)(self) else {
            return;
        };
        let Some(shown) = &area.shown else {
            return;
        };
        if !area.response_waiting(request) {
            return;
        }
        let target = Target {
            machine: MachineKey::from_book(&shown.machine),
            session: shown.session,
        };
        let pending = match self.areas.responses.registry.begin(target, request) {
            Ok(pending) => pending,
            Err(problem) => {
                self.response_refused(seat, request, problem);
                return;
            }
        };
        match Ask::new(pending.clone(), response) {
            Ok(ask) => {
                let _previous = self.areas.responses.seats.insert(pending.ticket, seat);
                self.command(Command::Respond(ask));
            }
            Err(problem) => {
                let _settled = self.areas.responses.registry.settle(&pending);
                self.response_refused(seat, request, problem);
            }
        }
    }

    /// Ignore duplicate, wrong-target and successor-request replies.
    pub(crate) fn response_answered(&mut self, answer: Reply) {
        if !self.areas.responses.registry.settle(&answer.pending) {
            return;
        }
        let Some(seat) = self.areas.responses.seats.remove(&answer.pending.ticket) else {
            return;
        };
        if !self.response_current(seat, &answer.pending) {
            return;
        }
        match answer.result {
            Ok(()) => {
                if let Some(area) = (seat.get_mut)(self) {
                    area.refused = None;
                }
            }
            Err(problem) => self.response_refused(seat, answer.pending.request, problem),
        }
        // Read after every outcome: a timed-out call may have succeeded, or a
        // second client may have answered. An acknowledgement is not a log.
        self.act_at(seat, Act::Conversation);
    }

    fn response_current(&self, seat: Seat, pending: &Pending) -> bool {
        (seat.get)(self).is_some_and(|area| {
            area.shown.as_ref().is_some_and(|shown| {
                shown.machine == pending.target.machine.as_str()
                    && shown.session == pending.target.session
            }) && area.response_waiting(pending.request)
                && match pending.request {
                    Request::Permission(id) => area
                        .permit_sending
                        .as_ref()
                        .is_some_and(|(asked, _)| *asked == id),
                    Request::Question(id) => area
                        .answering
                        .as_ref()
                        .is_some_and(|form| form.is_sending(id)),
                }
        })
    }

    fn response_refused(&mut self, seat: Seat, request: Request, problem: Problem) {
        let words = self.words.responses();
        let message = match problem {
            Problem::Offline => words.offline,
            Problem::Unconfirmed => words.unconfirmed,
            Problem::Busy => words.busy,
            Problem::Exhausted => words.exhausted,
            Problem::Unsupported => words.unsupported,
            Problem::Invalid => words.invalid,
            Problem::Unsent => words.unsent,
            Problem::Refused(_) => words.refused,
        };
        if let Some(area) = (seat.get_mut)(self)
            && area.response_waiting(request)
        {
            area.refused = Some((
                match request {
                    Request::Permission(_) => vessel_app_link::ActKind::Permit,
                    Request::Question(_) => vessel_app_link::ActKind::Answer,
                },
                message.to_owned(),
            ));
        }
    }
}

#[cfg(test)]
mod tests;
