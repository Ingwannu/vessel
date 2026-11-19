//! An agent's Diagnostic sheet (ADR 0111, TRACKER 568), as Paseo's
//! `provider-diagnostic-sheet.tsx` `DiagnosticSubSheet`: asked when it opens
//! and again on Refresh.
//!
//! [Decision Log]
//! - Paseo's body has three cases: running while it has no text, the text,
//!   or "No diagnostic available". A refresh keeps the text it has.
//! - Paseo writes a failed fetch's message into the text. Here a failure
//!   is kept as its typed reason, and the screen words it. A node's own
//!   words never reach the screen (the Devices page's rule).
//! - An answer for an earlier ask carries another ticket and is dropped.

use super::Problem;
use crate::devices::Ticket;

/// What the sheet shows below its title.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheetBody<'a> {
    /// Asked, nothing to show yet.
    Running,
    /// The diagnostic's text.
    Text(&'a str),
    /// The call failed.
    Failed(Problem),
    /// Nothing asked or answered.
    None,
}

/// One open sheet.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sheet {
    asking: Option<Ticket>,
    shown: Option<Result<String, Problem>>,
}

impl Sheet {
    /// A sheet that has just asked, with `ticket`.
    #[must_use]
    pub fn asking(ticket: Ticket) -> Self {
        Self {
            asking: Some(ticket),
            shown: None,
        }
    }

    /// Ask again (Refresh), keeping what it shows until the answer.
    pub fn refresh(&mut self, ticket: Ticket) {
        self.asking = Some(ticket);
    }

    /// Take an answer. False when it is not the one asked for.
    pub fn answered(&mut self, ticket: Ticket, result: Result<String, Problem>) -> bool {
        if self.asking != Some(ticket) {
            return false;
        }
        self.asking = None;
        self.shown = Some(result);
        true
    }

    /// Whether it is asking.
    #[must_use]
    pub const fn loading(&self) -> bool {
        self.asking.is_some()
    }

    /// The text to copy, when there is one.
    #[must_use]
    pub fn text(&self) -> Option<&str> {
        match &self.shown {
            Some(Ok(text)) => Some(text),
            Some(Err(_)) | None => None,
        }
    }

    /// What the body shows.
    #[must_use]
    pub fn body(&self) -> SheetBody<'_> {
        match &self.shown {
            Some(Ok(text)) => SheetBody::Text(text),
            Some(Err(problem)) => SheetBody::Failed(*problem),
            None if self.loading() => SheetBody::Running,
            None => SheetBody::None,
        }
    }
}
