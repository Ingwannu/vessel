//! A definition's time checked before it is kept (the contract's
//! `schedule.create`): its zone exists, its cron has a due time, its end
//! is after its first due time, and what follows a new session's run is
//! one this node knows. And what fills it: an `every` without an anchor
//! gets the moment it was set, so its phase survives restarts; an agent's
//! is always anchored so.

use chrono::{DateTime, Utc};
use vessel_proto::automation::{AfterRun, Cadence, CatchUp, ScheduleDefinition, ScheduleTarget};

use super::rules::{Asker, Via};
use crate::automation::error::AutomationError;
use crate::automation::time::{Due, NotComputable, Zones};

/// An agent's `every` is anchored by the node, at the moment it writes it.
///
/// An anchor given by the caller places the first due time anywhere in
/// the next period: an agent writing `every 60 s` anchored 59.9 s ago,
/// again and again, would run its schedule at once each time, past the
/// 60 s floor and the rule that an agent never runs one now (the join's
/// code review, finding 6). A person may run a schedule now anyway, so
/// keeps the anchor given (to line a schedule up with the hour, say).
pub(super) fn node_anchor(asker: &Asker, definition: &mut ScheduleDefinition) {
    if asker.via == Via::Agent
        && let Cadence::Every { anchor, .. } = &mut definition.cadence
    {
        *anchor = None;
    }
}

/// `definition` made ready to keep at `now`: an `every`'s anchor filled,
/// and its first due time after `now`.
pub(super) fn settle(
    zones: &Zones,
    definition: &mut ScheduleDefinition,
    now: DateTime<Utc>,
) -> Result<DateTime<Utc>, AutomationError> {
    settle_for(zones, definition, now, WindowRequirement::FutureRun)
}

/// An already completed definition never runs again merely because it is edited.
/// Preserve a past expiry while retaining all cadence/zone/target-policy checks.
/// Pinned Paseo schedule/service.ts458–509 keeps completed status on update.
pub(super) fn settle_completed(
    zones: &Zones,
    definition: &mut ScheduleDefinition,
    now: DateTime<Utc>,
) -> Result<DateTime<Utc>, AutomationError> {
    settle_for(zones, definition, now, WindowRequirement::CompletedEdit)
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum WindowRequirement {
    FutureRun,
    CompletedEdit,
}
fn settle_for(
    zones: &Zones,
    definition: &mut ScheduleDefinition,
    now: DateTime<Utc>,
    window: WindowRequirement,
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
    // Neither archived nor waited for, each run's session would stay
    // live beside the next (the join's code review, finding 4).
    if matches!(
        definition.target,
        ScheduleTarget::NewSession {
            after: AfterRun::Unknown,
            ..
        }
    ) {
        return Err(AutomationError::Invalid(
            "what happens to a run's session after its turn is one this node does not know"
                .to_owned(),
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
    if window == WindowRequirement::FutureRun
        && let Some(ends_at) = definition.ends_at
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
    fn an_after_this_node_does_not_know_is_refused() {
        let zones = Zones::default();
        let mut unknown = definition(Cadence::Every {
            seconds: EverySeconds::try_from(600).unwrap(),
            anchor: None,
        });
        unknown.target = ScheduleTarget::NewSession {
            spec: Box::new(vessel_proto::SessionSpec::new(
                vessel_proto::HarnessKind::ClaudeCode,
                vessel_proto::SessionMode::Structured {
                    model: None,
                    resume: None,
                    options: Vec::new(),
                },
            )),
            after: AfterRun::Unknown,
        };
        assert!(matches!(
            settle(&zones, &mut unknown, at("2026-01-01T10:00:00Z")),
            Err(AutomationError::Invalid(_))
        ));
    }

    #[test]
    fn an_agents_anchor_is_the_nodes_and_a_persons_is_kept() {
        let now = at("2026-01-01T10:00:00Z");
        let almost = Cadence::Every {
            seconds: EverySeconds::try_from(60).unwrap(),
            anchor: Some(at("2026-01-01T09:59:01Z")),
        };
        let agent = Asker {
            by: vessel_proto::Creator::Agent {
                node: vessel_proto::NodeId::random(),
                session: SessionId::random(),
            },
            via: Via::Agent,
        };
        let mut agents = definition(almost.clone());
        node_anchor(&agent, &mut agents);
        let first = settle(&Zones::default(), &mut agents, now).unwrap();
        assert_eq!(first, at("2026-01-01T10:01:00Z"), "a whole period away");
        let person = Asker {
            by: vessel_proto::Creator::Owner,
            via: Via::Person,
        };
        let mut persons = definition(almost);
        node_anchor(&person, &mut persons);
        let first = settle(&Zones::default(), &mut persons, now).unwrap();
        assert_eq!(first, at("2026-01-01T10:00:01Z"), "the phase asked for");
    }

    #[test]
    fn completed_expiry_only_relaxes_the_future_window_not_definition_validation() {
        let zones = Zones::default();
        let now = at("2026-01-01T10:00:00Z");
        let expiry = at("2026-01-01T09:30:00Z");
        let mut completed = definition(Cadence::Every {
            seconds: EverySeconds::try_from(600).unwrap(),
            anchor: None,
        });
        completed.ends_at = Some(expiry);
        assert!(settle(&zones, &mut completed, now).is_err());
        assert_eq!(
            settle_completed(&zones, &mut completed, now).unwrap(),
            at("2026-01-01T10:10:00Z")
        );
        assert_eq!(completed.ends_at, Some(expiry));
        let mut unknown_policy = completed.clone();
        unknown_policy.catch_up = CatchUp::Unknown;
        let mut unknown_target = completed.clone();
        unknown_target.target = ScheduleTarget::Unknown;
        let mut unknown_zone = completed;
        unknown_zone.cadence = Cadence::Cron {
            expr: "0 9 * * *".parse().unwrap(),
            zone: "Mars/Olympus".parse().unwrap(),
        };
        for mut invalid in [unknown_policy, unknown_target, unknown_zone] {
            assert!(matches!(
                settle_completed(&zones, &mut invalid, now),
                Err(AutomationError::Invalid(_))
            ));
        }
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
