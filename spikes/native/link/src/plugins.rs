//! A machine's plugin calls for Settings › Plugins (ADR 0115): the list,
//! the machine-wide switch (`node.settings` / `node.set_settings`'
//! `plugins_enabled`), install, enable, disable, reload, remove, an
//! installation's logs and the change watch (`plugin.watch`).
//!
//! [Decision Log]
//! - Purpose: Paseo's host Plugins page drives its daemon's plugin
//!   manager; the app drives the node's the same way.
//! - Existing assets: `diagnostics`, which runs one task per ask and sends
//!   its answer as an event carrying the machine and the page's ticket.
//! - Choice: the same shape, one request type for every plugin call
//!   (`view::plugins::Request`), so the page's model decides what to ask
//!   and this module only asks it. The screens decide from the machine's
//!   protocol before asking: a call is never sent below its version.
//! - Unlike `diagnostics`, a refusal keeps the node's words
//!   (`Problem::Refused::message`): Paseo shows a plugin manager's error
//!   message, and the page shows it; it is about the person's own code on
//!   their own machine (ADR 0115).
//! - Bounds: a read 15 s; a change 60 s (the node waits up to 35 s for a
//!   helper to settle); the watch 40 s (the node answers within 25 s).
//! - Cost: no retry here; the page's model asks again.
//! - The app's plugin catalog watcher and plugin method calls (ADR 0115,
//!   catalog and invoke) use the same task with their own event
//!   (`Event::PluginCatalog`): the page and the watcher each count their
//!   own tickets, so their answers must not meet. A catalog read is bounded
//!   as a read; a method call as a change (the node keeps the helper's own
//!   deadline inside it).

use vessel_app_view::devices::{MachineKey, Ticket};
use vessel_app_view::plugins::{Problem, Reply, Request};

/// One plugin call to one machine.
#[derive(Debug, Clone)]
pub struct Ask {
    /// The machine, by its book name.
    pub machine: MachineKey,
    /// The page's identity for this request.
    pub ticket: Ticket,
    /// The call.
    pub request: Request,
}

/// Its answer, or why there is none.
#[derive(Debug, Clone)]
pub struct Answer {
    /// The machine asked.
    pub machine: MachineKey,
    /// The request's identity.
    pub ticket: Ticket,
    /// The reply.
    pub result: Result<Reply, Problem>,
}

impl Ask {
    /// Its answer when it was not asked at all.
    #[must_use]
    pub fn refused(self, problem: Problem) -> Answer {
        Answer {
            machine: self.machine,
            ticket: self.ticket,
            result: Err(problem),
        }
    }
}

