//! The label field rule for existing session templates. It combines with the
//! assigned method and other fields through the single optional call accessor.

use crate::automation::ScheduleTarget;
use crate::{Call, ProtocolVersion, SessionLabels};

pub(super) fn on(call: &Call) -> Option<&SessionLabels> {
    if let Call::CreateSession(request) = call {
        return Some(&request.spec.labels);
    }
    if let Call::CreatePrompt(request) = call {
        return request.session.as_ref().map(|spec| &spec.labels);
    }
    if let Call::UpdatePrompt(request) = call {
        return request.session.as_ref().map(|spec| &spec.labels);
    }
    if let Call::CreateSchedule(request) = call {
        return target(&request.definition.target);
    }
    if let Call::UpdateSchedule(request) = call {
        return target(&request.definition.target);
    }
    None
}

fn target(value: &ScheduleTarget) -> Option<&SessionLabels> {
    match value {
        ScheduleTarget::NewSession { spec, .. } => Some(&spec.labels),
        ScheduleTarget::Session { .. } | ScheduleTarget::Unknown => None,
    }
}

pub(super) fn required(
    existing: ProtocolVersion,
    labels: &SessionLabels,
    assigned: Option<ProtocolVersion>,
) -> Option<ProtocolVersion> {
    if labels.is_empty() {
        Some(existing)
    } else {
        assigned.map(|version| existing.max(version))
    }
}

#[cfg(test)]
mod tests;
