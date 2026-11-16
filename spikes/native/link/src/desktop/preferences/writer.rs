//! One serial file owner publishes effects atomically with canonical snapshots.

use super::{
    Acknowledged, Command, CommittedScrollback, DesktopError, Outcome, Revision, Snapshot, merge,
    scrollback,
};
use std::sync::{Arc, mpsc};
use tokio::sync::watch;

pub(super) fn run(
    mut snapshot: Snapshot,
    path: Option<&std::path::Path>,
    published: &watch::Sender<Snapshot>,
    receiver: &mpsc::Receiver<Command>,
    delivery: &scrollback::Hub,
) {
    while let Ok(command) = receiver.recv() {
        if !delivery.healthy() {
            break;
        }
        match command {
            Command::Flush(done) => drop(done.send(())),
            Command::Change {
                before,
                after,
                sequence,
                acknowledged,
                receipt,
            } => {
                let Some(revision) = snapshot.revision.0.checked_add(1) else {
                    snapshot.outcome = Outcome::Failed(Arc::new(DesktopError::Closed));
                    let _previous = acknowledged.send_replace(Acknowledged {
                        sequence,
                        revision: snapshot.revision,
                    });
                    if let Err(error) = delivery.publish(snapshot.clone(), published) {
                        tracing::warn!(%error, "canonical preference publication stopped");
                        break;
                    }
                    if let Some(done) = receipt {
                        drop(done.send(snapshot.clone()));
                    }
                    break;
                };
                let next = merge::apply(&snapshot.prefs, &before, &after)
                    .map_err(DesktopError::from)
                    .and_then(|next| {
                        if let Some(path) = &path {
                            next.save_at(path).map_err(DesktopError::PreferencesWrite)?;
                        }
                        Ok(next)
                    });
                snapshot.revision = Revision(revision);
                snapshot.outcome = match next {
                    Ok(next) => {
                        let limit = next.settings.terminal_scrollback;
                        snapshot.prefs = Arc::new(next);
                        if path.is_some() {
                            if snapshot
                                .scrollback
                                .is_none_or(|saved| saved.limit() != limit)
                            {
                                snapshot.scrollback =
                                    Some(CommittedScrollback::new(limit, snapshot.revision));
                            }
                            Outcome::Saved
                        } else {
                            Outcome::LaunchOnly
                        }
                    }
                    Err(error) => Outcome::Failed(Arc::new(error)),
                };
                let _previous = acknowledged.send_replace(Acknowledged {
                    sequence,
                    revision: snapshot.revision,
                });
                if let Err(error) = delivery.publish(snapshot.clone(), published) {
                    tracing::warn!(%error, "canonical preference publication stopped");
                    break;
                }
                if let Some(done) = receipt {
                    drop(done.send(snapshot.clone()));
                }
            }
        }
    }
}
