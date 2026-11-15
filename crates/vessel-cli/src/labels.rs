//! One CLI label assignment parser; domain validation remains in vessel-proto.
use crate::error::CliError;

pub(crate) fn parse(raw: &[String]) -> Result<vessel_proto::SessionLabels, CliError> {
    let mut labels = vessel_proto::SessionLabels::new();
    for text in raw {
        let (key, value) = text
            .split_once('=')
            .ok_or(CliError::Usage("labels must be key=value"))?;
        labels
            .insert(
                key.parse().map_err(CliError::ParityLabel)?,
                value.parse().map_err(CliError::ParityLabel)?,
            )
            .map_err(CliError::ParityLabel)?;
    }
    Ok(labels)
}
