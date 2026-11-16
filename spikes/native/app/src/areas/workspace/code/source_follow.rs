//! Existing root-follow behavior, with original acquisition correlation.
use super::{Place, Shell, Stale, Summary, Target};

impl Shell {
    /// Follow the open session: read where it runs when it is not the one
    /// shown (the panel open only).
    pub(crate) fn code_follow(&mut self) {
        if !self.areas.workspace.code.open {
            return;
        }
        let Some((machine, session)) = self.code_session() else {
            return;
        };
        let context = self
            .fleet
            .machines
            .iter()
            .find(|item| item.name == machine)
            .and_then(vessel_app_view::Machine::connection_context);
        let same = self
            .areas
            .workspace
            .code
            .target
            .as_ref()
            .is_some_and(|target| {
                target.machine == machine
                    && target.session == session
                    && target.admission.as_ref().map(|receipt| &receipt.context) == context.as_ref()
            });
        if same {
            return;
        }
        self.areas.workspace.code.changes.sources.invalidate();
        // Original attempts stay with their retained owner, including failed
        // old-epoch flushes. No successor context is borrowed for that write.
        self.flush_file();
        if !self.park_file() {
            return;
        }
        let panel = &mut self.areas.workspace.code;
        panel.files.actions.invalidate();
        panel.target = Some(Target {
            machine: machine.clone(),
            session,
            place: Place::Reading,
            cwd: None,
            admission: None,
        });
        // A new session is read whole: nothing it shows is stale.
        panel.files.options = self.prefs.explorer;
        panel.stale = Stale::default();
        panel.changes.sources.invalidate();
        panel.changes.summary = Summary::Loading;
        panel.changes.states.clear();
        panel.changes.closed.clear();
        panel.changes.relist();
        panel.files.listings.clear();
        panel.files.modified.clear();
        panel.files.listing_tickets.clear();
        panel.files.open.clear();
        panel.files.viewer = None;
        panel.files.relist();
        if !self.source_root(&machine, session)
            && let Some(target) = &mut self.areas.workspace.code.target
        {
            target.place = Place::Failed("the original connection is unavailable".to_owned());
        }
    }
}
