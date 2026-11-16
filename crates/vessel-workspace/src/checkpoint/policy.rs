//! Discover effective ignore inputs without disabling user excludes or reading HOME.
//!
//! The binary supplies Git's default global-ignore path after resolving its own
//! HOME/XDG environment. Explicit core.excludesFile is resolved by Git itself.
//! Only policy bytes are hashed; external paths/contents are never archived.
use super::{
    Error,
    content::{self, Budget},
    rooted::{Identity, Root},
};
use crate::{Git, git::Limits, regular};
use sha2::{Digest as _, Sha256};
use std::{
    collections::BTreeSet,
    fs::File,
    io,
    path::{Path, PathBuf},
};
use tokio_util::sync::CancellationToken;
use vessel_proto::{
    BlobHash,
    checkpoint::{
        CHECKPOINT_FILE_BYTES, CheckpointError, CheckpointFileBytes, CheckpointPath,
        IgnoreFingerprint, PathCount,
    },
};

/// Every bounded policy input may use at most the same streamed file ceiling.
const POLICY_OUTPUT_BYTES: usize = 64 * 1024;
const POLICY_TOTAL_BYTES: u64 = CHECKPOINT_FILE_BYTES;
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Defaults {
    global: PathBuf,
}
impl Defaults {
    pub(super) fn new(global: PathBuf) -> Result<Self, Error> {
        if !global.is_absolute() || global.file_name().is_none() {
            return Err(CheckpointError::InvalidPath.into());
        }
        Ok(Self { global })
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Inputs {
    local: Vec<CheckpointPath>,
    admin: PathBuf,
    global: PathBuf,
    configuration: Vec<u8>,
}
async fn optional(
    git: &Git,
    root: &Path,
    args: &[&str],
    cancel: &CancellationToken,
) -> Result<Option<Vec<u8>>, Error> {
    let output = git
        .command(root)
        .args(args.iter().copied())
        .limits(Limits::LOCAL.with_stdout(POLICY_OUTPUT_BYTES))
        .cancel_on(cancel.clone())
        .output()
        .await?;
    if output.code == Some(1)
        && !output.stdout_cut
        && output.stdout.is_empty()
        && output.stderr.is_empty()
    {
        return Ok(None);
    }
    Ok(Some(output.checked()?.stdout))
}
fn absolute(root: &Path, raw: &[u8]) -> Result<PathBuf, Error> {
    // Both queries use -z/--null. A newline is a valid path byte, not a delimiter.
    let raw = raw.strip_suffix(&[0]).ok_or(CheckpointError::InvalidPath)?;
    if raw.is_empty() || raw.contains(&0) {
        return Err(CheckpointError::InvalidPath.into());
    }
    let path =
        Path::new(std::str::from_utf8(raw).map_err(|_invalid| CheckpointError::InvalidPath)?);
    Ok(if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    })
}
impl Inputs {
    pub(super) async fn discover(
        git: &Git,
        root: &Path,
        paths: &[CheckpointPath],
        defaults: &Defaults,
        cancel: &CancellationToken,
    ) -> Result<Self, Error> {
        let global = match optional(
            git,
            root,
            &["config", "--null", "--path", "--get", "core.excludesFile"],
            cancel,
        )
        .await?
        {
            Some(path) => absolute(root, &path)?,
            None => defaults.global.clone(),
        };
        let admin = git
            .command(root)
            .args(["rev-parse", "--git-path", "info/exclude"])
            .limits(Limits::LOCAL.with_stdout(POLICY_OUTPUT_BYTES))
            .cancel_on(cancel.clone())
            .output()
            .await?
            .checked()?
            .stdout;
        // rev-parse lacks -z on the supported Git floor. Refuse embedded line
        // delimiters rather than guessing which bytes belong to the directory.
        let admin = admin
            .strip_suffix(b"\n")
            .ok_or(CheckpointError::InvalidPath)?;
        if admin.contains(&b'\n') || admin.contains(&0) {
            return Err(CheckpointError::InvalidPath.into());
        }
        let mut terminated = admin.to_vec();
        terminated.push(0);
        let admin = absolute(root, &terminated)?;
        let configuration=optional(git,root,&["config","--null","--get-regexp",r"^(core\.(excludesfile|ignorecase|sparsecheckout|sparsecheckoutcone|precomposeunicode)|extensions\.worktreeconfig)$"],cancel).await?.unwrap_or_default();
        let mut local = BTreeSet::new();
        let _inserted = local.insert(policy_path(".gitignore")?);
        for path in paths {
            let mut ancestor = Path::new(path.relative().as_str()).parent();
            while let Some(parent) = ancestor.filter(|parent| !parent.as_os_str().is_empty()) {
                let name = parent.join(".gitignore");
                let _inserted = local.insert(policy_path(
                    name.to_str().ok_or(CheckpointError::InvalidPath)?,
                )?);
                let _count = PathCount::new(local.len())?;
                ancestor = parent.parent();
            }
        }
        Ok(Self {
            local: local.into_iter().collect(),
            admin,
            global,
            configuration,
        })
    }
    /// The admitted blocking worker owns every file handle until these reads finish.
    pub(super) fn fingerprint(
        &self,
        root: &Root,
        cancel: &CancellationToken,
    ) -> Result<IgnoreFingerprint, Error> {
        let mut hash = Sha256::new();
        hash.update(b"vessel-checkpoint-ignore-v1\0");
        feed_name(&mut hash, &self.configuration)?;
        let mut total = 0_u64;
        for path in &self.local {
            feed_name(&mut hash, path.relative().as_str().as_bytes())?;
            feed_file(&mut hash, root.file(path)?, &mut total, cancel)?;
        }
        for path in [&self.admin, &self.global] {
            feed_name(
                &mut hash,
                path.to_str()
                    .ok_or(CheckpointError::InvalidPath)?
                    .as_bytes(),
            )?;
            let file = match regular::open(path) {
                Ok(Some(file)) => Some(file),
                Ok(None) => return Err(CheckpointError::UnsafeEntry.into()),
                Err(error) if error.kind() == io::ErrorKind::NotFound => None,
                Err(error) => return Err(error.into()),
            };
            feed_file(&mut hash, file, &mut total, cancel)?;
        }
        root.revalidate()?;
        Ok(IgnoreFingerprint::new(BlobHash::from_bytes(
            hash.finalize().into(),
        )))
    }
}
fn policy_path(text: &str) -> Result<CheckpointPath, Error> {
    Ok(CheckpointPath::try_from(
        text.parse::<vessel_proto::workspace::RelPath>()
            .map_err(|_invalid| CheckpointError::InvalidPath)?,
    )?)
}
fn feed_name(hash: &mut Sha256, name: &[u8]) -> Result<(), Error> {
    hash.update(
        u64::try_from(name.len())
            .map_err(|_large| CheckpointError::InvalidPath)?
            .to_le_bytes(),
    );
    hash.update(name);
    Ok(())
}
fn feed_file(
    hash: &mut Sha256,
    file: Option<File>,
    total: &mut u64,
    cancel: &CancellationToken,
) -> Result<(), Error> {
    let Some(mut file) = file else {
        hash.update(b"absent\0");
        return Ok(());
    };
    let before = Identity::of(&file.metadata()?);
    let remaining = CheckpointFileBytes::new(POLICY_TOTAL_BYTES.saturating_sub(*total))?;
    let (digest, bytes) = content::transfer(
        &mut file,
        &mut io::sink(),
        Budget::ImageRemaining(remaining),
        cancel,
    )?;
    if Identity::of(&file.metadata()?) != before {
        return Err(CheckpointError::CaptureUnstable.into());
    }
    *total = total
        .checked_add(bytes.get())
        .ok_or(CheckpointError::ImageTooLarge)?;
    hash.update(b"present\0");
    hash.update(digest.as_bytes());
    hash.update(bytes.get().to_le_bytes());
    Ok(())
}
