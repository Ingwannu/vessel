//! Snapshot omission is meaningful only behind its exclusive event watermark.
//! This is a display reconciliation rule, never a synthetic resolution event or
//! authorization to send an answer. Session/follow admission remains the caller's.
use super::Request;
use crate::timeline::Live;
use vessel_proto::EventSeq;

/// Retain a record-backed request unless an authoritative snapshot covers that
/// record and omits its exact identity. An old peer without a watermark cannot
/// cancel known pending records by omission. Future delivery state is separate.
#[must_use]
pub fn retain_request(
    request: Request,
    asked: EventSeq,
    through: Option<EventSeq>,
    live: &Live,
) -> bool {
    if through.is_none_or(|boundary| asked >= boundary) {
        return true;
    }
    match request {
        Request::Permission(id) => live.permissions.iter().any(|request| request.id == id),
        Request::Question(id) => live.questions.iter().any(|question| question.id == id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::{PermissionId, PermissionRequest, QuestionId, ToolCallId};

    #[test]
    fn an_older_or_legacy_omission_cannot_withdraw_a_later_request() {
        let empty = Live::default();
        for request in [
            Request::Permission(PermissionId::random()),
            Request::Question(QuestionId::random()),
        ] {
            assert!(retain_request(request, EventSeq::new(9), None, &empty));
            assert!(retain_request(
                request,
                EventSeq::new(9),
                Some(EventSeq::new(5)),
                &empty
            ));
            assert!(retain_request(
                request,
                EventSeq::new(9),
                Some(EventSeq::new(9)),
                &empty
            ));
            assert!(!retain_request(
                request,
                EventSeq::new(9),
                Some(EventSeq::new(10)),
                &empty
            ));
        }
    }

    #[test]
    fn a_covering_snapshot_keeps_only_the_exact_request_it_still_contains() {
        let current = PermissionId::random();
        let successor = PermissionId::random();
        let live = Live {
            permissions: vec![PermissionRequest {
                id: current,
                tool_call: ToolCallId::new("tool"),
                options: Vec::new(),
            }],
            questions: Vec::new(),
        };
        let through = Some(EventSeq::new(10));
        assert!(retain_request(
            Request::Permission(current),
            EventSeq::new(9),
            through,
            &live
        ));
        assert!(!retain_request(
            Request::Permission(successor),
            EventSeq::new(9),
            through,
            &live
        ));
        assert!(!retain_request(
            Request::Question(QuestionId::random()),
            EventSeq::new(9),
            through,
            &live
        ));
        // A later log request is outside this read even when the snapshot only
        // contains the previous one. No event or card state is changed here.
        assert!(retain_request(
            Request::Permission(successor),
            EventSeq::new(11),
            through,
            &live
        ));
    }
}
