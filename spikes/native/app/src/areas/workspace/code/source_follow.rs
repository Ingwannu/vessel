//! Existing root-follow behavior, with original acquisition correlation.
use super::*;
use vessel_app_view::workspace::root::session_root;
use vessel_proto::SessionInfo;
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
                    && target.receipt.as_ref().map(|receipt| &receipt.context) == context.as_ref()
            });
        if same {
            return;
        }
        // The file edited in the session left is written first (Paseo
        // flushes an editor when its panel goes).
        self.flush_file();
        let panel = &mut self.areas.workspace.code;
        panel.target = Some(Target {
            machine: machine.clone(),
            session,
            place: Place::Reading,
            cwd: None,
            receipt: None,
        });
        // A new session is read whole: nothing it shows is stale.
        panel.stale = Stale::default();
        panel.changes.sources.invalidate();
        panel.changes.summary = Summary::Loading;
        panel.changes.states.clear();
        panel.changes.closed.clear();
        panel.changes.relist();
        panel.files.listings.clear();
        panel.files.open.clear();
        panel.files.viewer = None;
        panel.files.relist();
        if !self.source_root(&machine, session) {
            self.ask(&machine, Ask::Session(session));
        }
    }

    /// Where a session runs arrived.
    pub(super) fn code_session_read(
        &mut self,
        session: SessionId,
        result: Result<Box<SessionInfo>, Problem>,
    ) {
        self.finder_session_read(session, &result);
        if self
            .areas
            .workspace
            .code
            .target
            .as_ref()
            .is_some_and(|target| target.receipt.is_some())
        {
            return;
        }
        let words = self.words();
        let panel = &mut self.areas.workspace.code;
        let Some(target) = panel
            .target
            .as_mut()
            .filter(|target| target.session == session)
        else {
            return;
        };
        target.place = match &result {
            Ok(info) => session_root(info).map_or(Place::Outside, Place::Root),
            Err(problem) => Place::Failed(words.problem(problem)),
        };
        target.cwd = result.ok().and_then(|info| info.spec.cwd);
        self.code_read_tab();
    }
}
