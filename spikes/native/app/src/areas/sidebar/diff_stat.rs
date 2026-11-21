//! Each connected machine's diff stats (`workspace.list`, ADR 0120), for a
//! sidebar row, its hover card and the composer's count. One list is
//! outstanding per machine. The first read omits `changed_from`; later reads send the
//! generation they hold. An empty answer keeps that generation. A machine
//! below protocol 38 is not asked. `Problem::Older` stops that machine;
//! any other failure waits and tries again.

use std::collections::HashMap;
use std::time::Duration;

use vessel_app_kit::ui::Context;
use vessel_app_link::workspace::Ask;
use vessel_app_view::Reach;
use vessel_app_view::workspace::pace::QUIET;
use vessel_app_view::workspace::problem::Problem;
use vessel_proto::WorkspaceId;
use vessel_proto::registry::{DiffStat, ListWorkspaces, ServedWorkspace, serves_diff_stat};

use crate::shell::Shell;

/// How long to wait after a list that failed for a reason other than age.
const RETRY: Duration = Duration::from_secs(5);

/// The stats the sidebar holds, one host at a time.
#[derive(Debug, Default)]
pub(super) struct DiffStats {
    hosts: HashMap<String, Host>,
}

#[derive(Debug, Default)]
struct Host {
    phase: Phase,
    rest: u64,
    revision: Option<vessel_proto::registry::DiffStatRevision>,
    stats: HashMap<WorkspaceId, DiffStat>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Phase {
    #[default]
    Idle,
    Out,
    Resting,
    Stopped,
}

enum Step {
    Stop,
    Rest {
        rest: u64,
        wait: Duration,
        changed: bool,
    },
}

impl Shell {
    /// Ask each connected machine that serves a diff stat, unless a list
    /// for it is already out, resting, or stopped.
    pub(crate) fn diff_stat_follow(&mut self) {
        let wanted: Vec<String> = self
            .fleet
            .machines
            .iter()
            .filter(|machine| {
                machine.reach == Reach::Online && machine.protocol.is_some_and(serves_diff_stat)
            })
            .map(|machine| machine.name.clone())
            .collect();
        self.areas
            .sidebar
            .diff
            .hosts
            .retain(|name, _| wanted.iter().any(|wanted| wanted == name));
        let idle: Vec<String> = wanted
            .iter()
            .filter(|name| {
                self.areas
                    .sidebar
                    .diff
                    .hosts
                    .get(name.as_str())
                    .is_none_or(|host| host.phase == Phase::Idle)
            })
            .cloned()
            .collect();
        for name in &idle {
            self.diff_stat_send(name);
        }
    }

    /// `workspace`'s stat on `machine`, when the last list served one.
    #[must_use]
    pub(crate) fn diff_stat_of(&self, machine: &str, workspace: WorkspaceId) -> Option<DiffStat> {
        self.areas
            .sidebar
            .diff
            .hosts
            .get(machine)?
            .stats
            .get(&workspace)
            .copied()
    }

    /// A list came back. `false` when nothing drawn changed, so the shell
    /// asks for no frame. The next read is still scheduled.
    #[must_use]
    pub(crate) fn diff_stat_answered(
        &mut self,
        machine: &str,
        result: Result<Vec<ServedWorkspace>, Problem>,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        let step = {
            let Some(host) = self.areas.sidebar.diff.hosts.get_mut(machine) else {
                return false;
            };
            if host.phase != Phase::Out {
                return false;
            }
            match result {
                Err(Problem::Older) => {
                    host.phase = Phase::Stopped;
                    Step::Stop
                }
                Err(_) => {
                    host.rest = host.rest.wrapping_add(1);
                    let rest = host.rest;
                    host.phase = Phase::Resting;
                    Step::Rest {
                        rest,
                        wait: RETRY,
                        changed: false,
                    }
                }
                Ok(rows) => {
                    let next = served(&rows);
                    let changed = host.stats != next;
                    host.stats = next;
                    host.revision = rows
                        .iter()
                        .find_map(|row| row.stat_revision)
                        .or(host.revision);
                    host.rest = host.rest.wrapping_add(1);
                    let rest = host.rest;
                    host.phase = Phase::Resting;
                    Step::Rest {
                        rest,
                        wait: QUIET,
                        changed,
                    }
                }
            }
        };
        match step {
            Step::Stop => false,
            Step::Rest {
                rest,
                wait,
                changed,
            } => {
                Self::diff_stat_rest(machine, rest, wait, cx);
                changed
            }
        }
    }

    fn diff_stat_send(&mut self, machine: &str) {
        let changed_from = {
            let host = self
                .areas
                .sidebar
                .diff
                .hosts
                .entry(machine.to_owned())
                .or_default();
            if host.phase != Phase::Idle {
                return;
            }
            host.phase = Phase::Out;
            host.revision
        };
        self.ask(
            machine,
            Ask::Workspaces(ListWorkspaces {
                changed_from,
                ..ListWorkspaces::default()
            }),
        );
    }

    fn diff_stat_rest(machine: &str, rest: u64, wait: Duration, cx: &mut Context<'_, Self>) {
        let machine = machine.to_owned();
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(wait).await;
            let _gone = this.update(cx, |shell, _cx| {
                let ready = {
                    let Some(host) = shell.areas.sidebar.diff.hosts.get_mut(&machine) else {
                        return;
                    };
                    if host.phase == Phase::Resting && host.rest == rest {
                        host.phase = Phase::Idle;
                        true
                    } else {
                        false
                    }
                };
                if ready {
                    shell.diff_stat_send(&machine);
                }
            });
        })
        .detach();
    }
}

/// The stats a list served. A row with no stat draws nothing.
fn served(rows: &[ServedWorkspace]) -> HashMap<WorkspaceId, DiffStat> {
    rows.iter()
        .filter_map(|row| row.diff_stat.map(|stat| (row.workspace.id, stat)))
        .collect()
}
