//! Typed launch-profile CLI adapter; no account-file or preference discovery.
//!
//! File input reuses Tokio's bounded asynchronous file reader and the protocol's
//! whole-body budget. Requiring an explicit file avoids an uninterruptible stdin
//! worker. Replacement takes the revision the operator read, never a hidden
//! read/overwrite loop; lost acknowledgements are not retried.

use crate::{
    args::{LaunchProfilesCommand, Target},
    connect::connect,
    error::CliError,
    io,
};
use serde::{Serialize, de::DeserializeOwned};
use std::path::Path;
use tokio::io::AsyncReadExt as _;
use vessel_proto::launch_profiles::{
    ApplySessionOptions, GetLaunchProfileOptions, LaunchPayload, ReplaceLaunchProfiles,
};

#[derive(Debug, thiserror::Error)]
pub(crate) enum LaunchCliError {
    #[error("cannot read the launch request")]
    Read(#[source] std::io::Error),
    #[error("the launch request exceeds the protocol byte limit")]
    TooLarge,
    #[error("invalid typed launch request")]
    Decode(#[source] serde_json::Error),
    #[error("cannot encode the launch reply")]
    Encode(#[source] serde_json::Error),
    #[error(transparent)]
    Apply(#[from] vessel_client::launch_profiles::LaunchProfileClientError),
    #[error("configuration stopped at the reported failure; earlier choices remain applied")]
    Partial,
}

pub(crate) async fn command(
    target: &Target,
    command: LaunchProfilesCommand,
) -> Result<(), CliError> {
    match command {
        LaunchProfilesCommand::List => {
            let client = connect(target).await?.client;
            output(&client.launch_profiles().await?).await
        }
        LaunchProfilesCommand::Replace { input } => {
            let request: ReplaceLaunchProfiles = read(&input).await?;
            let client = connect(target).await?.client;
            output(
                &client
                    .replace_launch_profiles(request.expected, request.profiles)
                    .await?,
            )
            .await
        }
        LaunchProfilesCommand::Options { input } => {
            let request: GetLaunchProfileOptions = read(&input).await?;
            let client = connect(target).await?.client;
            output(&client.launch_profile_options(request.selection).await?).await
        }
        LaunchProfilesCommand::Applicable { session } => {
            let client = connect(target).await?.client;
            output(&client.applicable_launch_profiles(session).await?).await
        }
        LaunchProfilesCommand::Apply { input } => {
            let request: ApplySessionOptions = read(&input).await?;
            let client = connect(target).await?.client;
            let result = client
                .apply_session_options(request.session, request.options)
                .await
                .map_err(LaunchCliError::Apply)?;
            output(&result).await?;
            if result.failure().is_some() {
                return Err(LaunchCliError::Partial.into());
            }
            Ok(())
        }
    }
}

async fn read<T: DeserializeOwned>(path: &Path) -> Result<T, LaunchCliError> {
    let file = tokio::fs::File::open(path)
        .await
        .map_err(LaunchCliError::Read)?;
    // Saturation still refuses the body; it can never admit an oversized input.
    let limit = u64::try_from(LaunchPayload::MAX_BYTES)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    let mut bytes = Vec::new();
    let _read = file
        .take(limit)
        .read_to_end(&mut bytes)
        .await
        .map_err(LaunchCliError::Read)?;
    decode(&bytes)
}

fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, LaunchCliError> {
    if bytes.len() > LaunchPayload::MAX_BYTES {
        return Err(LaunchCliError::TooLarge);
    }
    serde_json::from_slice(bytes).map_err(LaunchCliError::Decode)
}

async fn output(value: &impl Serialize) -> Result<(), CliError> {
    let mut bytes = serde_json::to_vec(value).map_err(LaunchCliError::Encode)?;
    bytes.push(b'\n');
    Ok(io::out(&bytes).await?)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replacement_keeps_exact_revision_and_whole_list_and_rejects_extra_input() {
        let request: ReplaceLaunchProfiles = decode(br#"{"expected":7,"profiles":[]}"#).unwrap();
        assert_eq!(
            serde_json::to_string(&request).unwrap(),
            r#"{"expected":7,"profiles":[]}"#
        );
        assert!(
            decode::<ReplaceLaunchProfiles>(br#"{"expected":7,"profiles":[],"merge":true}"#)
                .is_err()
        );
        assert!(matches!(
            decode::<ReplaceLaunchProfiles>(&vec![
                b' ';
                LaunchPayload::MAX_BYTES.saturating_add(1)
            ]),
            Err(LaunchCliError::TooLarge)
        ));
    }
}
