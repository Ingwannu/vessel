//! The feature keeps policy in its area; only this private-shell bridge can
//! borrow the transport. No second sender or connection lookup is introduced.
use super::Shell;
use vessel_app_link::devices::Ask;
use vessel_app_view::devices::{ConnectionContext, Problem, Ticket};

impl Shell {
    pub(crate) fn devices_dispatch(
        &self,
        context: ConnectionContext,
        ticket: Ticket,
        ask: Ask,
    ) -> Result<(), Problem> {
        self.link
            .as_ref()
            .ok_or(Problem::Offline)?
            .request_devices(context, ticket, ask)
    }
}
