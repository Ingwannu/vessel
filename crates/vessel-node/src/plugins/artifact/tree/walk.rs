use super::*;
use std::collections::BTreeSet;

enum Unread {
    Directory {
        executable: bool,
    },
    File {
        bytes: u64,
        executable: bool,
    },
    Link {
        target: LinkTarget,
        resolved: PathBuf,
    },
}
struct Entry {
    path: PackagePath,
    stamp: file::Stamp,
    kind: Unread,
}

#[derive(PartialEq, Eq)]
pub(super) struct Pass {
    pub(super) digest: TreeDigest,
    evidence: Vec<(String, file::Stamp)>,
}
impl Pass {
    pub(super) fn read(scope: &Scope) -> Result<Self, Failure> {
        scope.check_root()?;
        let mut plan = Vec::new();
        collect(scope, "", &mut Budget::default(), &mut plan)?;
        check_links(scope, &plan)?;
        plan.sort_by(|left, right| left.path.as_str().cmp(right.path.as_str()));
        let mut records = Vec::with_capacity(plan.len());
        let mut evidence = Vec::with_capacity(plan.len());
        for entry in plan {
            scope.check()?;
            let full = scope.root.path().join(entry.path.as_str());
            let kind = match entry.kind {
                Unread::Directory { executable } => Kind::Directory { executable },
                Unread::File { bytes, executable } => {
                    let probe = file::Probe::prepare(full.clone(), scope.device)?;
                    if probe.stamp() != entry.stamp {
                        return Err(Failure::Changed);
                    }
                    let hash = probe.read_until(&scope.cancellation, scope.deadline)?;
                    Kind::File {
                        bytes,
                        executable,
                        hash,
                    }
                }
                Unread::Link { target, .. } => Kind::Link { target },
            };
            let after = fs::symlink_metadata(&full).map_err(file::io_failure)?;
            if file::Stamp::from(&after) != entry.stamp {
                return Err(Failure::Changed);
            }
            evidence.push((entry.path.as_str().to_owned(), entry.stamp));
            records.push(Record {
                path: entry.path,
                kind,
            });
        }
        scope.check_root()?;
        Ok(Self {
            digest: TreeDigest::new(records)?,
            evidence,
        })
    }
}

fn collect(
    scope: &Scope,
    parent: &str,
    budget: &mut Budget,
    plan: &mut Vec<Entry>,
) -> Result<(), Failure> {
    scope.check()?;
    let folder = scope.root.path().join(parent);
    for found in fs::read_dir(&folder).map_err(file::io_failure)? {
        scope.check()?;
        let found = found.map_err(file::io_failure)?;
        let name = found
            .file_name()
            .into_string()
            .map_err(|name| Failure::Name(PrivateName(name)))?;
        let relative = if parent.is_empty() {
            name
        } else {
            format!("{parent}/{name}")
        };
        let path = package_path(&relative)?;
        let full = scope.root.path().join(path.as_str());
        let metadata = fs::symlink_metadata(&full).map_err(file::io_failure)?;
        let kind = classify(scope, &full, &metadata)?;
        let bytes = match kind {
            Unread::File { bytes, .. } => bytes,
            _ => 0,
        };
        budget.admit_shape(&path, bytes)?;
        let descend = matches!(kind, Unread::Directory { .. });
        plan.push(Entry {
            path,
            stamp: file::Stamp::from(&metadata),
            kind,
        });
        if descend {
            collect(scope, &relative, budget, plan)?;
        }
    }
    Ok(())
}

fn classify(scope: &Scope, full: &Path, metadata: &fs::Metadata) -> Result<Unread, Failure> {
    if metadata.uid() != nix::unistd::geteuid().as_raw() || metadata.dev() != scope.device {
        return Err(Failure::Policy);
    }
    let executable = metadata.mode() & 0o100 != 0;
    if metadata.file_type().is_symlink() {
        let spelling = fs::read_link(full)
            .map_err(file::io_failure)?
            .into_os_string()
            .into_string()
            .map_err(|name| Failure::Name(PrivateName(name)))?;
        let target = LinkTarget::new(&spelling)?;
        let resolved = fs::canonicalize(full).map_err(file::io_failure)?;
        if !resolved.starts_with(scope.root.path()) {
            return Err(Failure::Policy);
        }
        return Ok(Unread::Link { target, resolved });
    }
    if !metadata.is_dir() && !metadata.is_file() {
        return Err(Failure::Policy);
    }
    // No traversal through a directory link, even when it happens to point in.
    if fs::canonicalize(full).map_err(file::io_failure)? != full {
        return Err(Failure::Policy);
    }
    if metadata.is_dir() {
        directory(metadata, scope.device)?;
        Ok(Unread::Directory { executable })
    } else if metadata.is_file() && metadata.mode() & 0o022 == 0 {
        Ok(Unread::File {
            bytes: metadata.len(),
            executable,
        })
    } else {
        Err(Failure::Policy)
    }
}

fn check_links(scope: &Scope, plan: &[Entry]) -> Result<(), Failure> {
    let mut covered = BTreeSet::new();
    for entry in plan {
        scope.check()?;
        if !matches!(entry.kind, Unread::Link { .. }) {
            let _inserted = covered.insert(entry.path.as_str());
        }
    }
    for entry in plan {
        scope.check()?;
        if let Unread::Link { resolved, .. } = &entry.kind {
            let admitted = resolved == scope.root.path()
                || resolved
                    .strip_prefix(scope.root.path())
                    .ok()
                    .and_then(Path::to_str)
                    .is_some_and(|relative| covered.contains(relative));
            if !admitted {
                return Err(Failure::Policy);
            }
        }
    }
    Ok(())
}