/// How long a call may take before its answer is given up.
#[cfg(not(target_arch = "wasm32"))]
fn bound(request: &Request) -> std::time::Duration {
    use std::time::Duration;
    match request {
        Request::List | Request::ReadSwitch | Request::Logs(_) | Request::Catalog => {
            Duration::from_secs(15)
        }
        Request::Switch(_) | Request::Install(_) | Request::Act(..) | Request::Invoke(_) => {
            Duration::from_secs(60)
        }
        Request::Watch(_) => Duration::from_secs(40),
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl crate::State {
    /// Ask the machine on a task of its own; the answer comes back as the
    /// event `answered` makes.
    pub(crate) async fn plugins(&self, ask: Ask, answered: fn(Answer) -> crate::Event) {
        let Ok(client) = self.client(ask.machine.as_str()).cloned() else {
            self.emit(answered(ask.refused(Problem::Offline))).await;
            return;
        };
        let events = self.events.clone();
        drop(tokio::spawn(async move {
            let result = tokio::time::timeout(bound(&ask.request), call(&client, ask.request))
                .await
                .unwrap_or(Err(Problem::TimedOut));
            let answer = Answer {
                machine: ask.machine,
                ticket: ask.ticket,
                result,
            };
            let _closed = events.send(answered(answer)).await;
        }));
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn call(client: &vessel_client::NodeClient, request: Request) -> Result<Reply, Problem> {
    use vessel_app_view::plugins::Action;
    use vessel_proto::plugins::{
        DisablePlugin, EnablePlugin, GetPluginCatalog, GetPluginLogs, InstallPlugin, ListPlugins,
        ReloadPlugin, RemovePlugin, WatchPlugins,
    };
    use vessel_proto::settings::{GetNodeSettings, SetNodeSettings};
    let done = |_item| Reply::Done;
    match request {
        Request::List => client
            .call(ListPlugins {})
            .await
            .map(|list| Reply::List(list.plugins)),
        Request::ReadSwitch => client
            .call(GetNodeSettings::default())
            .await
            .map(Reply::Settings),
        Request::Switch(on) => client
            .call(SetNodeSettings {
                plugins_enabled: Some(on),
                ..SetNodeSettings::default()
            })
            .await
            .map(Reply::Settings),
        Request::Install(source) => client
            .call(InstallPlugin {
                source,
                id: None,
                reference: None,
            })
            .await
            .map(Reply::Item),
        Request::Act(Action::Enable, plugin) => {
            client.call(EnablePlugin { plugin }).await.map(done)
        }
        Request::Act(Action::Disable, plugin) => {
            client.call(DisablePlugin { plugin }).await.map(done)
        }
        Request::Act(Action::Reload, plugin) => {
            client.call(ReloadPlugin { plugin }).await.map(done)
        }
        Request::Act(Action::Remove, plugin) => client
            .call(RemovePlugin { plugin })
            .await
            .map(|_ack| Reply::Done),
        Request::Logs(plugin) => client.call(GetPluginLogs { plugin }).await.map(Reply::Logs),
        Request::Watch(after) => client
            .call(WatchPlugins { after })
            .await
            .map(Reply::Generation),
        Request::Catalog => client.call(GetPluginCatalog {}).await.map(Reply::Catalog),
        Request::Invoke(invoke) => client
            .call(invoke)
            .await
            .map(|answer| Reply::Output(answer.output)),
    }
    .map_err(problem)
}

/// A call's failure, with the node's words when it gave some.
#[cfg(not(target_arch = "wasm32"))]
fn problem(error: vessel_client::CallError) -> Problem {
    use vessel_proto::ErrorCode;
    match error {
        vessel_client::CallError::Disconnected => Problem::Dropped,
        vessel_client::CallError::Remote(error) => Problem::Refused {
            code: error.code,
            message: Some(error.message).filter(|message| !message.trim().is_empty()),
        },
        vessel_client::CallError::NotInProtocol { .. }
        | vessel_client::CallError::UnassignedProtocol { .. } => Problem::Unsupported,
        vessel_client::CallError::UnexpectedReply(_)
        | vessel_client::CallError::RequestIdsExhausted => Problem::Refused {
            code: ErrorCode::Internal,
            message: None,
        },
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::problem;
    use vessel_app_view::plugins::Problem;
    use vessel_proto::ErrorCode;

    #[test]
    fn a_refusal_keeps_the_nodes_words_and_a_blank_one_none() {
        let worded = vessel_client::CallError::Remote(vessel_proto::WireError::new(
            ErrorCode::InvalidRequest,
            "plugin directory has no vessel-plugin.json",
        ));
        assert_eq!(
            problem(worded),
            Problem::Refused {
                code: ErrorCode::InvalidRequest,
                message: Some("plugin directory has no vessel-plugin.json".to_owned()),
            }
        );
        let blank = vessel_client::CallError::Remote(vessel_proto::WireError::new(
            ErrorCode::Forbidden,
            " ",
        ));
        assert_eq!(
            problem(blank),
            Problem::Refused {
                code: ErrorCode::Forbidden,
                message: None,
            }
        );
        assert_eq!(
            problem(vessel_client::CallError::Disconnected),
            Problem::Dropped
        );
        assert_eq!(
            problem(vessel_client::CallError::UnassignedProtocol {
                method: vessel_proto::Method::ListPlugins,
            }),
            Problem::Unsupported
        );
    }
}
