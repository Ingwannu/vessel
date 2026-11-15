//! The successful book edit retires only affected admission, preserving the
//! existing physical client, agents and follow on rename. Failed edits never
//! enter this hook. The book writer remains the one owner in machines.
use crate::{Event, State};
use vessel_app_view::machine_book::Edit;
impl State {
    pub(crate) async fn book_connections_applied(&mut self, edit: Edit) {
        let retired = match edit {
            Edit::Rename { from, to } => {
                if self.follow_machine.as_deref() == Some(from.as_str()) {
                    self.follow_machine = Some(to.as_str().to_owned());
                }
                if let Some(connected) = self.clients.remove(from.as_str()) {
                    let tag = connected.context.tag();
                    connected.retire();
                    if let Ok(context) = self.admissions.admit(
                        vessel_app_view::connection::MachineKey::from_book(to.as_str()),
                        connected.client.node().id,
                    ) {
                        let _old = self
                            .clients
                            .insert(to.as_str().to_owned(), connected.renamed(context));
                    }
                    Some(tag)
                } else {
                    None
                }
            }
            Edit::Forget(name) => {
                let retired = self.clients.remove(name.as_str()).map(|connected| {
                    connected.retire();
                    connected.context.tag()
                });
                // A watched conversation owns another client clone. Only
                // the app's stream is cancelled; no remote mutation is sent.
                if self.follow_machine.as_deref() == Some(name.as_str()) {
                    if let Some(follow) = self.follow.take() {
                        follow.abort();
                    }
                    self.follow_machine = None;
                }
                retired
            }
            Edit::MakeDefault(_) => None,
        };
        if let Some(tag) = retired {
            self.emit(Event::DeviceRetired(tag)).await;
        }
    }
}
