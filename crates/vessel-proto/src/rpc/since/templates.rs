//! Metadata field requirements for existing session templates. It combines with the
//! assigned method and other fields through the single optional call accessor.

use crate::automation::ScheduleTarget;
use crate::{Call, ProtocolVersion, SessionSpec};

pub(super) fn on(call: &Call) -> Option<&SessionSpec> {
    if let Call::CreateSession(request) = call {
        return Some(&request.spec);
    }
    if let Call::CreatePrompt(request) = call {
        return request.session.as_deref();
    }
    if let Call::UpdatePrompt(request) = call {
        return request.session.as_deref();
    }
    if let Call::CreateSchedule(request) = call {
        return target(&request.definition.target);
    }
    if let Call::UpdateSchedule(request) = call {
        return target(&request.definition.target);
    }
    None
}

fn target(value: &ScheduleTarget) -> Option<&SessionSpec> {
    match value {
        ScheduleTarget::NewSession { spec, .. } => Some(spec),
        ScheduleTarget::Session { .. } | ScheduleTarget::Unknown => None,
    }
}

pub(super) fn required(
    existing: ProtocolVersion,
    spec: &SessionSpec,
    assigned: Option<ProtocolVersion>,
) -> Option<ProtocolVersion> {
    if spec.labels.is_empty() && spec.listing.is_none() {
        Some(existing)
    } else {
        assigned.map(|version| existing.max(version))
    }
}

#[cfg(test)]
mod tests;
