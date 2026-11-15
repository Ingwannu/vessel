//! Private fixture messages carry public process facts, never node credentials.

use serde::{Deserialize, Serialize};
use vessel_proc::ProcessId;
use vessel_proto::NodeId;

pub(super) const FRAME_LIMIT: usize = 1024;
pub(super) const CONTROL_ENV: &str = "VESSEL_LOCAL_FIXTURE_CONTROL";
pub(super) const HOME_ENV: &str = "VESSEL_LOCAL_FIXTURE_HOME";
pub(super) const PROGRAMS_ENV: &str = "VESSEL_LOCAL_FIXTURE_PROGRAMS";

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Notice {
    #[serde(with = "pid")]
    pub(super) process: ProcessId,
    pub(super) node: NodeId,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Command {
    Stop,
}

mod pid {
    use super::ProcessId;
    use serde::{Deserialize as _, Deserializer, Serializer};

    pub(super) fn serialize<S: Serializer>(
        value: &ProcessId,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_u32(value.get())
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<ProcessId, D::Error> {
        u32::deserialize(deserializer)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}
