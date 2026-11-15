//! Windows uses a local named pipe with the account's default DACL, never TCP.

use super::{DesktopError, Launch, bounded, receive, send};
use std::hash::{Hash as _, Hasher as _};
use std::path::Path;
use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeServer, ServerOptions};
use tokio::sync::{mpsc, oneshot};

fn name(directory: &Path) -> String {
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    directory.hash(&mut hash);
    format!(r"\\.\pipe\vessel-desktop-{:016x}", hash.finish())
}

pub(super) fn bind(
    directory: &Path,
    runtime: &tokio::runtime::Runtime,
) -> Result<(String, NamedPipeServer), DesktopError> {
    let _entered = runtime.enter();
    let name = name(directory);
    let server = ServerOptions::new()
        .first_pipe_instance(true)
        .reject_remote_clients(true)
        .create(&name)?;
    Ok((name, server))
}

pub(super) async fn serve(
    (name, mut server): (String, NamedPipeServer),
    sender: mpsc::Sender<Launch>,
    mut stop: oneshot::Receiver<()>,
) {
    loop {
        tokio::select! {
            _ = &mut stop=>break,
            connected=server.connect()=> {
                if let Err(error)=connected { tracing::warn!(%error,"desktop pipe stopped");break; }
                if let Err(error)=bounded(receive(&mut server,&sender)).await {
                    tracing::warn!(%error,"desktop launch request was refused");
                }
                match ServerOptions::new().reject_remote_clients(true).create(&name) {
                    Ok(next)=>server=next,
                    Err(error)=>{tracing::warn!(%error,"desktop pipe could not reopen");break;}
                }
            }
        }
    }
}

pub(super) async fn forward(directory: &Path, launch: &Launch) -> Result<(), DesktopError> {
    bounded(async {
        let mut stream = ClientOptions::new().open(name(directory))?;
        send(&mut stream, launch).await
    })
    .await
}
