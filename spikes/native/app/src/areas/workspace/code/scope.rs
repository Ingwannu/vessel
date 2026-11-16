//! Root receipts and owner callbacks keep the original shared admission.
use super::Place;
use crate::shell::Shell;
use vessel_app_kit::ui::Context;
use vessel_app_link::workspace::scoped::{Outcome, Reply};
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
        let guarded = reply.is_current();
        let (context, outcome) = reply.into_parts();
        let current =
            guarded && self.file_connection(context.machine().as_str()).as_ref() == Some(&context);
        match outcome {
            Outcome::Root {
                session,
                ticket,
                result,
            } => {
                self.finder_scoped_root(&context, session, ticket, &result, current);
            }
            Outcome::File(reply) => {
                let (scope, answer, dispatched) = (*reply).into_parts();
                self.file_scoped_answer(&scope, answer, current, dispatched, cx);
            }
        }
    }
    /// Legacy session.get answers do not supply a checked connection receipt.
    pub(in crate::areas::workspace) fn code_session_read(
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
        let admission = target.admission.as_ref()?;
        if !admission.current() {
            return None;
        }
        let context = &admission.context;
        if self.file_connection(context.machine().as_str()).as_ref() != Some(context) {
            return None;
        }
        let (_, root) = panel.root()?;
        Some(FileScope::new(context.clone(), *root))
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
