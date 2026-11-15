//! Finite saved-file observations share G4 admission and never settle writes.
//! A minimum interval also bounds a node that cannot reserve a waiting slot
//! and therefore returns the same snapshot immediately (ADR0083).
use super::buffers::FileKey;
use crate::shell::Shell;
use std::time::{Duration, Instant};
use vessel_app_kit::ui::Context;
use vessel_app_link::workspace::FileAsk;
use vessel_app_view::workspace::{
    file_scope::FileScope, file_ticket::FileTicket, problem::Problem,
};
use vessel_proto::workspace::ReadFile;
use vessel_proto::workspace::panel_files::{ObserveFile, ObservedFile};
const MIN_OBSERVATION_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Default)]
pub(super) struct Watcher {
    anchor: Option<ObservedFile>,
    phase: Phase,
    reconcile: bool,
}
#[derive(Default)]
enum Phase {
    #[default]
    Idle,
    Pending {
        scope: FileScope,
        ticket: FileTicket,
        started: Instant,
    },
    Cooling {
        scope: FileScope,
        ticket: FileTicket,
    },
    Unsupported,
    Paused,
}
impl Watcher {
    pub(super) fn paused(&self) -> bool {
        matches!(self.phase, Phase::Paused)
    }
    pub(super) fn restart(&mut self) {
        if matches!(self.phase, Phase::Paused | Phase::Unsupported) {
            self.phase = Phase::Idle;
        }
    }
}
fn poll_delay(elapsed: Duration) -> Duration {
    MIN_OBSERVATION_INTERVAL.saturating_sub(elapsed)
}
impl Shell {
    fn observation_needed(&self, key: &FileKey) -> bool {
        self.areas.workspace.code.file_key().as_ref() == Some(key)
            || self
                .areas
                .workspace
                .code
                .viewer_at(key)
                .is_some_and(|viewer| {
                    viewer.edit.as_ref().is_some_and(|edit| {
                        edit.model.modified()
                            || edit.model.writing().is_some()
                            || edit.writing.is_some()
                    })
                })
    }
    pub(super) fn begin_file_watch(&mut self, key: &FileKey) {
        if !self.observation_needed(key) {
            return;
        }
        let Some(viewer) = self.areas.workspace.code.viewer_at(key) else {
            return;
        };
        if matches!(
            viewer.watch.phase,
            Phase::Pending { .. } | Phase::Cooling { .. } | Phase::Paused
        ) {
            return;
        }
        let scope = viewer.scope.clone();
        if !self.file_panels_available(&scope) {
            if let Some(viewer) = self.areas.workspace.code.viewer_for(key) {
                viewer.watch.phase = Phase::Unsupported;
            }
            return;
        }
        let Some(ticket) = self.next_file_ticket() else {
            return;
        };
        let Some(viewer) = self.areas.workspace.code.viewer_for(key) else {
            return;
        };
        let ask = ObserveFile {
            root: scope.root(),
            path: key.path.clone(),
            after: viewer.watch.anchor.clone(),
        };
        viewer.watch.phase = Phase::Pending {
            scope,
            ticket,
            started: Instant::now(),
        };
        self.send_file_for(key, FileAsk::Observe { ask, ticket });
    }
    pub(super) fn file_observed(
        &mut self,
        scope: &FileScope,
        ask: &ObserveFile,
        ticket: FileTicket,
        result: Result<ObservedFile, Problem>,
        live: bool,
        cx: &mut Context<'_, Self>,
    ) {
        let key = FileKey::new(scope, ask.path.clone());
        let Some(viewer) = self.areas.workspace.code.viewer_for(&key) else {
            return;
        };
        let Phase::Pending {
            scope: original,
            ticket: pending,
            started,
        } = &viewer.watch.phase
        else {
            return;
        };
        if original != scope || *pending != ticket {
            return;
        }
        let elapsed = started.elapsed();
        if !live || viewer.scope != *scope {
            viewer.watch.phase = Phase::Paused;
            return;
        }
        let observed = match result {
            Ok(observed) => observed,
            Err(Problem::Older) => {
                viewer.watch.phase = Phase::Unsupported;
                return;
            }
            Err(_) => {
                viewer.watch.phase = Phase::Paused;
                return;
            }
        };
        viewer.watch.reconcile |= viewer.watch.anchor.as_ref() != Some(&observed);
        viewer.watch.anchor = Some(observed);
        viewer.watch.phase = Phase::Idle;
        self.reconcile_watched_file(&key, cx);
        if !self.observation_needed(&key) {
            return;
        }
        let Some(next) = self.next_file_ticket() else {
            return;
        };
        if let Some(viewer) = self.areas.workspace.code.viewer_for(&key) {
            viewer.watch.phase = Phase::Cooling {
                scope: scope.clone(),
                ticket: next,
            };
        }
        let scope = scope.clone();
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(poll_delay(elapsed)).await;
            let _gone = this.update(cx, |shell, _cx| {
                let Some(viewer) = shell.areas.workspace.code.viewer_for(&key) else { return; };
                if viewer.scope != scope || !matches!(&viewer.watch.phase, Phase::Cooling { scope: original, ticket } if *ticket == next && *original == scope) { return; }
                viewer.watch.phase = Phase::Idle;
                shell.begin_file_watch(&key);
            });
        }).detach();
    }
    /// Disk observations cannot discharge an original uncertain write. A read is
    /// postponed until its existing owner write/read lane is actually settled.
    pub(super) fn reconcile_watched_file(&mut self, key: &FileKey, cx: &mut Context<'_, Self>) {
        let Some(viewer) = self.areas.workspace.code.viewer_for(key) else {
            return;
        };
        if !viewer.watch.reconcile || viewer.reading.is_some() {
            return;
        }
        if let Some(editing) = &mut viewer.edit {
            if editing.reading.is_some()
                || editing.writing.is_some()
                || editing.model.writing().is_some()
            {
                return;
            }
            let step = editing.model.reconcile();
            if matches!(step, vessel_app_view::workspace::editor::Step::Read { .. }) {
                viewer.watch.reconcile = false;
            }
            self.run_step_for(key, step, cx);
        } else {
            viewer.watch.reconcile = false;
            self.read_viewer_for(
                key,
                ReadFile {
                    root: key.root,
                    path: key.path.clone(),
                    offset: 0,
                    limit: None,
                },
            );
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn immediate_or_degraded_snapshot_cannot_create_a_busy_loop() {
        assert_eq!(poll_delay(Duration::ZERO), Duration::from_secs(1));
        assert_eq!(
            poll_delay(Duration::from_millis(250)),
            Duration::from_millis(750)
        );
        assert_eq!(poll_delay(Duration::from_secs(30)), Duration::ZERO);
    }
}

#[cfg(test)]
mod owner_tests {
    use super::super::{editing::WriteTicket, io_tests};
    use super::*;
    use gpui::TestAppContext;
    use vessel_app_view::workspace::editor::{Step, WriteState, Written};
    #[gpui::test]
    fn changed_disk_observation_never_discharges_an_unknown_original_write(
        cx: &mut TestAppContext,
    ) {
        crate::shell::language_tests::init(cx);
        let (shell, cx) = cx.add_window_view(Shell::without_io);
        shell.update(cx, |shell, cx| {
            let root = io_tests::root();
            io_tests::target(shell, root);
            shell.open_file_at("a.txt".parse().unwrap(), None, cx);
            let key = shell.areas.workspace.code.file_key().unwrap();
            let scope = shell
                .areas
                .workspace
                .code
                .viewer_at(&key)
                .unwrap()
                .scope
                .clone();
            let ticket = shell.editing_for(&key).unwrap().reading.unwrap().0;
            let read = super::super::editing::whole_read(root, key.path.clone());
            shell.editor_read_answered("node", ticket, &read, Ok(io_tests::text("baseline")), cx);
            let writing = shell.next_file_ticket().unwrap();
            let edit = shell.editing_for(&key).unwrap();
            let _step = edit.model.edit("submitted");
            let Step::Write { sequence, .. } = edit.model.save() else {
                panic!("original write")
            };
            edit.writing = Some(WriteTicket {
                ticket: writing,
                sequence,
                scope: scope.clone(),
            });
            let _step = edit
                .model
                .saved(sequence, Written::Uncertain("no terminal evidence".into()));
            let _step = edit.model.edit("baseline");
            let ticket = shell.next_file_ticket().unwrap();
            shell
                .areas
                .workspace
                .code
                .viewer_for(&key)
                .unwrap()
                .watch
                .phase = Phase::Pending {
                scope: scope.clone(),
                ticket,
                started: Instant::now(),
            };
            shell.file_observed(
                &scope,
                &ObserveFile {
                    root,
                    path: key.path.clone(),
                    after: None,
                },
                ticket,
                Ok(ObservedFile::Missing),
                true,
                cx,
            );
            let edit = shell.editing_for(&key).unwrap();
            assert_eq!(edit.model.write_state(), Some(WriteState::Uncertain));
            assert_eq!(edit.model.writing(), Some(sequence));
            assert!(
                edit.reading.is_none(),
                "an uncertain write blocks reconcile reads"
            );
            assert!(!edit.model.modified());
            let viewer = shell.areas.workspace.code.viewer_at(&key).unwrap();
            assert!(viewer.watch.reconcile);
            assert_eq!(viewer.watch.anchor, Some(ObservedFile::Missing));
        });
    }
}
