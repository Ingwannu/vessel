//! Carry one validated startup snapshot across Store initialization to Settings.
//! No wire authority, recovery actor or second saved-state owner lives here.

use std::sync::Arc;

use vessel_proto::parking::ParkAfter;
use vessel_store::{DataDir, SavedNodeSettings, SettingsDisposition};

use super::set_aside;

/// Startup owns the same opaque document that the eventual consumer receives.
#[derive(Debug)]
pub(crate) struct Opening {
    pub(super) data_dir: DataDir,
    pub(super) saved: SavedNodeSettings,
    pub(super) flag: ParkAfter,
    pub(super) flag_given: bool,
    pub(super) kept: Option<Arc<str>>,
    pub(super) disposition: SettingsDisposition,
}

impl Opening {
    /// Called under the directory lock, before Store startup retention/recovery.
    /// I/O stays in the node's existing blocking startup phase.
    pub(crate) fn read(config: &crate::Config) -> Self {
        Self::read_parts(
            config.data_dir.clone(),
            ParkAfter::from_duration(config.park_after),
            config.park_after_given,
        )
    }

    pub(super) fn read_parts(data_dir: DataDir, flag: ParkAfter, flag_given: bool) -> Self {
        let (saved, kept, disposition) = match data_dir.read_settings_snapshot() {
            Ok(snapshot) => {
                let (saved, disposition) = snapshot.into_parts();
                (saved.unwrap_or_default(), None, disposition)
            }
            Err(error) => {
                let kept = set_aside(&data_dir, &error);
                (
                    SavedNodeSettings::default(),
                    kept,
                    SettingsDisposition::Current,
                )
            }
        };
        Self {
            data_dir,
            saved,
            flag,
            flag_given,
            kept,
            disposition,
        }
    }
}

#[cfg(test)]
mod tests;
