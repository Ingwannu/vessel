//! Successful book changes retire every affected admission before publication.
//! Rename keeps its source physical client, agents and follow. A stale cached
//! destination is also retired even when an external removal was not refreshed;
//! observing the successful rename must not reuse that destination's epoch.
use crate::{Event, State};
use vessel_app_view::machine_book::Edit;

/// A rename can replace its source and one stale cached destination.
const MAX_EDIT_ADMISSIONS: usize = 2;

impl State {
    pub(crate) async fn book_connections_applied(&mut self, edit: Edit) {
        let mut retired = Vec::with_capacity(MAX_EDIT_ADMISSIONS);
        match edit {
            Edit::Rename { from, to } => {
                if from == to {
                    return;
                }
                let follows_source = self.follow_machine.as_deref() == Some(from.as_str());
                if follows_source {
                    self.follow_machine = Some(to.as_str().to_owned());
                }
                if let Some(previous) = self.clients.remove(to.as_str()) {
                    previous.retire();
                    retired.push(previous.context.tag());
                    if !follows_source {
                        self.discard_follow(to.as_str());
                    }
                }
                if let Some(connected) = self.clients.remove(from.as_str()) {
                    connected.retire();
                    retired.push(connected.context.tag());
                    if let Ok(context) = self.admissions.admit(
                        vessel_app_view::connection::MachineKey::from_book(to.as_str()),
                        connected.client.node().id,
                    ) {
                        let _old = self
                            .clients
                            .insert(to.as_str().to_owned(), connected.renamed(context));
                    }
                }
            }
            Edit::Forget(name) => {
                if let Some(connected) = self.clients.remove(name.as_str()) {
                    connected.retire();
                    retired.push(connected.context.tag());
                }
                self.discard_follow(name.as_str());
            }
            Edit::MakeDefault(_) => {}
        }
        // All validity watches above change before backpressure can suspend an
        // event send. A UI waiting on either old alias cannot admit new work.
        for tag in retired {
            self.emit(Event::DeviceRetired(tag)).await;
        }
    }

    fn discard_follow(&mut self, machine: &str) {
        if self.follow_machine.as_deref() == Some(machine) {
            if let Some(follow) = self.follow.take() {
                follow.abort();
            }
            self.follow_machine = None;
        }
    }
}
