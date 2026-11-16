//! Private fixture DTOs shared by the node fixture binary and local-node unit tests.
//! One source avoids schema copies; only serde and existing process/node IDs live
//! here. No node runtime, I/O, parent-module path or process authority is exposed.

use serde::{Deserialize, Serialize};
use vessel_proc::ProcessId;
use vessel_proto::NodeId;

pub(super) const FRAME_LIMIT: std::num::NonZeroUsize = match std::num::NonZeroUsize::new(1024) {
    Some(limit) => limit,
    None => std::num::NonZeroUsize::MIN,
};
pub(super) const CONTROL_ENV: &str = "VESSEL_LOCAL_FIXTURE_CONTROL";
pub(super) const HOME_ENV: &str = "VESSEL_LOCAL_FIXTURE_HOME";
pub(super) const PROGRAMS_ENV: &str = "VESSEL_LOCAL_FIXTURE_PROGRAMS";

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Notice {
    Ready {
        #[serde(with = "pid")]
        process: ProcessId,
        node: NodeId,
    },
    Stopped {
        #[serde(with = "pid")]
        process: ProcessId,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Command {
    Stop,
    Exit,
}

mod pid {
    use serde::{Deserialize as _, Deserializer, Serializer};
    use vessel_proc::ProcessId;

    pub(super) fn serialize<S: Serializer>(
        value: impl std::borrow::Borrow<ProcessId>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_u32(value.borrow().get())
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<ProcessId, D::Error> {
        u32::deserialize(deserializer)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}
