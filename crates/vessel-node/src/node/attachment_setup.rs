//! Existing attachment wiring, separated from startup assembly unchanged.

use crate::attachments::{
    Attachments, Verified, load, staging,
    uploads::{self, Uploads},
};
use crate::{config::Config, sessions::Sessions};
use std::sync::Arc;
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use vessel_store::Store;

/// How many prompts' attachment loads may wait for the node's loader; a
/// session finding no room waits (each has at most one in flight).
pub(super) const ATTACHMENT_LOADS: usize = 16;

/// The node's attachments (protocol 10): its uploads and their idle sweep,
/// and the loader that answers the sessions' `loads`.
pub(super) fn attachments(
    config: &Config,
    store: &Store,
    sessions: &Arc<Sessions>,
    loads: tokio::sync::mpsc::Receiver<vessel_session::LoadAttachments>,
    (tasks, stop): (&TaskTracker, &CancellationToken),
) -> Arc<Attachments> {
    let verified = Arc::new(Verified::default());
    load::serve(store.clone(), Arc::clone(&verified), loads, tasks, stop);
    let uploads = Arc::new(Uploads::new(staging::root(&config.data_dir), uploads::IDLE));
    uploads.sweep(tasks, stop);
    Arc::new(Attachments::new(
        store.clone(),
        Arc::clone(sessions),
        uploads,
        verified,
    ))
}
