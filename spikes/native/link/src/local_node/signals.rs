//! Observe OS quit requests; no signal is sent and no cleanup policy is chosen here.

use tokio::sync::mpsc;

/// App-level quit intent always returns through its ordinary confirmation path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuitSignal {
    /// Ctrl-C or the platform's interrupt event.
    Interrupt,
    /// POSIX termination request; never treated as permission to interrupt sessions.
    Terminate,
}

pub(super) async fn observe(sender: mpsc::Sender<Result<QuitSignal, std::io::Error>>) {
    #[cfg(unix)]
    let mut terminate =
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(signal) => signal,
            Err(error) => {
                let _closed = sender.send(Err(error)).await;
                return;
            }
        };
    loop {
        #[cfg(unix)]
        let result = tokio::select! {
            interrupt = tokio::signal::ctrl_c() => interrupt.map(|()| QuitSignal::Interrupt),
            term = terminate.recv() => match term { Some(()) => Ok(QuitSignal::Terminate), None => return },
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
