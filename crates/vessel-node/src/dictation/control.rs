//! First selected stop and independent record retirement; no audio or I/O.

use std::sync::Arc;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

#[derive(Clone, Copy)]
pub(super) enum Retention {
    KeepTerminal,
    Forget,
}

pub(super) struct Control<E> {
    selected: watch::Sender<Option<Arc<E>>>,
    work: CancellationToken,
    retire: CancellationToken,
}
impl<E> Clone for Control<E> {
    fn clone(&self) -> Self {
        Self {
            selected: self.selected.clone(),
            work: self.work.clone(),
            retire: self.retire.clone(),
        }
    }
}
impl<E> Control<E> {
    pub(super) fn new(connection: &CancellationToken) -> Self {
        let (selected, _receiver) = watch::channel(None);
        Self {
            selected,
            work: connection.child_token(),
            retire: CancellationToken::new(),
        }
    }
    pub(super) fn stop(&self, incoming: Arc<E>, retention: Retention) -> Arc<E> {
        let mut chosen = Arc::clone(&incoming);
        let _changed = self.selected.send_if_modified(|current| {
            if let Some(first) = current {
                chosen = Arc::clone(first);
                false
            } else {
                *current = Some(incoming);
                true
            }
        });
        self.work.cancel();
        match retention {
            Retention::KeepTerminal => {}
            Retention::Forget => self.retire.cancel(),
        }
        chosen
    }
    #[cfg(test)]
    pub(super) fn subscribe(&self) -> watch::Receiver<Option<Arc<E>>> {
        self.selected.subscribe()
    }
    pub(super) fn chosen(&self) -> Option<Arc<E>> {
        self.selected.borrow().clone()
    }
    pub(super) fn work(&self) -> CancellationToken {
        self.work.clone()
    }
    pub(super) fn retiring(&self) -> CancellationToken {
        self.retire.clone()
    }
}
