use vessel_proto::automation::{Cadence, PreviewSchedule, SchedulePreview};

pub(super) fn valid_input(input: &PreviewSchedule, value: &SchedulePreview) -> bool {
    if input.after != Some(value.after)
        || u32::try_from(value.runs.len()) != Ok(input.count.get())
        || value
            .runs
            .iter()
            .try_fold(value.after, |previous, next| {
                (next > &previous).then_some(*next)
            })
            .is_none()
    {
        return false;
    }
    match &input.cadence {
        Cadence::Every {
            seconds: wanted,
            anchor,
        } => match &value.cadence {
            Cadence::Every {
                seconds,
                anchor: Some(resolved),
            } => wanted == seconds && anchor.unwrap_or(value.after) == *resolved,
            Cadence::Every { anchor: None, .. } | Cadence::Cron { .. } | Cadence::Unknown => false,
        },
        Cadence::Cron {
            expr: wanted,
            zone: wanted_zone,
        } => match &value.cadence {
            Cadence::Cron { expr, zone } => wanted == expr && wanted_zone == zone,
            Cadence::Every { .. } | Cadence::Unknown => false,
        },
        Cadence::Unknown => false,
    }
}
