//! Preserve legacy read routing; creation alone requires a captured admission.
use super::{Ask, Hub, StartEnded, start};
use crate::State;
use vessel_app_view::{creation::Failure, workspace::problem::Problem};

impl State {
    pub(crate) fn workspace(&self, machine: String, ask: Ask) {
        let Ask::Start(starting) = ask else {
            let events = self.events.clone();
            match self.client(&machine) {
                Ok(client) => self.workspace.ask(client.clone(), machine, ask, events),
                Err(error) => Hub::refuse(machine, ask, Problem::Gone(error.to_string()), events),
            }
            return;
        };
        let selected = (starting.creation.context.machine().as_str() == machine)
            .then(|| self.selected_connection(&starting.creation.context))
            .and_then(Result::ok);
        if let Some(selected) = selected {
            start::start_on(selected, *starting, self.events.clone());
        } else {
            let reply = super::Reply {
                machine,
                events: self.events.clone(),
            };
            drop(tokio::spawn(async move {
                reply
                    .send(super::Answer::StartEnded {
                        key: starting.key,
                        ended: StartEnded::Refused(Failure::Refused(
                            vessel_proto::ErrorCode::Conflict,
                        )),
                    })
                    .await;
            }));
        }
    }
}
