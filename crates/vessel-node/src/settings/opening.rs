//! Bind the boot policy and the settings owner to one validated file snapshot.

use std::sync::Arc;
use vessel_proto::parking::ParkAfter;
use vessel_proto::restart::{RestartRecovery, SetRestartRecovery};
use vessel_proto::settings::SetNodeSettings;
use vessel_store::DataDir;

#[derive(Debug)]
pub(crate) struct Opening {
    pub(super) data_dir: DataDir,
    pub(super) saved: SetNodeSettings,
    pub(super) flag: ParkAfter,
    pub(super) flag_given: bool,
    pub(super) kept: Option<Arc<str>>,
    pub(super) later: bool,
}

impl Opening {
    pub(crate) fn read(data_dir: DataDir, flag: ParkAfter, flag_given: bool) -> Self {
        let (saved, kept, later) = match data_dir.read_settings_snapshot() {
            Ok((saved, later)) => (saved, None, later),
            Err(error) => (SetNodeSettings::default(), super::set_aside(&data_dir, &error), false),
        };
        Self { data_dir, saved, flag, flag_given, kept, later }
    }

    pub(crate) fn recovery_policy(&self) -> RestartRecovery {
        recovery_policy(&self.saved)
    }
}

pub(super) fn recovery_policy(saved: &SetNodeSettings) -> RestartRecovery {
    saved.restart_recovery.as_ref().map_or_else(
        RestartRecovery::default,
        SetRestartRecovery::effective,
    )
}
