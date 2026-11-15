//! Synchronous admitted-worker API; the node retains root/pool/quota ownership.
//! Never call these filesystem operations directly on an async runtime worker.
use super::{Error, archive, policy, restore, rooted::Root};
use crate::Git;
use std::path::{Path, PathBuf};
use tokio_util::sync::CancellationToken;
use vessel_proto::{
    BlobHash,
    checkpoint::{
        CheckpointError, EffectiveDesiredImage, FileImage, ObservedSnapshot, PathCount,
        PhysicalRoot, StagedPaths,
    },
};

/// Opened project root checked against the actual physical admission key.
#[derive(Debug)]
pub struct Tree(Root);
/// Service-owned immutable raw blobs in an existing private capture directory.
#[derive(Debug)]
pub struct Archive(pub(super) archive::Archive);
/// Explicit global-ignore fallback resolved by the node, never ambient HOME here.
#[derive(Debug, Clone)]
pub struct IgnoreDefaults(policy::Defaults);
/// Opened stage and immutable expected objects; the node owns durable progress.
#[derive(Debug)]
pub struct Restore {
    root: Root,
    stage: restore::stage::Stage,
    prepared: restore::Prepared,
}

impl Tree {
    /// Open a managed root without accepting a logical id as physical authority.
    /// # Errors
    /// Unsafe/foreign metadata, replaced root, unsupported mount or filesystem I/O.
    pub fn open(path: &Path, expected: PhysicalRoot) -> Result<Self, Error> {
        let root = Root::open(path)?;
        if root.physical() != expected {
            return Err(CheckpointError::StalePreview.into());
        }
        Ok(Self(root))
    }
    /// Capture and recheck the entire tracked/nonignored scope on this worker.
    /// # Errors
    /// Unsafe/unstable scope, ignore-policy changes, cancellation or I/O failure.
    pub fn capture(
        &self,
        runtime: &tokio::runtime::Handle,
        git: &Git,
        archive: &Archive,
        defaults: &IgnoreDefaults,
        cancel: &CancellationToken,
    ) -> Result<ObservedSnapshot, Error> {
        super::scope::capture_opened(
            runtime,
            git,
            self.0.path(),
            &self.0,
            &archive.0,
            &defaults.0,
            cancel,
        )
    }
    /// Stage every effective target before any project mutation. The node must
    /// commit evidence() through its store barrier before calling apply().
    /// # Errors
    /// Stale before image, unsupported metadata/mount, corrupt blob or I/O failure.
    pub fn prepare(
        self,
        archive: &Archive,
        stage: &Path,
        before: &FileImage,
        desired: &EffectiveDesiredImage,
        cancel: &CancellationToken,
    ) -> Result<Restore, Error> {
        self.0.revalidate_authority()?;
        let stage = restore::stage::Stage::open(stage)?;
        let prepared =
            restore::Prepared::new(&self.0, &archive.0, &stage, before, desired, cancel)?;
        Ok(Restore {
            root: self.0,
            stage,
            prepared,
        })
    }
    /// Reopen recovery from durable expected objects; this cannot apply forward.
    /// # Errors
    /// Changed root/stage, incomplete/foreign evidence or filesystem I/O failure.
    pub fn recovering(
        self,
        stage: &Path,
        before: &FileImage,
        desired: &EffectiveDesiredImage,
        evidence: &StagedPaths,
    ) -> Result<Restore, Error> {
        self.0.revalidate_authority()?;
        let stage = restore::stage::Stage::open(stage)?;
        self.0.check_mount(&stage.folder)?;
        let prepared = restore::Prepared::recovering(before, desired, evidence)?;
        Ok(Restore {
            root: self.0,
            stage,
            prepared,
        })
    }
}
impl Archive {
    /// Open an existing service-owned 0700 directory; never loosen its policy.
    /// # Errors
    /// Unsafe directory/access metadata or filesystem I/O failure.
    pub fn open(path: PathBuf) -> Result<Self, Error> {
        Ok(Self(archive::Archive::open(path)?))
    }
}
impl IgnoreDefaults {
    /// Accept the node's absolute Git global-ignore fallback.
    /// # Errors
    /// The supplied path is relative or lacks a file name.
    pub fn new(path: PathBuf) -> Result<Self, Error> {
        Ok(Self(policy::Defaults::new(path)?))
    }
}
impl Restore {
    /// Return the complete canonical object set to commit before first apply.
    /// # Errors
    /// Incomplete or invalid staged evidence.
    pub fn evidence(&self) -> Result<StagedPaths, Error> {
        self.prepared.evidence()
    }
    /// Apply one pre-journaled path while the node retains the physical lease.
    /// # Errors
    /// Recovery-only operation, stale/unsafe path, cancellation or I/O failure.
    pub fn apply(&self, index: PathCount, cancel: &CancellationToken) -> Result<(), Error> {
        self.root.revalidate_authority()?;
        self.prepared
            .apply(&self.root, &self.stage, index, cancel)?;
        self.root.revalidate_authority()
    }
    /// Undo only an unchanged object written by this restore, retaining conflicts.
    /// # Errors
    /// Unsafe/replaced root, invalid path, cancellation or I/O failure.
    pub fn undo(
        &self,
        index: PathCount,
        cancel: &CancellationToken,
    ) -> Result<restore::Undo, Error> {
        self.root.revalidate_authority()?;
        let outcome = self.prepared.undo(&self.root, &self.stage, index, cancel)?;
        self.root.revalidate_authority()?;
        Ok(outcome)
    }
    /// Observe restart state without inventing a successful rollback.
    /// # Errors
    /// Unsafe/replaced root, invalid path, cancellation or I/O failure.
    pub fn recovery_state(
        &self,
        index: PathCount,
        cancel: &CancellationToken,
    ) -> Result<restore::RecoveryState, Error> {
        self.root.revalidate_authority()?;
        self.prepared.recovery_state(&self.root, index, cancel)
    }
}

