//! Correlation uses the same first waiting requests as the cards.
use super::ConversationArea;
use vessel_app_view::{responses::Request, timeline::EntryKind};

impl ConversationArea {
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
                    .and_then(|entry| match &entry.kind {
                        EntryKind::Question { question, .. } => Some(question.id),
                        _ => None,
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
