//! Root receipts and owner callbacks keep the original shared admission.
use super::{Place, Stale, Target};
use crate::shell::Shell;
use vessel_app_kit::ui::Context;
use vessel_app_link::workspace::scoped::{Outcome, Reply, Request};
use vessel_app_view::connection::ConnectionContext;
use vessel_app_view::workspace::{
    file_scope::FileScope, file_ticket::FileTicket, problem::Problem, root::session_root,
};
use vessel_proto::{SessionId, SessionInfo};

#[derive(Clone)]
pub(super) struct Acquisition {
    pub(super) context: ConnectionContext,
    pub(super) ticket: FileTicket,
}
impl Shell {
    pub(super) fn file_connection(&self, machine: &str) -> Option<ConnectionContext> {
        self.fleet
            .machines
            .iter()
            .find(|candidate| candidate.name == machine)?
            .connection_context()
    }
    pub(crate) fn file_operation_answered(&mut self, reply: Reply, cx: &mut Context<'_, Self>) {
        let (context, outcome) = reply.into_parts();
        let current = self.file_connection(context.machine().as_str()).as_ref() == Some(&context);
        match outcome {
            Outcome::Root {
                session,
                ticket,
                result,
            } => {
                self.finder_scoped_root(&context, session, ticket, &result, current);
                let owns = self
                    .areas
                    .workspace
                    .code
                    .target
                    .as_ref()
                    .is_some_and(|target| {
                        target.session == session
                            && target.admission.as_ref().is_some_and(|read| {
                                read.context == context && read.ticket == ticket
                            })
                    });
                if !owns {
                    return;
                }
                let result = if current {
                    result
                } else {
                    Err(Problem::Gone("the original connection retired".to_owned()))
                };
                self.install_code_session_read(session, result);
            }
            Outcome::File(reply) => {
                let (scope, answer, dispatched) = (*reply).into_parts();
                self.file_scoped_answer(&scope, answer, current, dispatched, cx);
            }
        }
    }
    /// Legacy session.get answers do not supply a checked connection receipt.
    pub(super) fn code_session_read(
        &mut self,
        session: SessionId,
        result: Result<Box<SessionInfo>, Problem>,
    ) {
        if self
            .areas
            .workspace
            .code
            .target
            .as_ref()
            .is_some_and(|target| target.admission.is_some())
        {
            return;
        }
        self.install_code_session_read(session, result);
    }
    /// Snapshot only; every effect must still use the shared actor preflight.
    pub(crate) fn file_admitted_scope(&self) -> Option<FileScope> {
        let panel = &self.areas.workspace.code;
        let target = panel.target.as_ref()?;
        let context = &target.admission.as_ref()?.context;
        if self.file_connection(context.machine().as_str()).as_ref() != Some(context) {
            return None;
        }
        let (_, root) = panel.root()?;
        Some(FileScope::new(context.clone(), *root))
    }
    /// Follow the open session: read where it runs when it is not the one
    /// shown (the panel open only).
    pub(crate) fn code_follow(&mut self) {
        if !self.areas.workspace.code.open {
            return;
        }
        let Some((machine, session)) = self.code_session() else {
            return;
        };
        let context = self.file_connection(&machine);
        let same = self
            .areas
            .workspace
            .code
            .target
            .as_ref()
            .is_some_and(|target| {
                target.machine == machine
                    && target.session == session
                    && target.admission.as_ref().map(|read| &read.context) == context.as_ref()
            });
        if same {
            return;
        }
        // The file edited in the session left is written first (Paseo
        // flushes an editor when its panel goes).
        self.flush_file();
        if !self.park_file() {
            return;
        }
        let acquisition = context.and_then(|context| {
            self.next_file_ticket()
                .map(|ticket| Acquisition { context, ticket })
        });
        let panel = &mut self.areas.workspace.code;
        panel.files.actions.invalidate();
        panel.target = Some(Target {
            admission: acquisition.clone(),
            machine: machine.clone(),
            session,
            place: Place::Reading,
            cwd: None,
        });
        panel.files.options = self.prefs.explorer;
        // A new session is read whole: nothing it shows is stale.
        panel.stale = Stale::default();
        panel.changes.summary = Summary::Loading;
        panel.changes.states.clear();
        panel.changes.closed.clear();
        panel.changes.relist();
        panel.files.listings.clear();
        panel.files.listing_tickets.clear();
        panel.files.open.clear();
        panel.files.viewer = None;
        panel.files.relist();
        if let Some(read) = acquisition {
            self.command(vessel_app_link::Command::FileOperation(Request::root(
                read.context,
                session,
                read.ticket,
            )));
        } else if let Some(target) = &mut self.areas.workspace.code.target {
            target.place = Place::Failed("the original connection is unavailable".to_owned());
        }
    }

    /// Where a session runs arrived.
    pub(super) fn install_code_session_read(
        &mut self,
        session: SessionId,
        result: Result<Box<SessionInfo>, Problem>,
    ) {
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
        self.restore_file_tree();
        self.code_read_tab();
    }
}
