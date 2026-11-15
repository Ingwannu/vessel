//! Metadata wakeups for the one push worker. Receivers do not retain a terminal
//! Handle/sender: pruning closes them; output never creates notification work.
use super::Terminals;
use vessel_proto::terminals::TerminalInfo;

impl Terminals {
    pub(crate) fn attention_changes(&self) -> tokio::sync::watch::Receiver<()> {
        self.attention_changes.subscribe()
    }
    pub(super) fn watch_attention(&self, changes: tokio::sync::watch::Receiver<TerminalInfo>) {
        if vessel_proto::notifications::ATTENTION_NOTIFICATIONS_PROTOCOL.is_none() {
            return;
        }
        let wake = self.attention_changes.clone();
        let stop = self.runner.shutdown.clone();
        let _seen = wake.send(());
        drop(self.runner.tasks.spawn(watch(changes, wake, stop)));
    }
}
async fn watch(
    mut changes: tokio::sync::watch::Receiver<TerminalInfo>,
    wake: tokio::sync::watch::Sender<()>,
    stop: tokio_util::sync::CancellationToken,
) {
    loop {
        tokio::select! {
            biased;
            () = stop.cancelled() => return,
            changed = changes.changed() => {
                if changed.is_err() { return; }
                let _seen = wake.send(());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const DEADLINE: std::time::Duration = std::time::Duration::from_secs(5);
    #[tokio::test]
    async fn a_closed_metadata_sender_ends_the_watcher_without_a_retained_handle() {
        let info = TerminalInfo::new(
            vessel_proto::TerminalId::random(),
            vessel_proto::terminals::TerminalPlace::Session {
                session: vessel_proto::SessionId::random(),
            },
            std::path::PathBuf::from("/fixture"),
            vessel_proto::terminals::TerminalKind::Unknown,
            chrono::DateTime::UNIX_EPOCH,
        );
        let (info, changes) = tokio::sync::watch::channel(info);
        let (wake, mut receiver) = tokio::sync::watch::channel(());
        let stop = tokio_util::sync::CancellationToken::new();
        let worker = tokio::spawn(watch(changes, wake, stop));
        info.send_modify(|info| info.attention = None);
        tokio::time::timeout(DEADLINE, receiver.changed())
            .await
            .unwrap()
            .unwrap();
        drop(info);
        tokio::time::timeout(DEADLINE, worker)
            .await
            .unwrap()
            .unwrap();
    }
}
