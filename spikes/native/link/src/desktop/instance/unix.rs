//! Filesystem socket access is restricted by an owner-private directory.

use super::{DesktopError, Launch, bounded, receive, send};
use std::path::Path;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{mpsc, oneshot};

pub(super) fn check_directory(directory: &Path) -> Result<(), DesktopError> {
    use std::os::unix::fs::PermissionsExt as _;
    let metadata = std::fs::symlink_metadata(directory)?;
    if !metadata.is_dir() || metadata.permissions().mode() & 0o077 != 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "desktop IPC requires a private directory",
        )
        .into());
    }
    Ok(())
}

pub(super) fn bind(
    directory: &Path,
    runtime: &tokio::runtime::Runtime,
) -> Result<UnixListener, DesktopError> {
    use std::os::unix::fs::FileTypeExt as _;
    let path = directory.join("launch.sock");
    match std::fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_socket() => std::fs::remove_file(&path)?,
        Ok(_) => return Err(DesktopError::InvalidLaunch),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let _entered = runtime.enter();
    Ok(UnixListener::bind(path)?)
}

pub(super) async fn serve(
    listener: UnixListener,
    sender: mpsc::Sender<Launch>,
    mut stop: oneshot::Receiver<()>,
) {
    loop {
        tokio::select! {
            _ = &mut stop=>break,
            accepted=listener.accept()=> match accepted {
                Ok((mut stream,_address))=> {
                    if let Err(error)=bounded(receive(&mut stream,&sender)).await {
                        tracing::warn!(%error,"desktop launch request was refused");
                    }
                }
                Err(error)=>{tracing::warn!(%error,"desktop launch listener stopped");break;}
            }
        }
    }
}

pub(super) async fn forward(directory: &Path, launch: &Launch) -> Result<(), DesktopError> {
    bounded(async {
        let mut stream = UnixStream::connect(directory.join("launch.sock")).await?;
        send(&mut stream, launch).await
    })
    .await
}
