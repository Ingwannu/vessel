//! One bounded settings row uses the existing `SQLite` transaction authority.

use crate::{StoreError, codec};
use rusqlite::{Connection, OptionalExtension as _, params};
use vessel_proto::terminal_profiles::{
    MAX_PROFILE_SETTINGS_BYTES, ProfileError, ProfileRevision, ProfileSettings, Profiles,
    SetTerminalProfiles,
};

pub(crate) fn read(db: &Connection) -> Result<ProfileSettings, StoreError> {
    let raw: Option<(i64, String)> = db
        .query_row(
            "SELECT revision,profiles FROM terminal_profile_settings WHERE singleton=1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let Some((revision, raw)) = raw else {
        return Ok(ProfileSettings {
            revision: ProfileRevision::INITIAL,
            profiles: Profiles::shipped()?,
        });
    };
    let revision = u64::try_from(revision).map_err(|_negative| ProfileError::CorruptRevision)?;
    if raw.len() > MAX_PROFILE_SETTINGS_BYTES {
        return Err(ProfileError::CorruptRevision.into());
    }
    let profiles = serde_json::from_str(&raw).map_err(StoreError::CorruptTerminalProfiles)?;
    Ok(ProfileSettings {
        revision: ProfileRevision::new(revision),
        profiles,
    })
}

pub(crate) fn apply(db: &Connection, request: &SetTerminalProfiles) -> Result<(), StoreError> {
    let current = read(db)?;
    if current.revision != request.expected {
        return Err(ProfileError::StaleRevision.into());
    }
    if current.profiles == request.profiles {
        return Ok(());
    }
    let revision = current.revision.next()?;
    let raw = codec::encode(&request.profiles)?;
    if raw.len() > MAX_PROFILE_SETTINGS_BYTES {
        return Err(ProfileError::SettingsTooLarge.into());
    }
    let revision =
        i64::try_from(revision.get()).map_err(|_overflow| ProfileError::RevisionExhausted)?;
    let _changed = db.execute("INSERT INTO terminal_profile_settings(singleton,revision,profiles) VALUES (1,?1,?2) ON CONFLICT(singleton) DO UPDATE SET revision=excluded.revision,profiles=excluded.profiles", params![revision, raw])?;
    Ok(())
}
