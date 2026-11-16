//! Fleet observations feed one registry cache; metadata never chooses an action route.
use crate::shell::Shell;
use vessel_app_link::{Command, workspace::registry_reads::Response};

impl Shell {
    pub(super) fn registry_fleet(&mut self) {
        let contexts: Vec<_> = self
            .fleet
            .machines
            .iter()
            .filter_map(vessel_app_view::Machine::connection_context)
            .collect();
        self.areas.workspace.registry.reconcile(&contexts);
        self.read_registry_queue();
    }

    pub(crate) fn registry_read_answer(&mut self, response: Response) {
        self.areas.workspace.registry.receive(response);
        self.read_registry_queue();
    }

    fn read_registry_queue(&mut self) {
        while let Some(request) = self.areas.workspace.registry.next_read() {
            self.command(Command::RegistryRead(request));
        }
    }
}
