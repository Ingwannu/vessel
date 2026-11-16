//! Isolated actor inputs and the one framed test peer, shared by book and
//! device-binding regressions. No live node, global HOME edit or credentials.
pub(crate) mod actor;
mod deadline;
pub(crate) mod peer;
use crate::{Event, State};
pub(crate) use deadline::{phase, within};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::mpsc;
pub(crate) const DEADLINE: std::time::Duration = std::time::Duration::from_secs(5);
static NEXT: AtomicU64 = AtomicU64::new(0);
pub(crate) struct Home(pub(crate) PathBuf);
impl Home {
    pub(crate) fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "vessel-machine-actor-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    pub(crate) fn book(&self) -> PathBuf {
        self.0.join("nodes.toml")
    }
}
impl Drop for Home {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
pub(crate) fn state(home: &Home) -> (State, mpsc::Receiver<Event>) {
    let (events, receiving) = mpsc::channel(crate::CHANNEL);
    let (commands, _inbox) = mpsc::channel(crate::CHANNEL);
    (
        State {
            label: "test".to_owned(),
            commands: commands.downgrade(),
            events,
            clients: HashMap::new(),
            follow: None,
            follow_machine: None,
            book_path: crate::machines::BookPath::File(home.book()),
            workspace: crate::workspace::Hub::default(),
            agents: crate::agents::Hub::default(),
            admissions: vessel_app_view::connection::ConnectionEpochs::default(),
            device_calls: crate::devices::Hub::default(),
            history_calls: crate::history::Hub::default(),
            sync_calls: crate::sync::Hub::default(),
            session_navigation: crate::sessions::Hub::default(),
            session_mark_calls: crate::session_marks::Hub::default(),
            session_projection: crate::sessions::Projection::default(),
            offers: std::sync::Arc::default(),
        },
        receiving,
    )
}
