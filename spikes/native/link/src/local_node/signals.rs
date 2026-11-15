//! Observe OS quit requests; no signal is sent and no cleanup policy is chosen here.

use tokio::sync::mpsc;

/// App-level quit intent always returns through its ordinary confirmation path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuitSignal {
    /// Ctrl-C or the platform's interrupt event.
    Interrupt,
    /// POSIX termination request; never treated as permission to interrupt sessions.
    Terminate,
    /// POSIX hangup uses the same protected quit admission as other quit intent.
    Hangup,
}

pub(super) async fn observe(sender: mpsc::Sender<Result<QuitSignal, std::io::Error>>) {
    #[cfg(unix)]
    let (mut terminate, mut hangup) = {
        use tokio::signal::unix::{SignalKind, signal};
        match (
            signal(SignalKind::terminate()),
            signal(SignalKind::hangup()),
        ) {
            (Ok(terminate), Ok(hangup)) => (terminate, hangup),
            (Err(error), _) | (_, Err(error)) => {
                let _closed = sender.send(Err(error)).await;
                return;
            }
        }
    };
    loop {
        #[cfg(unix)]
        let result = tokio::select! {
            interrupt = tokio::signal::ctrl_c() => interrupt.map(|()| QuitSignal::Interrupt),
            term = terminate.recv() => match term { Some(()) => Ok(QuitSignal::Terminate), None => return },
            hup = hangup.recv() => match hup { Some(()) => Ok(QuitSignal::Hangup), None => return },
        };
        #[cfg(not(unix))]
        let result = tokio::signal::ctrl_c()
            .await
            .map(|()| QuitSignal::Interrupt);
        let failed = result.is_err();
        if sender.send(result).await.is_err() || failed {
            break;
        }
    }
}
