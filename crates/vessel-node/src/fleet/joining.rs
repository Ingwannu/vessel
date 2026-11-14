//! Joining and leaving a fleet, shared by the offline CLI and Admin RPC.
//!
//! A Tokio admission permit spans network pairing. An OS try-lock is held
//! only inside blocking preflight and commit, never across network awaits.
//! Commit rechecks configuration: a racing CLI may consume another link,
//! but cannot replace membership. Post-pair failure is an uncertain outcome,
//! never an instruction to retry the same single-use link.

use super::files::{self, FileError, FleetFiles};
use super::member::Membership;
use super::mutation::Reservation;
use chrono::Utc;
use tokio::sync::OwnedSemaphorePermit;
use vessel_client::CredentialError;
use vessel_client::secure::{DialError, ReachError, RouteTimeouts, TlsError, WsDialer, pair_over};
use vessel_proto::{NodeInfo, PairingOffer, Scope};

/// A node joined a fleet.
#[derive(Debug, Clone)]
pub struct Joined {
    /// The home, as it introduced itself.
    pub home: NodeInfo,
    /// The route the pairing took.
    pub route: String,
}

/// Why joining did not complete.
#[derive(Debug, thiserror::Error)]
pub enum JoinError {
    /// A home cannot follow another home.
    #[error("this node is a fleet's home (it has fleet.toml); a home does not join another")]
    Home,
    /// Replacing a membership requires an explicit leave.
    #[error("this node is already a member of a fleet; `vessel fleet leave` first")]
    Member,
    /// No TLS for a secure route.
    #[error("TLS for the home's relays: {0}")]
    Tls(#[source] TlsError),
    /// The home could not be reached or refused pairing.
    #[error("{0}")]
    Reach(#[source] ReachError<DialError>),
    /// Membership could not be read or written, or its lock is busy.
    #[error(transparent)]
    File(#[from] FileError),
    /// The credential could not be written.
    #[error(transparent)]
    Credential(#[from] CredentialError),
    /// Blocking membership work failed.
    #[error("fleet membership work failed")]
    Task(#[source] tokio::task::JoinError),
    /// The invitation was redeemed, but membership did not commit.
    #[error(
        "the invitation was redeemed but membership could not be saved; inspect fleet status before making another invitation"
    )]
    AfterPairing(#[source] CommitError),
}

/// Why a redeemed invitation did not become committed membership.
#[derive(Debug, thiserror::Error)]
pub enum CommitError {
    /// Another offline operation changed the role while pairing.
    #[error("this machine's fleet role changed while pairing")]
    RoleChanged,
    /// Commit admission or a membership write failed.
    #[error(transparent)]
    File(#[from] FileError),
    /// The new credential could not be persisted.
    #[error(transparent)]
    Credential(#[from] CredentialError),
    /// Blocking commit did not return normally.
    #[error("fleet membership commit did not finish normally")]
    Task(#[source] tokio::task::JoinError),
}

/// Redeem the home's single-use invitation and persist membership. The
/// caller confirms its identity and chosen grant before calling. Never put
/// the link on the command line or automatically retry uncertain outcomes.
///
/// # Errors
/// See [`JoinError`]. A busy membership lock refuses before pairing.
pub async fn join(
    files: &FleetFiles,
    offer: &PairingOffer,
    device_name: &str,
    grant: Vec<Scope>,
) -> Result<Joined, JoinError> {
    join_reserved(files, offer, device_name, grant, None, || {}).await
}

pub(crate) async fn join_reserved(
    files: &FleetFiles,
    offer: &PairingOffer,
    device_name: &str,
    grant: Vec<Scope>,
    permit: Option<OwnedSemaphorePermit>,
    after_commit: impl FnOnce() + Send + 'static,
) -> Result<Joined, JoinError> {
    let preflight = files.clone();
    let permit = tokio::task::spawn_blocking(move || {
        let _reservation = Reservation::take(&preflight, None)?;
        available(&preflight).map_err(JoinError::from)?;
        Ok::<_, JoinError>(permit)
    })
    .await
    .map_err(JoinError::Task)??;
    let dialer =
        WsDialer::for_routes(offer.direct.iter().chain(&offer.relays)).map_err(JoinError::Tls)?;
    let (paired, route) = pair_over(&dialer, offer, device_name, RouteTimeouts::default())
        .await
        .map_err(JoinError::Reach)?;
    let membership = Membership {
        home: offer.node,
        key: offer.key,
        relays: offer.relays.clone(),
        direct: offer.direct.clone(),
        joined_at: Utc::now(),
        grant,
    };
    let files = files.clone();
    let credential = paired.credential;
    tokio::task::spawn_blocking(move || {
        let result = commit(&files, &membership, &credential, permit);
        // A vanished RPC must not hide a completed/partial disk commit
        // from the existing supervisor. Offline CLI still relies on polling.
        after_commit();
        result
    })
    .await
    .map_err(|error| JoinError::AfterPairing(CommitError::Task(error)))?
    .map_err(JoinError::AfterPairing)?;
    Ok(Joined {
        home: paired.node,
        route: route.to_string(),
    })
}

fn commit(
    files: &FleetFiles,
    membership: &Membership,
    credential: &vessel_client::DeviceCredential,
    permit: Option<OwnedSemaphorePermit>,
) -> Result<(), CommitError> {
    let _reservation = Reservation::take(files, permit)?;
    // A racing CLI or editor can finish during network I/O. The first
    // committed configuration wins; never overwrite it after redemption.
    match available(files) {
        Ok(()) => {}
        Err(AdmissionError::Home | AdmissionError::Member) => return Err(CommitError::RoleChanged),
        Err(AdmissionError::File(error)) => return Err(CommitError::File(error)),
    }
    // Credential first: a member is never visible without its key. A
    // failed second write can leave an unused owner-only credential; leave
    // removes it, and restart sees no membership to follow.
    credential.write(&files.credential())?;
    files::write_json(&files.membership(), membership)?;
    Ok(())
}

/// Remove membership and this node's credential on the home, keeping
/// materialized files. Whether a membership existed. Blocks.
///
/// # Errors
/// The membership lock is busy, or a file could not be inspected/removed.
pub fn leave(files: &FleetFiles) -> Result<bool, FileError> {
    leave_reserved(files, None)
}

pub(crate) fn leave_reserved(
    files: &FleetFiles,
    permit: Option<OwnedSemaphorePermit>,
) -> Result<bool, FileError> {
    let _reservation = Reservation::take(files, permit)?;
    let was = files
        .membership()
        .try_exists()
        .map_err(|error| FileError::io(&files.membership(), error))?;
    files::remove(&files.membership())?;
    files::remove(&files.credential())?;
    Ok(was)
}

enum AdmissionError {
    Home,
    Member,
    File(FileError),
}

impl From<FileError> for AdmissionError {
    fn from(error: FileError) -> Self {
        Self::File(error)
    }
}

impl From<AdmissionError> for JoinError {
    fn from(error: AdmissionError) -> Self {
        match error {
            AdmissionError::Home => Self::Home,
            AdmissionError::Member => Self::Member,
            AdmissionError::File(error) => Self::File(error),
        }
    }
}

fn available(files: &FleetFiles) -> Result<(), AdmissionError> {
    if files
        .sources()
        .try_exists()
        .map_err(|error| FileError::io(files.sources(), error))?
    {
        return Err(AdmissionError::Home);
    }
    if files
        .membership()
        .try_exists()
        .map_err(|error| FileError::io(&files.membership(), error))?
    {
        return Err(AdmissionError::Member);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_client::DeviceCredential;
    use vessel_proto::{DeviceId, DeviceSecret, NodeId, NodePublicKey};

    fn proposed() -> (Membership, DeviceCredential) {
        let home = NodeId::random();
        let key = NodePublicKey::from_bytes([7; 32]);
        (
            Membership {
                home,
                key,
                relays: Vec::new(),
                direct: Vec::new(),
                joined_at: chrono::DateTime::UNIX_EPOCH,
                grant: Vec::new(),
            },
            DeviceCredential {
                node: home,
                key,
                device: DeviceId::random(),
                secret: DeviceSecret::from_bytes([8; 32]),
            },
        )
    }

    #[test]
    fn a_role_committed_while_pairing_is_never_overwritten() {
        for home_wins in [true, false] {
            let temp = tempfile::tempdir().unwrap();
            let files = FleetFiles::new(&vessel_store::DataDir::new(temp.path().join("data")));
            files::create_private_dir(files.dir()).unwrap();
            assert!(available(&files).is_ok());
            let (first, credential) = proposed();
            if home_wins {
                let _sources = super::super::init(
                    &files,
                    &super::super::FleetInit {
                        instructions: Some(temp.path().join("instructions")),
                        skills: None,
                        skip: Vec::new(),
                    },
                )
                .unwrap();
            } else {
                commit(&files, &first, &credential, None).unwrap();
            }
            let (racing, other_credential) = proposed();
            assert!(matches!(
                commit(&files, &racing, &other_credential, None),
                Err(CommitError::RoleChanged)
            ));
            if home_wins {
                assert!(files.sources().is_file());
                assert!(!files.membership().exists());
                assert!(!files.credential().exists());
            } else {
                assert_eq!(
                    files::read_json::<Membership>(&files.membership()).unwrap(),
                    Some(first)
                );
                assert_eq!(
                    DeviceCredential::read(&files.credential()).unwrap(),
                    credential
                );
            }
        }
    }

    #[test]
    fn an_unused_credential_after_an_incomplete_commit_is_not_membership_and_leave_removes_it() {
        let temp = tempfile::tempdir().unwrap();
        let files = FleetFiles::new(&vessel_store::DataDir::new(temp.path().join("data")));
        files::create_private_dir(files.dir()).unwrap();
        let (_, credential) = proposed();
        credential.write(&files.credential()).unwrap();
        assert_eq!(
            super::super::read_role(&files).unwrap(),
            vessel_proto::FleetRole::Standalone
        );
        assert!(!leave(&files).unwrap());
        assert!(!files.credential().exists());
        assert_eq!(
            super::super::read_role(&files).unwrap(),
            vessel_proto::FleetRole::Standalone
        );
    }
}
