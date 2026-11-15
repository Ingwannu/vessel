//! Preview covers the selected/current union without reading currently ignored files.
use super::{Archive, CheckpointFsError, IgnoreDefaults, Tree};
use crate::Git;
use tokio_util::sync::CancellationToken;
use vessel_proto::checkpoint::{
    CheckpointError, Entry, FileImage, FileMetadata, FileState, GroupId, ObservedEntry,
    ObservedPresence, ObservedSnapshot, OrdinaryMode, OwnerId, SnapshotEvidence,
};

impl Tree {
    /// Capture today's covered files and explicitly observe selected missing paths.
    /// Existing paths outside today's Git coverage refuse instead of being read
    /// or overwritten through a previously captured filename.
    /// # Errors
    /// Changed/unsafe scope, outside-coverage entries, cancellation or actual I/O.
    pub fn capture_preview(
        &self,
        runtime: &tokio::runtime::Handle,
        git: &Git,
        archive: &Archive,
        defaults: &IgnoreDefaults,
        selected: &FileImage,
        cancel: &CancellationToken,
    ) -> Result<ObservedSnapshot, CheckpointFsError> {
        let current = self.capture(runtime, git, archive, defaults, cancel)?;
        let mut entries = current.image().entries().to_vec();
        let mut observed = current.evidence().entries().to_vec();
        for entry in selected.entries() {
            if cancel.is_cancelled() {
                return Err(CheckpointError::CaptureTimedOut.into());
            }
            if current.image().get(&entry.path).is_none() {
                if self.0.file(&entry.path)?.is_some() {
                    return Err(CheckpointError::UnsupportedScope.into());
                }
                entries.push(Entry {
                    path: entry.path.clone(),
                    state: FileState::Absent,
                });
                observed.push(ObservedEntry {
                    path: entry.path.clone(),
                    presence: ObservedPresence::Absent,
                });
            }
        }
        self.0.revalidate()?;
        Ok(ObservedSnapshot::new(
            FileImage::new(entries)?,
            SnapshotEvidence::new(
                current.evidence().physical(),
                current.evidence().root(),
                current.evidence().ignore(),
                observed,
            )?,
        )?)
    }
}
/// Security metadata for recreating absent files, from the actual node account.
/// Captured ownership/access bits never become creation defaults.
/// # Errors
/// The named ordinary private access mode cannot be represented.
pub fn private_metadata() -> Result<FileMetadata, CheckpointError> {
    const PRIVATE_ACCESS: u32 = 0o600;
    Ok(FileMetadata {
        owner: OwnerId::new(rustix::process::geteuid().as_raw()),
        group: GroupId::new(rustix::process::getegid().as_raw()),
        mode: OrdinaryMode::new(PRIVATE_ACCESS)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, os::unix::fs::PermissionsExt as _};
    fn private(path: &std::path::Path) -> Archive {
        fs::create_dir(path).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
        Archive::open(path.to_path_buf()).unwrap()
    }
    #[tokio::test]
    async fn checkpoint_preview_observes_missing_selected_paths_and_refuses_currently_ignored_data()
    {
        let sandbox = crate::testing::Sandbox::new();
        let root = sandbox.repo("project");
        let git = sandbox.git().await;
        let runtime = tokio::runtime::Handle::current();
        tokio::task::spawn_blocking(move || {
            fs::write(root.join("lost"),b"captured before deletion").unwrap();
            let physical=crate::physical::Directory::open(&root).unwrap().identity();
            let defaults=IgnoreDefaults::new(sandbox.path().join("global-ignore")).unwrap();
            let selected_archive=private(&sandbox.path().join("selected"));
            let cancel=CancellationToken::new();
            let selected=Tree::open(&root,physical).unwrap().capture(&runtime,&git,&selected_archive,&defaults,&cancel).unwrap();
            fs::remove_file(root.join("lost")).unwrap();
            let archive=private(&sandbox.path().join("current"));
            let current=Tree::open(&root,physical).unwrap().capture_preview(&runtime,&git,&archive,&defaults,selected.image(),&cancel).unwrap();
            let path=serde_json::from_str("\"lost\"").unwrap();
            assert_eq!(current.image().get(&path),Some(&FileState::Absent));
            assert!(current.evidence().entries().iter().any(|entry| entry.path==path && entry.presence==ObservedPresence::Absent));
            let account=private_metadata().unwrap();
            assert_eq!(account.owner.get(),rustix::process::geteuid().as_raw());
            assert_eq!(account.group.get(),rustix::process::getegid().as_raw());
            assert_eq!(account.mode.get(),0o600);
            let desired=selected.image().effective(current.image(),account).unwrap();
            assert!(matches!(desired.image().get(&path),Some(FileState::Present {metadata,..}) if metadata.mode.get()==0o600));
            fs::write(root.join(".git/info/exclude"),b"lost\n").unwrap();
            fs::write(root.join("lost"),b"ignored data must not be archived or replaced").unwrap();
            let result=Tree::open(&root,physical).unwrap().capture_preview(&runtime,&git,&archive,&defaults,selected.image(),&cancel);
            assert!(matches!(result,Err(CheckpointFsError::Refused(CheckpointError::UnsupportedScope))));
            assert_eq!(fs::read(root.join("lost")).unwrap(),b"ignored data must not be archived or replaced");
        }).await.unwrap();
    }
}
