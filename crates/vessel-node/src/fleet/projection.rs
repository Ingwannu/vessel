//! Credential-free fleet read projection, on a blocking task at the RPC edge.
use super::files::{FileError, FleetFiles};
use super::member::Membership;
use super::state::FleetState;
use serde::de::DeserializeOwned;
use std::collections::BTreeSet;
use std::io::Read as _;
use std::path::Path;
use vessel_proto::{FleetApplied, FleetRole, FleetSyncProblem, FleetSyncStatus};

/// A whole-result refusal; status never truncates a report into false emptiness.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ProjectionError {
    #[error(transparent)]
    File(#[from] FileError),
    #[error("fleet state is not valid JSON")]
    Decode(#[source] serde_json::Error),
    #[error("fleet status exceeds its read or item budget")]
    TooLarge,
    #[error("fleet status exceeds its serialized reply budget")]
    Reply(#[source] serde_json::Error),
}

// File bytes bound decoding before item limits are checked. BundlePath
// already caps a path at 1024 bytes; the reply cap also covers JSON escaping.
pub(super) const STATE_BYTES: u64 = 1 << 20;
const MEMBERSHIP_BYTES: u64 = 64 << 10;
pub(super) const MAX_CONFLICTS: usize = 512;
const LOCAL_CONFLICTS: usize = 256;
const REPLY_BYTES: usize = (1 << 20) - 1024; // Reserve the RPC envelope.

pub(crate) fn read(files: &FleetFiles) -> Result<FleetSyncStatus, ProjectionError> {
    let role = role(files)?;
    let state = read_json::<FleetState>(&files.state(), STATE_BYTES)?.unwrap_or_default();
    if state.conflicts.len() > LOCAL_CONFLICTS {
        return Err(ProjectionError::TooLarge);
    }
    let mut problems = Vec::new();
    if state.error.is_some() {
        problems.push(FleetSyncProblem::OperationFailed);
    }
    if !state.failures.is_empty() {
        problems.push(FleetSyncProblem::ApplyFailed);
    }
    if !state.problems.is_empty() {
        problems.push(FleetSyncProblem::IncompleteSources);
    }
    if state.grant.is_some() {
        problems.push(FleetSyncProblem::GrantFailed);
    }
    // Several profiles can report the same bundle path. A read-only summary
    // needs the path once, not absolute destination names or incoming files.
    let conflicts = state
        .conflicts
        .into_iter()
        .map(|entry| entry.bundle)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    Ok(FleetSyncStatus {
        role,
        local: FleetApplied {
            generation: state.generation,
            applied_at: state.applied_at,
            contacted_at: state.contacted_at,
            conflicts,
            problems,
        },
        // The service adds existing per-member reports only for a local
        // authenticated Admin. No credentials or secret resolution here.
        members: None,
    })
}

/// Configured role only, without state or secret resolution. Blocks.
pub(crate) fn role(files: &FleetFiles) -> Result<FleetRole, ProjectionError> {
    let is_home = files
        .sources()
        .try_exists()
        .map_err(|error| FileError::io(files.sources(), error))?;
    let role = if is_home {
        FleetRole::Home
    } else if let Some(membership) = read_json::<Membership>(&files.membership(), MEMBERSHIP_BYTES)?
    {
        FleetRole::Member {
            home: membership.home,
            joined_at: membership.joined_at,
        }
    } else {
        FleetRole::Standalone
    };
    Ok(role)
}

pub(super) fn read_json<T: DeserializeOwned>(
    path: &Path,
    bytes: u64,
) -> Result<Option<T>, ProjectionError> {
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(FileError::io(path, error).into()),
    };
    let mut text = Vec::new();
    // One sentinel byte detects oversize even if the file grows while read.
    let _read = file
        .take(bytes.saturating_add(1))
        .read_to_end(&mut text)
        .map_err(|error| FileError::io(path, error))?;
    if u64::try_from(text.len()).map_or(true, |size| size > bytes) {
        return Err(ProjectionError::TooLarge);
    }
    serde_json::from_slice(&text)
        .map(Some)
        .map_err(ProjectionError::Decode)
}

/// Check the whole payload before returning it; no partial or empty stand-in.
pub(crate) fn finish(status: FleetSyncStatus) -> Result<FleetSyncStatus, ProjectionError> {
    let mut conflicts = status.local.conflicts.len();
    if conflicts > MAX_CONFLICTS {
        return Err(ProjectionError::TooLarge);
    }
    if let Some(members) = &status.members {
        for member in members {
            conflicts = conflicts.saturating_add(member.conflicts.len());
            if conflicts > MAX_CONFLICTS {
                return Err(ProjectionError::TooLarge);
            }
        }
    }
    within_reply(&status)?;
    Ok(status)
}

/// Whether `reply` encodes within a reply's byte budget, checked before it
/// is returned (`fleet.status`, `fleet.published`): no partial stand-in.
pub(super) fn within_reply<T: serde::Serialize>(reply: &T) -> Result<(), ProjectionError> {
    // This sink allocates nothing and can fail only at the byte budget.
    let mut budget = ReplyBudget {
        remaining: REPLY_BYTES,
    };
    serde_json::to_writer(&mut budget, reply).map_err(ProjectionError::Reply)
}

struct ReplyBudget {
    remaining: usize,
}
impl std::io::Write for ReplyBudget {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        self.remaining = self
            .remaining
            .checked_sub(buffer.len())
            .ok_or_else(|| std::io::Error::other("fleet status reply budget exceeded"))?;
        Ok(buffer.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fleet::files;
    use crate::fleet::state::{ConflictEntry, ConflictWhy, FailureEntry};
    use chrono::Utc;

    #[test]
    fn status_refuses_oversize_files_instead_of_decoding_or_truncating_them() {
        let temp = tempfile::tempdir().unwrap();
        let files = FleetFiles::new(&vessel_store::DataDir::new(temp.path().join("data")));
        files::create_private_dir(files.dir()).unwrap();
        let mut exact = b"{}".to_vec();
        exact.resize(usize::try_from(STATE_BYTES).unwrap(), b' ');
        std::fs::write(files.state(), &exact).unwrap();
        assert_eq!(read(&files).unwrap().role, FleetRole::Standalone);
        exact.push(b' ');
        std::fs::write(files.state(), &exact).unwrap();
        assert!(matches!(read(&files), Err(ProjectionError::TooLarge)));
        std::fs::remove_file(files.state()).unwrap();
        std::fs::File::create(files.membership())
            .unwrap()
            .set_len(MEMBERSHIP_BYTES + 1)
            .unwrap();
        assert!(matches!(role(&files), Err(ProjectionError::TooLarge)));
    }

    #[test]
    fn status_is_all_or_nothing_for_conflict_and_serialized_reply_budgets() {
        let temp = tempfile::tempdir().unwrap();
        let files = FleetFiles::new(&vessel_store::DataDir::new(temp.path().join("data")));
        let mut status = read(&files).unwrap();
        let conflict = ConflictEntry {
            bundle: "AGENTS.md".parse().unwrap(),
            path: temp.path().join("profile/AGENTS.md"),
            kind: ConflictWhy::Edited,
            incoming: None,
        };
        files::create_private_dir(files.dir()).unwrap();
        files::write_json(
            &files.state(),
            &FleetState {
                conflicts: vec![conflict; LOCAL_CONFLICTS + 1],
                ..FleetState::default()
            },
        )
        .unwrap();
        assert!(matches!(read(&files), Err(ProjectionError::TooLarge)));
        let path = "AGENTS.md".parse().unwrap();
        let mut member = vessel_proto::FleetMember {
            device: vessel_proto::DeviceId::random(),
            label: "member".to_owned(),
            applied: None,
            conflicts: vec![path; MAX_CONFLICTS],
            reported_at: None,
        };
        status.members = Some(vec![member.clone()]);
        assert!(finish(status.clone()).is_ok());
        status.local.conflicts = vec!["AGENTS.md".parse().unwrap()];
        assert!(matches!(
            finish(status.clone()),
            Err(ProjectionError::TooLarge)
        ));
        status.local.conflicts.clear();
        member.conflicts.clear();
        member.label = "m".repeat(REPLY_BYTES);
        status.members = Some(vec![member]);
        assert!(matches!(finish(status), Err(ProjectionError::Reply(_))));
    }

    #[test]
    fn read_projection_never_resolves_secrets_or_exposes_private_file_errors() {
        let temp = tempfile::tempdir().unwrap();
        let files = FleetFiles::new(&vessel_store::DataDir::new(temp.path().join("data")));
        files::create_private_dir(files.dir()).unwrap();
        let at = Utc::now();
        let home = vessel_proto::NodeId::random();
        let membership = Membership {
            home,
            key: *vessel_secure::NodeKeypair::generate().unwrap().public(),
            relays: Vec::new(),
            direct: Vec::new(),
            joined_at: at,
            grant: Vec::new(),
        };
        files::write_json(&files.membership(), &membership).unwrap();
        let path = temp.path().join("a-private-profile/AGENTS.md");
        let bundle: vessel_proto::BundlePath = "AGENTS.md".parse().unwrap();
        let conflict = ConflictEntry {
            bundle: bundle.clone(),
            path: path.clone(),
            kind: ConflictWhy::Edited,
            incoming: Some(path.with_extension("incoming")),
        };
        let state = FleetState {
            generation: Some(vessel_proto::Generation::new(7)),
            applied_at: Some(at),
            contacted_at: Some(at),
            conflicts: vec![conflict.clone(), conflict],
            failures: vec![FailureEntry {
                path,
                error: "private failure text".to_owned(),
            }],
            problems: vec!["private source text".to_owned()],
            error: Some("private connection text".to_owned()),
            grant: Some("private grant text".to_owned()),
            ..FleetState::default()
        };
        files::write_json(&files.state(), &state).unwrap();
        // Neither file is needed for a local apply-history projection.
        std::fs::write(files.manifest(), "invalid manifest fixture").unwrap();
        std::fs::write(files.secrets(), "invalid secrets fixture").unwrap();
        assert_eq!(
            read(&files).unwrap(),
            FleetSyncStatus {
                role: FleetRole::Member {
                    home,
                    joined_at: at
                },
                local: FleetApplied {
                    generation: state.generation,
                    applied_at: Some(at),
                    contacted_at: Some(at),
                    conflicts: vec![bundle],
                    problems: vec![
                        FleetSyncProblem::OperationFailed,
                        FleetSyncProblem::ApplyFailed,
                        FleetSyncProblem::IncompleteSources,
                        FleetSyncProblem::GrantFailed
                    ],
                },
                members: None,
            }
        );
    }
}
