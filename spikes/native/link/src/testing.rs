//! Explicit, nondefault regression scenarios over the one real framed actor.
//! No facade exposes a client, State, epoch source, sender or guard constructor.
//!
//! [Decision Log]
//! - Purpose: App tests must consume real guarded replies across retirement.
//! - Constraint: dependency feature builds have no cfg(test) or dev dependencies.
//! - Alternatives: fabricated guards or another codec would bypass the boundary.
//! - Choice: one fallible framed owner and an opaque bounded scenario, with
//!   legacy assertions confined to unit-only modules and the existing feature.
//! - Why: the same State admission/dispatch and original lease remain authority.
//! - Cost: optional direct RPC framing is explicitly testing-only (package plan).
#[cfg(test)]
pub(crate) mod actor;
#[cfg(test)]
mod assertions;
mod failure;
mod home;
pub(crate) mod peer;
mod scenario;
#[cfg(test)]
pub(crate) use assertions::{phase, state, within};
pub use failure::{FixtureError, Phase};
pub(crate) use home::Home;
pub use scenario::{Reconnect, Scenario};

use crate::{Event, State};
use std::collections::HashMap;
use tokio::sync::mpsc;
pub(crate) const DEADLINE: std::time::Duration = std::time::Duration::from_secs(5);

fn make_state(
    home: &Home,
    commands: mpsc::WeakSender<crate::Command>,
    events: mpsc::Sender<Event>,
) -> State {
    State {
        usage_calls: crate::usage::Hub::default(),
        label: "test".to_owned(),
        commands,
        events,
        clients: HashMap::new(),
        follow: None,
        follow_machine: None,
        book_path: crate::machines::BookPath::File(home.book()),
        workspace: crate::workspace::Hub::default(),
        agents: crate::agents::Hub::default(),
        mentions: crate::mentions::Hub::default(),
        admissions: vessel_app_view::connection::ConnectionEpochs::default(),
        device_calls: crate::devices::Hub::default(),
        history_calls: crate::history::Hub::default(),
        sync_calls: crate::sync::Hub::default(),
        session_navigation: crate::sessions::Hub::default(),
        offers: std::sync::Arc::default(),
    }
}

#[cfg(test)]
mod actor_cases;
