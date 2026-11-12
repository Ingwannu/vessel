//! A definition's time checked before it is kept (the contract's
//! `schedule.create`): its zone exists, its cron has a due time, its end
//! is after its first due time. And what fills it: an `every` without an
//! anchor gets the moment it was set, so its phase survives restarts.

use chrono::{DateTime, Utc};
use vessel_proto::automation::{Cadence, CatchUp, ScheduleDefinition, ScheduleTarget};

use crate::automation::error::AutomationError;
use crate::automation::time::{Due, NotComputable, Zones};

/// `definition` made ready to keep at `now`: an `every`'s anchor filled,
/// and its first due time after `now`.
pub(super) fn settle(
    zones: &Zones,
    definition: &mut ScheduleDefinition,
    now: DateTime<Utc>,
) -> Result<DateTime<Utc>, AutomationError> {
    if let Cadence::Every { anchor, .. } = &mut definition.cadence
        && anchor.is_none()
    {
        *anchor = Some(now);
    }
    if matches!(definition.catch_up, CatchUp::Unknown) {
        return Err(AutomationError::Invalid(
            "a catch-up this node does not know".to_owned(),
        ));
    }
    if matches!(definition.target, ScheduleTarget::Unknown) {
        return Err(AutomationError::Invalid(
            "a target this node does not know".to_owned(),
        ));
    }
    let due = Due::of(&definition.cadence, now, zones).map_err(|error| match error {
        NotComputable::Zone(unknown) => AutomationError::Invalid(unknown.to_string()),
        NotComputable::Unknown => {
            AutomationError::Invalid("a cadence this node does not know".to_owned())
        }
    })?;
    let first = due.next_after(now).ok_or_else(|| {
        AutomationError::Invalid(
            "this cron expression matches no day in the next eight years, so it would never run"
                .to_owned(),
        )
    })?;
    if let Some(ends_at) = definition.ends_at
        && ends_at < first
    {
        return Err(AutomationError::Invalid(format!(
            "it ends ({ends_at}) before its first due time ({first}), so it would never run"
        )));
    }
    Ok(first)
}

#[cfg(test)]
mod tests {
    use vessel_proto::SessionId;
    use vessel_proto::automation::EverySeconds;

    use super::*;

    fn definition(cadence: Cadence) -> ScheduleDefinition {
        ScheduleDefinition {
            name: None,
            cadence,
            target: ScheduleTarget::Session {
                session: SessionId::random(),
            },
            prompt: "Look.".parse().unwrap(),
            catch_up: CatchUp::Once,
            max_runs: None,
            ends_at: None,
        }
    }

    fn at(text: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(text)
            .unwrap()
            .with_timezone(&Utc)
    }

    #[test]
    fn an_every_gets_its_anchor_and_its_first_due_time() {
        let zones = Zones::default();
        let now = at("2026-01-01T10:00:00Z");
        let mut every = definition(Cadence::Every {
            seconds: EverySeconds::try_from(600).unwrap(),
            anchor: None,
        });
        let first = settle(&zones, &mut every, now).unwrap();
        assert_eq!(first, at("2026-01-01T10:10:00Z"));
        assert_eq!(
            every.cadence,
            Cadence::Every {
                seconds: EverySeconds::try_from(600).unwrap(),
                anchor: Some(now),
            }
        );
    }

    #[test]
    fn what_would_never_run_is_refused() {
        let zones = Zones::default();
        let now = at("2026-01-01T10:00:00Z");
        let never = Cadence::Cron {
            expr: "0 0 30 2 *".parse().unwrap(),
            zone: "UTC".parse().unwrap(),
        };
        assert!(matches!(
            settle(&zones, &mut definition(never), now),
            Err(AutomationError::Invalid(_))
        ));
        let mars = Cadence::Cron {
            expr: "0 9 * * *".parse().unwrap(),
            zone: "Mars/Olympus".parse().unwrap(),
        };
        let refused = settle(&zones, &mut definition(mars), now).unwrap_err();
        assert!(
            refused.to_string().contains("unknown time zone"),
            "{refused}"
        );
        let mut ends_early = definition(Cadence::Cron {
            expr: "0 9 * * *".parse().unwrap(),
            zone: "UTC".parse().unwrap(),
        });
        ends_early.ends_at = Some(at("2026-01-01T11:00:00Z"));
        assert!(settle(&zones, &mut ends_early, now).is_err());
        ends_early.ends_at = Some(at("2026-01-02T09:00:00Z"));
        assert_eq!(
            settle(&zones, &mut ends_early, now).unwrap(),
            at("2026-01-02T09:00:00Z")
        );
    }
}
