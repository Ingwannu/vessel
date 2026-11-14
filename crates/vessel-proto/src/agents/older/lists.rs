//! Lists without the agents a peer cannot read: each function returns
//! `None` when nothing changes, so a current peer costs no copy.

use super::{newer_prompt, newer_schedule, reads};
use crate::agents::{AgentInfo, AgentReadiness, Catalogue, CatalogueRun};
use crate::automation::{PromptList, ScheduleList};
use crate::harness::Harness;
use crate::node::ProtocolVersion;
use crate::profile::ProfileInfo;
use crate::usage::UsageReport;

/// `items` without the ones `shown` refuses, or `None` when it refuses
/// none.
fn kept<T: Clone>(items: &[T], shown: impl Fn(&T) -> bool) -> Option<Vec<T>> {
    (!items.iter().all(&shown)).then(|| items.iter().filter(|item| shown(item)).cloned().collect())
}

/// `harness.list`'s harnesses.
pub(crate) fn harnesses(harnesses: &[Harness], protocol: ProtocolVersion) -> Option<Vec<Harness>> {
    kept(harnesses, |harness| reads(&harness.kind, protocol))
}

/// `profile.list`'s and `profile.reload`'s profiles.
pub(crate) fn profiles(
    profiles: &[ProfileInfo],
    protocol: ProtocolVersion,
) -> Option<Vec<ProfileInfo>> {
    kept(profiles, |profile| reads(&profile.harness, protocol))
}

/// `agent.list`'s agents.
pub(crate) fn agents(agents: &[AgentInfo], protocol: ProtocolVersion) -> Option<Vec<AgentInfo>> {
    kept(agents, |agent| reads(&agent.agent, protocol))
}

/// `agent.status`'s answers.
pub(crate) fn readiness(
    readiness: &[AgentReadiness],
    protocol: ProtocolVersion,
) -> Option<Vec<AgentReadiness>> {
    kept(readiness, |answer| reads(&answer.agent, protocol))
}

/// `agent.catalogue`'s entries: an entry that runs a known agent the peer
/// does not know is left out and counted as skipped.
pub(crate) fn catalogue(catalogue: &Catalogue, protocol: ProtocolVersion) -> Option<Catalogue> {
    let shown = |entry: &crate::agents::CatalogueEntry| match &entry.runs {
        CatalogueRun::Known { harness } => harness.since() <= protocol,
        CatalogueRun::Command { .. }
        | CatalogueRun::Download { .. }
        | CatalogueRun::Unavailable => true,
    };
    kept(&catalogue.entries, shown).map(|entries| {
        let left_out = catalogue.entries.len().saturating_sub(entries.len());
        Catalogue {
            fetched_at: catalogue.fetched_at,
            version: catalogue.version.clone(),
            skipped: catalogue
                .skipped
                .saturating_add(u32::try_from(left_out).unwrap_or(u32::MAX)),
            entries,
        }
    })
}

/// `schedule.list`'s schedules.
pub(crate) fn schedules(list: &ScheduleList, protocol: ProtocolVersion) -> Option<ScheduleList> {
    kept(&list.schedules, |schedule| {
        newer_schedule(schedule, protocol).is_none()
    })
    .map(|schedules| ScheduleList { schedules })
}

/// `prompt.list`'s saved prompts.
pub(crate) fn prompts(list: &PromptList, protocol: ProtocolVersion) -> Option<PromptList> {
    kept(&list.prompts, |prompt| {
        newer_prompt(prompt, protocol).is_none()
    })
    .map(|prompts| PromptList { prompts })
}

/// `usage.query`'s agent groups (protocol 21). The report's totals still
/// count every agent: they are the machine's, as the peer asked, and only
/// the groups it cannot decode are left out.
pub(crate) fn usage(report: &UsageReport, protocol: ProtocolVersion) -> Option<UsageReport> {
    kept(&report.agents, |group| reads(&group.harness, protocol)).map(|agents| UsageReport {
        agents,
        ..report.clone()
    })
}
