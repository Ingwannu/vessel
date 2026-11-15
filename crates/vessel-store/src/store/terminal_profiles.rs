//! Profile configuration stays ordered and survives restart without another file.

use super::Store;
use crate::{StoreError, writer::Write};
use vessel_proto::terminal_profiles::{ProfileSettings, SetTerminalProfiles};

#[cfg(test)]
mod tests;

impl Store {
    /// Read administrator configuration; absent means reference defaults, not empty.
    /// # Errors
    /// Corrupt settings or storage failures; neither silently resets configuration.
    pub async fn terminal_profiles(&self) -> Result<ProfileSettings, StoreError> {
        self.read(|reader| reader.with(crate::terminal_profiles::read))
            .await
    }

    /// CAS-replace one bounded configuration through the existing committed writer.
    /// # Errors
    /// A stale revision, invalid/oversized configuration or storage failure.
    pub async fn set_terminal_profiles(
        &self,
        settings: SetTerminalProfiles,
    ) -> Result<ProfileSettings, StoreError> {
        self.write(|done| Write::TerminalProfiles { settings, done })
            .await?;
        self.terminal_profiles().await
    }
}
