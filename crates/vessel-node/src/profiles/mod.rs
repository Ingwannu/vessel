//! The node's harness profiles (ADR 0011): `profiles.toml` in the data
//! directory, `profile.list` and `profile.reload`, and what a new session is
//! launched with (`launch`).
//!
//! [Decision Log]
//! - Purpose: a session runs with the account and configuration its
//!   profile names; the node owner defines the names, clients only pick.
//! - Reading: at start a broken file stops the node, so it never runs
//!   sessions under a guess. On `profile.reload` a broken file leaves the
//!   profiles in effect and the error names the profile, key and position
//!   (never a value: `vessel_harness::ProfilesError`). A file other users
//!   can read is used, with a warning, as for `token`: the owner wrote it
//!   and decides.
//! - State: the parsed set behind a `std` `RwLock`, swapped whole on reload
//!   and cloned out under the lock (never held across `.await`), so a
//!   session resolves against one consistent set and a reload never
//!   changes a session already started.
//! - Alternatives: watching the file (a half-saved file would be read);
//!   reading it per session (a broken edit would fail every new session
//!   instead of the one reload call).
//!
//! [Decision Log: added agents] (protocol 11, ADR 0032)
//! - `agents.toml` is read with `profiles.toml`, first, and the two are
//!   swapped in together: a profile may extend an added agent, so a reload
//!   keeps both files' set or neither. A broken `agents.toml`, or one
//!   another user could change, stops the node at start and leaves the
//!   set in effect on reload, as `profiles.toml` does.
//! - `agent.add` and `agent.remove` change the file through `edit`, one at
//!   a time (a `tokio` lock held across the read, the check and the
//!   write), so two of them never lose each other's change.

pub(crate) mod launch;
pub(crate) mod moving;

use std::path::PathBuf;
use std::sync::{Arc, PoisonError, RwLock};

use vessel_harness::{
    AddedAgents, AgentsError, HarnessEnv, Profile, Profiles, ProfilesError, parse_agents,
    parse_profiles_with,
};
use vessel_proto::{HarnessRef, ProfileInfo, ProfileName};
use vessel_store::{AgentsFileError, DataDir, StoreError};

