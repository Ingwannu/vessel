//! Existing session answers are checked against the exact freshly fetched
//! request. The node still owns authorization and first-answer arbitration.
use vessel_app_view::inbox::{Content, Entry, Problem};
use vessel_proto::{ErrorCode, PermissionId, PermissionOptionId, QuestionAnswer, QuestionId};

/// An answer uses the session protocol's existing validation boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Input {
    /// An agent-provided permission choice.
    Permission(PermissionId, PermissionOptionId),
    /// A checked question form, or the protocol's explicit decline.
    Question(QuestionId, QuestionAnswer),
}
impl Input {
    /// Reject a stale request id, invented permission choice or invalid form
    /// before handing it to the existing session send path.
    /// # Errors
    /// `Conflict` for a different/resolved request, `InvalidRequest` for a choice
    /// or form not admitted by the actor's current request.
    pub fn validate(&self, entry: &Entry) -> Result<(), Problem> {
        match self {
            Self::Permission(id, option) => {
                let Content::Permission { request, .. } = &entry.content else {
                    return Err(Problem::Refused(ErrorCode::Conflict));
                };
                if request.id != *id {
                    return Err(Problem::Refused(ErrorCode::Conflict));
                }
                if !request.options.iter().any(|choice| choice.id == *option) {
                    return Err(Problem::Refused(ErrorCode::InvalidRequest));
                }
            }
            Self::Question(id, answer) => {
                let Content::Question(question) = &entry.content else {
                    return Err(Problem::Refused(ErrorCode::Conflict));
                };
                if question.id != *id {
                    return Err(Problem::Refused(ErrorCode::Conflict));
                }
                question
                    .check(answer)
                    .map_err(|_invalid| Problem::Refused(ErrorCode::InvalidRequest))?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_app_view::inbox::{Key, Position, Read};
    use vessel_proto::{
        EventSeq, NodeId, PermissionOption, PermissionOptionKind, PermissionRequest, Question,
        SessionId,
    };
    fn entry(content: Content) -> Entry {
        Entry {
            position: Position::new(1),
            key: Key {
                node: NodeId::random(),
                session: SessionId::random(),
                seq: EventSeq::new(1),
            },
            title: None,
            at: chrono::DateTime::UNIX_EPOCH,
            read: Read::Unread,
            content,
        }
    }
    #[test]
    fn permission_options_must_belong_to_the_same_current_request() {
        let id = PermissionId::random();
        let option: PermissionOptionId = PermissionOptionId::new("allow");
        let current = entry(Content::Permission {
            request: PermissionRequest {
                id,
                tool_call: vessel_proto::ToolCallId::new("tool"),
                options: vec![PermissionOption {
                    id: option.clone(),
                    name: "Allow".to_owned(),
                    kind: PermissionOptionKind::AllowOnce,
                }],
            },
            tool: None,
        });
        assert_eq!(
            Input::Permission(id, option.clone()).validate(&current),
            Ok(())
        );
        assert_eq!(
            Input::Permission(PermissionId::random(), option).validate(&current),
            Err(Problem::Refused(ErrorCode::Conflict))
        );
        assert_eq!(
            Input::Permission(id, PermissionOptionId::new("invented")).validate(&current),
            Err(Problem::Refused(ErrorCode::InvalidRequest))
        );
    }
    #[test]
    fn a_question_decline_cannot_answer_a_different_request_or_an_outcome() {
        let id = QuestionId::random();
        let current = entry(Content::Question(Question {
            id,
            message: "Continue?".to_owned(),
            tool_call: None,
            fields: Vec::new(),
        }));
        assert_eq!(
            Input::Question(id, QuestionAnswer::Decline).validate(&current),
            Ok(())
        );
        assert_eq!(
            Input::Question(QuestionId::random(), QuestionAnswer::Decline).validate(&current),
            Err(Problem::Refused(ErrorCode::Conflict))
        );
        assert_eq!(
            Input::Question(id, QuestionAnswer::Decline).validate(&entry(Content::Finished)),
            Err(Problem::Refused(ErrorCode::Conflict))
        );
    }
}
