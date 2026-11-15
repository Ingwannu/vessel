//! Workspace registration stays small in the shared link actor. Script reads
//! additionally resolve safe direct-host metadata on the blocking pool.
use super::{Answer, Ask, Hub, Reply};
use crate::State;
use vessel_app_view::workspace::problem::Problem;
impl State {
    pub(crate) fn workspace(&self, machine: String, ask: Ask) {
        let events = self.events.clone();
        let client = match self.client(&machine) {
            Ok(client) => client.clone(),
            Err(error) => {
                Hub::refuse(machine, ask, Problem::Gone(error.to_string()), events);
                return;
            }
        };
        if let Ask::Scripts(ask) = ask {
            let path = self.book_path.path().ok().map(std::path::Path::to_owned);
            drop(tokio::spawn(async move {
                let (mut answer, host) = tokio::join!(
                    super::scripts::run(&client, ask),
                    super::script_host::read(path, machine.clone())
                );
                if let super::ScriptAnswer::Read {
                    result: Ok(snapshot),
                    ..
                } = &mut answer
                {
                    snapshot.host = host;
                }
                Reply { machine, events }
                    .send(Answer::Scripts(answer))
                    .await;
            }));
        } else {
            self.workspace.ask(client, machine, ask, events);
        }
    }
}
