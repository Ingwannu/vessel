//! Installation uses original requests and validates the SAME actor-minted guard.
use super::{
    Place, Shell,
    shown_source::{ShownChangesContext, ShownDiffContext},
};
use vessel_app_link::{
    DeliveryGuard,
    workspace::code_source::{Answer, Body, Request},
};
use vessel_app_view::workspace::{problem::Problem, root::session_root};
use vessel_proto::{
    SessionInfo,
    workspace::{DiffSummary, FileDiff},
};
impl Shell {
    pub(crate) fn code_source_answer(&mut self, answer: Answer) {
        let (request, guard, body) = answer.into_parts();
        let live = guard
            .as_ref()
            .is_some_and(|guard| guard.validate(request.context()).is_ok());
        match body {
            Body::Root(result) => self.source_root_answer(&request, guard, &checked(result, live)),
            Body::Summary(result) => {
                self.source_summary_answer(request, guard, checked(result, live));
            }
            Body::Diff(result) => self.source_diff_answer(&request, guard, checked(result, live)),
        }
    }
    fn source_root_answer(
        &mut self,
        request: &Request,
        guard: Option<DeliveryGuard>,
        result: &Result<Box<SessionInfo>, Problem>,
    ) {
        let Request::Root {
            context,
            session,
            ticket,
        } = request
        else {
            return;
        };
        let words = self.words();
        let panel = &mut self.areas.workspace.code;
        let Some(target) = panel.target.as_mut() else {
            return;
        };
        let Some(origin) = target.receipt.as_mut().filter(|origin| {
            origin.context == *context && origin.session == *session && origin.ticket == *ticket
        }) else {
            return;
        };
        origin.guard = guard.filter(|guard| guard.validate(context).is_ok());
        target.place = match result {
            Ok(info) => session_root(info).map_or(Place::Outside, Place::Root),
            Err(problem) => Place::Failed(words.problem(problem)),
        };
        target.cwd = result.as_ref().ok().and_then(|info| info.spec.cwd.clone());
        self.finder_session_read(*session, result);
        self.code_read_tab();
    }
    fn source_summary_answer(
        &mut self,
        request: Request,
        guard: Option<DeliveryGuard>,
        result: Result<DiffSummary, Problem>,
    ) {
        let panel = &mut self.areas.workspace.code;
        if panel.changes.sources.requested.as_ref() != Some(&request) {
            return;
        }
        panel.changes.sources.ready = None;
        panel.changes.sources.diffs.clear();
        let Request::Summary {
            scope,
            root_ticket,
            ticket,
            ask,
        } = request
        else {
            return;
        };
        let Some(origin) = panel
            .target
            .as_ref()
            .and_then(|target| target.receipt.as_ref())
            .filter(|origin| origin.ticket == root_ticket && origin.context == *scope.connection())
            .cloned()
        else {
            return;
        };
        let comparison = panel.changes.scope;
        let result = checked(result, origin.current());
        self.install_changes_read(&ask, result);
        let panel = &mut self.areas.workspace.code;
        if let Some(guard) = guard.filter(|guard| {
            guard.validate(scope.connection()).is_ok()
                && matches!(panel.changes.summary, super::Summary::Ready(_))
        }) {
            panel.changes.sources.ready = Some(ShownChangesContext {
                root: origin,
                scope,
                ticket,
                comparison,
                guard,
            });
        }
    }
    fn source_diff_answer(
        &mut self,
        request: &Request,
        guard: Option<DeliveryGuard>,
        result: Result<FileDiff, Problem>,
    ) {
        let Request::Diff {
            ask,
            root_ticket,
            summary,
            ticket,
            scope,
        } = request
        else {
            return;
        };
        let panel = &mut self.areas.workspace.code;
        let Some(parent) = panel
            .changes
            .sources
            .ready
            .as_ref()
            .filter(|parent| {
                parent.ticket == *summary
                    && parent.root.ticket == *root_ticket
                    && parent.scope == *scope
            })
            .cloned()
        else {
            return;
        };
        let Some(source) = panel
            .changes
            .sources
            .diffs
            .get_mut(&ask.path)
            .filter(|source| &source.request == request)
        else {
            return;
        };
        source.ready = None;
        let result = checked(
            result,
            parent.guard.validate(scope.connection()).is_ok() && parent.root.current(),
        );
        self.install_diff_read(ask, result);
        let panel = &mut self.areas.workspace.code;
        if let Some(guard) = guard.filter(|guard| {
            guard.validate(scope.connection()).is_ok()
                && matches!(
                    panel.changes.states.get(&ask.path),
                    Some(vessel_app_view::workspace::changes::FileState::Read(_))
                )
        }) && let Some(source) = panel.changes.sources.diffs.get_mut(&ask.path)
        {
            source.ready = Some(ShownDiffContext {
                summary: parent,
                path: ask.path.clone(),
                ticket: *ticket,
                guard,
            });
        }
    }
}
fn checked<T>(result: Result<T, Problem>, live: bool) -> Result<T, Problem> {
    match result {
        Ok(value) if live => Ok(value),
        Ok(_) => Err(Problem::Gone(String::new())),
        Err(problem) => Err(problem),
    }
}
