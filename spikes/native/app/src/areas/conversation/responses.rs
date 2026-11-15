//! Correlation uses the same exact pending identities as simultaneous cards.
use super::ConversationArea;
use vessel_app_link::responses::Response;
use vessel_app_view::{responses::Request, timeline::EntryKind};

impl ConversationArea {
    /// Cached pending forms stay retained locally but are never actionable
    /// without a current connection to their selected node-book entry.
    pub(crate) fn response_online(&self, fleet: &vessel_app_view::Fleet) -> bool {
        self.lost.is_none()
            && self.shown.as_ref().is_some_and(|shown| {
                fleet.machines.iter().any(|machine| {
                    machine.name == shown.machine
                        && machine.reach == vessel_app_view::Reach::Online
                        && machine.connection_context().is_some()
                })
            })
    }

    /// A resolved request or a same-session successor never receives a late reply.
    pub(crate) fn response_waiting(&self, request: Request) -> bool {
        match request {
            Request::Permission(id) => {
                self.timeline.pending_permissions().any(|(_, permission)| permission.request.id == id)
                    || self.timeline.unseen_permissions(&self.live).into_iter().any(|request| request.id == id)
            }
            Request::Question(id) => {
                self.timeline.waiting_questions().into_iter().any(|entry| {
                    matches!(&entry.kind, EntryKind::Question { question, .. } if question.id == id)
                }) || self.timeline.unseen_questions(&self.live).into_iter().any(|question| question.id == id)
            }
        }
    }
    /// Validate against the same exact form that the card currently draws.
    /// A stale queued option/answer must fail before response admission.
    pub(crate) fn response_valid(&self, response: &Response) -> bool {
        match response {
            Response::Permission { id, option } => self
                .timeline
                .pending_permissions()
                .find(|(_, permission)| permission.request.id == *id)
                .map(|(_, permission)| &permission.request)
                .or_else(|| {
                    self.timeline
                        .unseen_permissions(&self.live)
                        .into_iter()
                        .find(|permission| permission.id == *id)
                })
                .is_some_and(|permission| {
                    permission
                        .options
                        .iter()
                        .any(|offered| offered.id == *option)
                }),
            Response::Question { id, answer } => self
                .timeline
                .waiting_questions()
                .into_iter()
                .find_map(|entry| {
                    if let EntryKind::Question { question, .. } = &entry.kind
                        && question.id == *id
                    {
                        Some(question)
                    } else {
                        None
                    }
                })
                .or_else(|| {
                    self.timeline
                        .unseen_questions(&self.live)
                        .into_iter()
                        .find(|question| question.id == *id)
                })
                .is_some_and(|question| question.check(answer).is_ok()),
        }
    }

    /// Prune only local per-request controls after authoritative changes.
    pub(super) fn reconcile_responses(&mut self) {
        if self.response_failures.is_empty()
            && self.permit_sending.is_empty()
            && self.plan_folded.is_empty()
        {
            return;
        }
        let mut pending: Vec<_> = self
            .timeline
            .pending_permissions()
            .map(|(_, permission)| Request::Permission(permission.request.id))
            .collect();
        pending.extend(
            self.timeline
                .unseen_permissions(&self.live)
                .into_iter()
                .map(|permission| Request::Permission(permission.id)),
        );
        pending.extend(
            self.timeline
                .waiting_questions()
                .into_iter()
                .filter_map(|entry| {
                    if let EntryKind::Question { question, .. } = &entry.kind {
                        Some(Request::Question(question.id))
                    } else {
                        None
                    }
                }),
        );
        pending.extend(
            self.timeline
                .unseen_questions(&self.live)
                .into_iter()
                .map(|question| Request::Question(question.id)),
        );
        self.response_failures.retain(&pending);
        self.permit_sending
            .retain(|id, _| pending.contains(&Request::Permission(*id)));
        self.plan_folded
            .retain(|id| pending.contains(&Request::Permission(*id)));
    }
}
