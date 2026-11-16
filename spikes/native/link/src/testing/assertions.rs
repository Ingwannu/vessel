//! Unit-only assertion adapters; feature builds never compile these panics.
use super::{Home, make_state};
use crate::{Event, State};
use tokio::sync::mpsc;
#[path = "deadline.rs"]
mod deadline;
pub(crate) use deadline::{phase, within};

impl Home {
    pub(crate) fn new() -> Self {
        Self::try_new().expect("isolated actor directory")
    }
}
pub(crate) fn state(home: &Home) -> (State, mpsc::Receiver<Event>) {
    let (events, receiving) = mpsc::channel(crate::CHANNEL);
    let (commands, _inbox) = mpsc::channel(crate::CHANNEL);
    (make_state(home, commands.downgrade(), events), receiving)
}
