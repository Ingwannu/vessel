//! Optional event fields are masked even when the enclosing variant is supported.

use super::{ProtocolVersion, SessionEvent};

impl SessionEvent {
    /// One origin rule shared with synthetic assigned-threshold tests,
    /// without changing the durable event's own protocol version.
    pub(crate) fn origin_for_protocol(
        &self,
        protocol: ProtocolVersion,
        assigned: Option<ProtocolVersion>,
    ) -> Option<Self> {
        if let Some(projected) = self.composer_commands_for_protocol(protocol) {
            return Some(projected);
        }
        if let Self::QueueChanged { queue } = self
            && let Some(queue) =
                crate::timeline::compat::queue(queue, protocol, crate::timeline::TIMELINE_PARITY)
        {
            return Some(Self::QueueChanged { queue });
        }
        if let Self::Renamed { title, origin } = self {
            let adapted = origin.for_protocol(protocol, assigned);
            return (adapted != *origin).then(|| Self::Renamed {
                title: title.clone(),
                origin: adapted,
            });
        }
        None
    }
}
