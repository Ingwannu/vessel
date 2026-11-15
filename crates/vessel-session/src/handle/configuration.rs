//! A live bundle captures one process input; it never follows a moving route.

use super::SessionHandle;
use crate::SessionError;
use vessel_proto::launch_profiles::LaunchChoices;

impl SessionHandle {
    /// Apply one ordered bundle through the process's shared mutation owner.
    /// A parked session wakes first; the resulting input is captured once,
    /// so a concurrent move cannot send later steps to another account.
    ///
    /// # Errors
    /// Existing wake, terminal, ended and moving refusals, or a process
    /// closed before admission. Later refusal retains its completed prefix.
    pub async fn apply_options(
        &self,
        choices: LaunchChoices,
    ) -> Result<vessel_agent::ConfigurationOutcome, SessionError> {
        if self.parked() {
            let _woken = self.wake().await?;
        }
        let process = self.agent()?;
        Ok(process.apply_options(choices).await?)
    }
}