/// Why `profiles.toml` or `agents.toml` could not be used.
#[derive(Debug, thiserror::Error)]
pub enum ProfilesFileError {
    /// It could not be read.
    #[error("cannot read profiles.toml")]
    Read(#[source] StoreError),
    /// It does not parse; the message says where.
    #[error(transparent)]
    Parse(#[from] ProfilesError),
    /// `agents.toml` could not be read, or another user could change it.
    #[error(transparent)]
    ReadAgents(#[from] AgentsFileError),
    /// `agents.toml` does not parse; the message says where.
    #[error(transparent)]
    ParseAgents(#[from] AgentsError),
}

/// The profiles in effect on this node.
#[derive(Debug)]
pub(crate) struct NodeProfiles {
    dir: DataDir,
    /// The account harnesses run as: what `~/` expands to, where default
    /// homes are, and the home variables profiles inherit. Given by whoever
    /// started the node (`Config::harness_env`), never read here.
    env: HarnessEnv,
    current: RwLock<Arc<Profiles>>,
    /// Held across a change of `agents.toml` (`edit`).
    pub(crate) editing: tokio::sync::Mutex<()>,
}

impl NodeProfiles {
    /// Read `profiles.toml` (none means the default profiles only). Blocks
    /// on the file system.
    ///
    /// # Errors
    /// The file cannot be read or does not parse.
    pub(crate) fn load(dir: DataDir, env: HarnessEnv) -> Result<Self, ProfilesFileError> {
        let profiles = read(&dir, env.user_home())?;
        log_loaded(&profiles);
        Ok(Self {
            dir,
            env,
            current: RwLock::new(Arc::new(profiles)),
            editing: tokio::sync::Mutex::new(()),
        })
    }

    /// The profiles in effect: the defaults, then the defined ones, only
    /// `harness`'s if given. Names, labels and variable names only.
    /// The default profile of `harness`: one of the four's, or an added
    /// agent's when the node has it.
    pub(crate) fn default_of(&self, harness: &HarnessRef) -> Option<Profile> {
        self.snapshot().default_of(harness)
    }

    /// The set in effect now: the profiles and the added agents.
    pub(crate) fn current(&self) -> Arc<Profiles> {
        self.snapshot()
    }

    /// The data directory the two files are in.
    pub(crate) const fn dir(&self) -> &DataDir {
        &self.dir
    }

    pub(crate) fn list(&self, harness: Option<&HarnessRef>) -> Vec<ProfileInfo> {
        self.snapshot()
            .all()
            .filter(|profile| harness.is_none_or(|harness| *harness == profile.harness()))
            .map(Profile::info)
            .collect()
    }

    /// Read the file again and use it; a file that cannot be used leaves
    /// the profiles in effect. Blocks on the file system.
    ///
    /// # Errors
    /// The file cannot be read or does not parse.
    pub(crate) fn reload(&self) -> Result<Vec<ProfileInfo>, ProfilesFileError> {
        let profiles = read(&self.dir, self.env.user_home())?;
        log_loaded(&profiles);
        *self.current.write().unwrap_or_else(PoisonError::into_inner) = Arc::new(profiles);
        Ok(self.list(None))
    }

    /// Use `profiles` from now on: a change of `agents.toml` already read
    /// and checked (`added`).
    pub(crate) fn install(&self, profiles: Profiles) {
        log_loaded(&profiles);
        *self.current.write().unwrap_or_else(PoisonError::into_inner) = Arc::new(profiles);
    }

    /// The profile named `name`, a defined one or a harness's default.
    pub(crate) fn get(&self, name: &ProfileName) -> Option<Profile> {
        self.snapshot().get(name).cloned()
    }

    /// What `~/` expands to, and where default homes are.
    pub(crate) fn user_home(&self) -> &std::path::Path {
        self.env.user_home()
    }

    /// The account harnesses run as on this node.
    pub(crate) const fn env(&self) -> &HarnessEnv {
        &self.env
    }

    /// Where `profile`'s harness keeps its configuration, logins and
    /// sessions on this node: the one place node code resolves a home
    /// (moving a session, the fleet's targets, transfers). `None` when the
    /// profile's home variable is relative.
    pub(crate) fn home(&self, profile: &Profile) -> Option<PathBuf> {
        self.env.home(profile)
    }

    fn snapshot(&self) -> Arc<Profiles> {
        Arc::clone(&self.current.read().unwrap_or_else(PoisonError::into_inner))
    }
}

fn read(dir: &DataDir, user_home: &std::path::Path) -> Result<Profiles, ProfilesFileError> {
    let added = match dir.read_agents()? {
        Some(file) => parse_agents(&file.text, Some(user_home))?,
        None => AddedAgents::default(),
    };
    with_agents(dir, user_home, added)
}

/// `profiles.toml` read beside `added`: what a change of `agents.toml`
/// checks before it is written.
pub(crate) fn with_agents(
    dir: &DataDir,
    user_home: &std::path::Path,
    added: AddedAgents,
) -> Result<Profiles, ProfilesFileError> {
    match dir.read_profiles().map_err(ProfilesFileError::Read)? {
        Some(file) => Ok(parse_profiles_with(&file.text, Some(user_home), added)?),
        None => Ok(parse_profiles_with("", Some(user_home), added)?),
    }
}

fn log_loaded(profiles: &Profiles) {
    let names: Vec<&str> = profiles
        .defined()
        .map(|profile| profile.name().as_str())
        .collect();
    let agents: Vec<&str> = profiles
        .added()
        .all()
        .map(|agent| agent.name().as_str())
        .collect();
    tracing::info!(profiles = ?names, agents = ?agents, "harness profiles loaded");
}
