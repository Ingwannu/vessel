//! Invoke the real dispatcher without starting IO or widening its product API.
use crate::shell::Shell;
use vessel_app_kit::ui::Context;
use vessel_app_link::Event;
use vessel_app_view::{Fleet, connection::ConnectionContext};

impl Shell {
    pub(crate) fn history_test_fleet(&mut self, fleet: Fleet, cx: &mut Context<'_, Self>) {
        self.on_event(Event::Fleet(fleet), cx);
    }
    pub(crate) fn history_test_retirement(
        &mut self,
        context: &ConnectionContext,
        cx: &mut Context<'_, Self>,
    ) {
        self.on_event(Event::DeviceRetired(context.tag()), cx);
    }
}
