//! The replacement path is shared by wake, move and conversation fork/rewind.
use super::startup::StartupAttempt;
use crate::{
    StartupRefusal,
    moving::{Launch, MoveError},
    structured::StructuredActor,
};

/// Detached runtimes keep their existing behavior; a node hook must authorize
/// each actual replacement rather than just the original session creation.
pub(crate) enum RestartAdmission {
    Detached,
    Admitted(StartupAttempt),
    Refused(StartupRefusal),
}
impl StructuredActor {
    pub(crate) async fn admit_restart(
        &mut self,
        launch: &Launch,
    ) -> Result<RestartAdmission, MoveError> {
        let Some(hooks) = self.starts.clone() else {
            return Ok(RestartAdmission::Detached);
        };
        let session = self.life.out.session();
        let folder = launch.setup.cwd.clone();
        let admitted = self
            .pump(async move { hooks.admit(session, folder).await })
            .await
            .map_err(|_stopped| MoveError::Stopped)?;
        let permit = match admitted {
            Ok(permit) => permit,
            Err(reason) => return Ok(RestartAdmission::Refused(reason)),
        };
        if self.life.stop.killed() {
            return Err(MoveError::Stopped);
        }
        Ok(match permit.begin() {
            Ok(attempt) => RestartAdmission::Admitted(attempt),
            Err(reason) => RestartAdmission::Refused(reason),
        })
    }
}
