//! Profile metadata and administrator configuration use separate scoped calls.

use super::{Connection, NodeService, wire};
use vessel_proto::terminal_profiles::{
    GetTerminalProfiles, ListTerminalProfiles, ProfileSettings, ProfileSummary, SetTerminalProfiles,
};
use vessel_proto::{ErrorCode, WireError};

impl NodeService {
    pub(crate) async fn list_terminal_profiles(
        &self,
        connection: &Connection,
        _: ListTerminalProfiles,
    ) -> Result<Vec<ProfileSummary>, WireError> {
        person(connection)?;
        self.sessions
            .store()
            .terminal_profiles()
            .await
            .map(|settings| settings.summaries())
            .map_err(|error| wire(&error.into()))
    }
    pub(crate) async fn get_terminal_profiles(
        &self,
        connection: &Connection,
        _: GetTerminalProfiles,
    ) -> Result<ProfileSettings, WireError> {
        person(connection)?;
        self.sessions
            .store()
            .terminal_profiles()
            .await
            .map_err(|error| wire(&error.into()))
    }
    pub(crate) async fn set_terminal_profiles(
        &self,
        connection: &Connection,
        request: SetTerminalProfiles,
    ) -> Result<ProfileSettings, WireError> {
        person(connection)?;
        self.sessions
            .store()
            .set_terminal_profiles(request)
            .await
            .map_err(|error| wire(&error.into()))
    }
}

fn person(connection: &Connection) -> Result<(), WireError> {
    if connection.principal().agent().is_some() {
        return Err(WireError::new(
            ErrorCode::Forbidden,
            "terminal launch profiles are a person's configuration",
        ));
    }
    Ok(())
}
