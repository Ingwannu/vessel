//! Workspace registration stays small in the shared link actor. Script calls
//! use the exact shared connection admission, never a later alias resolution.
use super::{Answer, Ask, Hub, Reply};
use crate::State;
use vessel_app_view::workspace::problem::Problem;
impl State {
    pub(crate) fn workspace(&self, machine: String, ask: Ask) {
        if let Ask::CommandAccess(ask) = ask {
            self.workspace_commands(machine, ask);
            return;
        }
        let events = self.events.clone();
        if let Ask::Scripts(ask) = ask {
            // Actor admission precedes spawning; SelectedConnection fences the
            // original physical client again before poll and after completion.
            let selected = (ask.target().machine() == machine)
                .then(|| self.selected_connection(&ask.target().context));
            let client = match selected {
                Some(Ok(client)) => client,
                Some(Err(_)) | None => {
                    Hub::refuse(
                        machine,
                        Ask::Scripts(ask),
                        Problem::Gone(String::new()),
                        events,
                    );
                    return;
                }
            };
            drop(tokio::spawn(async move {
                let answer = super::scripts::run(&client, ask).await;
                Reply { machine, events }
                    .send(Answer::Scripts(answer))
                    .await;
            }));
        } else {
            let client = match self.client(&machine) {
                Ok(client) => client.clone(),
                Err(error) => {
                    Hub::refuse(machine, ask, Problem::Gone(error.to_string()), events);
                    return;
                }
            };
            self.workspace.ask(client, machine, ask, events);
        }
    }
}