/// Canonical digest for confirmation/stale comparison, never JSON formatting.
/// # Errors
/// An image path length cannot be represented by the canonical digest format.
pub fn image_fingerprint(image: &FileImage) -> Result<BlobHash, CheckpointError> {
    super::fingerprint::image(image)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        os::unix::fs::{MetadataExt as _, PermissionsExt as _},
    };
    use vessel_proto::checkpoint::{FileMetadata, GroupId, OrdinaryMode, OwnerId};

    fn private(path: &Path) -> PathBuf {
        fs::create_dir(path).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
        path.to_path_buf()
    }
    #[tokio::test]
    async fn checkpoint_public_worker_api_checks_physical_root_and_reopens_guarded_recovery() {
        let sandbox = crate::testing::Sandbox::new();
        let dir = sandbox.repo("project");
        let git = sandbox.git().await;
        let runtime = tokio::runtime::Handle::current();
        tokio::task::spawn_blocking(move || {
            let admitted = crate::physical::Directory::open(&dir).unwrap();
            let physical = admitted.identity();
            let other = crate::physical::Directory::open(sandbox.path()).unwrap();
            assert!(matches!(
                Tree::open(&dir, other.identity()),
                Err(Error::Refused(CheckpointError::StalePreview))
            ));
            let defaults = IgnoreDefaults::new(sandbox.path().join("global-ignore")).unwrap();
            let selected_archive =
                Archive::open(private(&sandbox.path().join("selected"))).unwrap();
            let cancel = CancellationToken::new();
            let selected = Tree::open(&dir, physical)
                .unwrap()
                .capture(&runtime, &git, &selected_archive, &defaults, &cancel)
                .unwrap();
            fs::write(dir.join("README.md"), b"later edit").unwrap();
            let current_archive = Archive::open(private(&sandbox.path().join("current"))).unwrap();
            let before = Tree::open(&dir, physical)
                .unwrap()
                .capture(&runtime, &git, &current_archive, &defaults, &cancel)
                .unwrap();
            let account = fs::metadata(&dir).unwrap();
            assert_ne!(account.uid(), 0, "non-root permission proof");
            let desired = selected
                .image()
                .effective(
                    before.image(),
                    FileMetadata {
                        owner: OwnerId::new(account.uid()),
                        group: GroupId::new(account.gid()),
                        mode: OrdinaryMode::new(0o600).unwrap(),
                    },
                )
                .unwrap();
            let stage = private(&sandbox.path().join("stage"));
            let restore = Tree::open(&dir, physical)
                .unwrap()
                .prepare(&selected_archive, &stage, before.image(), &desired, &cancel)
                .unwrap();
            // This fixture models the persisted evidence, not a real store commit.
            let saved = serde_json::to_string(&restore.evidence().unwrap()).unwrap();
            let index = PathCount::new(0).unwrap();
            restore.apply(index, &cancel).unwrap();
            assert_eq!(fs::read(dir.join("README.md")).unwrap(), b"hello\n");
            drop(restore);
            let recovered = Tree::open(&dir, physical)
                .unwrap()
                .recovering(
                    &stage,
                    before.image(),
                    &desired,
                    &serde_json::from_str(&saved).unwrap(),
                )
                .unwrap();
            assert_eq!(
                recovered.recovery_state(index, &cancel).unwrap(),
                restore::RecoveryState::Desired
            );
            assert!(recovered.apply(index, &cancel).is_err());
            assert_eq!(
                recovered.undo(index, &cancel).unwrap(),
                restore::Undo::Restored
            );
            assert_eq!(fs::read(dir.join("README.md")).unwrap(), b"later edit");
            admitted.revalidate().unwrap();
        })
        .await
        .unwrap();
    }
}
