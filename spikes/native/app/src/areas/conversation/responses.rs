//! Correlation uses the same first waiting requests as the cards.
use super::ConversationArea;
use vessel_app_view::{responses::Request, timeline::EntryKind};

impl ConversationArea {
    /// Cached pending forms stay retained locally but are never actionable
    /// without a current connection to their selected node-book entry.
    pub(crate) fn response_online(&self, fleet: &vessel_app_view::Fleet) -> bool {
        self.lost.is_none()
            && self.shown.as_ref().is_some_and(|shown| {
                fleet.machines.iter().any(|machine| {
                    machine.name == shown.machine && machine.reach == vessel_app_view::Reach::Online
                })
            })
    }

    /// A resolved request or a same-session successor never receives a late reply.
    pub(crate) fn response_waiting(&self, request: Request) -> bool {
        match request {
            Request::Permission(id) => {
                self.timeline
                    .pending()
                    .map(|(_, permission)| permission.request.id)
                    .or_else(|| {
                        self.timeline
                            .unseen_permissions(&self.live)
                            .first()
                            .map(|request| request.id)
                    })
                    == Some(id)
            }
            Request::Question(id) => {
                self.timeline
                    .waiting_questions()
                    .first()
                    .and_then(|entry| {
                        if let EntryKind::Question { question, .. } = &entry.kind {
                            Some(question.id)
                        } else {
                            None
                        }
                    })
                    .or_else(|| {
                        self.timeline
                            .unseen_questions(&self.live)
                            .first()
                            .map(|question| question.id)
                    })
                    == Some(id)
            }
        }
    }
}
