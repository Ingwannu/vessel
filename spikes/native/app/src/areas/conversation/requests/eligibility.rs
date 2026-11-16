//! Read existing pending indices and the actual retained form; never infer origin.
use super::{ConversationArea, Form};
use vessel_app_view::residents::ResidentProblem;
use vessel_app_view::timeline::EntryKind;

pub(super) fn check(area: &ConversationArea, form: &Form) -> Result<(), ResidentProblem> {
    match form {
        Form::Permission { id, option } => {
            if area.permit_sending.is_some() {
                return Err(ResidentProblem::Held);
            }
            let request = permission(area, *id);
            let request = request.ok_or(ResidentProblem::Missing)?;
            if !request.options.iter().any(|offered| offered.id == *option) {
                return Err(ResidentProblem::UnadmittedOperation);
            }
        }
        Form::Question { id, source } => {
            let waiting = question_waiting(area, *id);
            if !waiting {
                return Err(ResidentProblem::Missing);
            }
            let (actual, sent) = area
                .answering
                .as_ref()
                .and_then(|answering| answering.request_source(*id))
                .ok_or(ResidentProblem::Missing)?;
            if actual != *source {
                return Err(ResidentProblem::StaleEpoch);
            }
            if sent {
                return Err(ResidentProblem::Held);
            }
        }
    }
    Ok(())
}

pub(super) fn permission(
    area: &ConversationArea,
    id: vessel_proto::PermissionId,
) -> Option<&vessel_proto::PermissionRequest> {
    area.timeline
        .pending_permissions()
        .find(|(_, permission)| permission.request.id == id)
        .map(|(_, permission)| &permission.request)
        .or_else(|| {
            area.timeline
                .unseen_permissions(&area.live)
                .into_iter()
                .find(|request| request.id == id)
        })
}

pub(super) fn question_waiting(area: &ConversationArea, id: vessel_proto::QuestionId) -> bool {
    area.timeline.waiting_questions().iter().any(
        |entry| matches!(&entry.kind, EntryKind::Question { question, .. } if question.id == id),
    ) || area
        .timeline
        .unseen_questions(&area.live)
        .iter()
        .any(|question| question.id == id)
}
