//! One validated file snapshot before store recovery or runtime startup.
use std::sync::Arc;
use vessel_proto::parking::ParkAfter;
use vessel_store::{DataDir, SavedNodeSettings, SetAside};

#[derive(Debug)]
pub(crate) struct Opening {
    pub(super) data_dir: DataDir,
    pub(super) saved: SavedNodeSettings,
    pub(super) kept: Option<Arc<str>>,
    pub(super) later: bool,
    pub(super) flag: ParkAfter,
    pub(super) flag_given: bool,
}
impl Opening {
    pub(crate) fn restart_enabled(&self) -> bool {
        super::recovery(&self.saved).enabled
    }
    pub(crate) async fn read(
        config: &crate::config::Config,
    ) -> Result<Self, tokio::task::JoinError> {
        let data_dir = config.data_dir.clone();
        let flag = ParkAfter::from_duration(config.park_after);
        let flag_given = config.park_after_given;
        tokio::task::spawn_blocking(move || Self::load(data_dir, flag, flag_given)).await
    }
    fn load(data_dir: DataDir, flag: ParkAfter, flag_given: bool) -> Self {
        let (saved, kept, later) = match data_dir.read_settings_record() {
            Ok(saved) => {
                let saved = saved.unwrap_or_default();
                // An uncertain coverage result makes a private copy, never loses data.
                let later = data_dir
                    .settings_record_holds_more_than(&saved)
                    .unwrap_or(true);
                (saved, None, later)
            }
            Err(error) => {
                let kept = set_aside(&data_dir, &error);
                (SavedNodeSettings::default(), kept, false)
            }
        };
        Self {
            data_dir,
            saved,
            kept,
            later,
            flag,
            flag_given,
        }
    }
}

/// Move an unusable `settings.json` aside, so no change replaces it;
/// answers why changes must not write when even that failed.
fn set_aside(data_dir: &DataDir, unusable: &vessel_store::SettingsFileError) -> Option<Arc<str>> {
    match data_dir.set_aside_settings(chrono::Utc::now(), SetAside::Unreadable) {
        Ok(aside) => {
            tracing::warn!(
                error = %unusable,
                aside = %aside.display(),
                "settings.json could not be used and was moved aside; the node starts from its flags"
            );
            None
        }
        Err(moved) => {
            tracing::error!(
                error = %unusable,
                %moved,
                "settings.json could not be used or moved aside; the node starts from its flags and will not replace it"
            );
            Some(Arc::from(moved.to_string()))
        }
    }
}
