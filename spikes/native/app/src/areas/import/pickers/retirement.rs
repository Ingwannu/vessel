//! Retire a UI opening synchronously when its drawn host metadata changes.
use crate::shell::Shell;
use vessel_app_kit::ui::Context;
use vessel_app_view::{Fleet, connection::ConnectionContext};

impl Shell {
    /// Registration runs immediately after the sole `Fleet` installation, before
    /// another retained input can dispatch. Session activity alone is irrelevant.
    pub(crate) fn history_fleet(&mut self, earlier: &Fleet) {
        if !self.areas.import.open {
            return;
        }
        let controls = |machine: &vessel_app_view::Machine| {
            (
                machine.name.clone(),
                machine.connection_context(),
                machine.agents.clone(),
            )
        };
        if earlier
            .online()
            .map(controls)
            .ne(self.fleet.online().map(controls))
        {
            let _current = self.areas.import.advance_visit();
        }
    }

    /// An exact retired context can invalidate a still-displayed metadata row.
    /// A delayed retirement for a context absent from the current fleet/scope
    /// must not touch a successor opening. No node lookup or replacement occurs.
    pub(crate) fn history_retired(
        &mut self,
        context: &ConnectionContext,
        cx: &mut Context<'_, Self>,
    ) {
        if self.areas.import.open
            && (self
                .areas
                .import
                .model
                .scope()
                .is_some_and(|scope| &scope.context == context)
                || self
                    .fleet
                    .online()
                    .any(|machine| machine.connection_context().as_ref() == Some(context)))
        {
            let _current = self.areas.import.advance_visit();
            // DeviceRetired may be irrelevant to Devices and return before the
            // outer dispatch notify; this consumer still changed its own state.
            cx.notify();
        }
    }
}
