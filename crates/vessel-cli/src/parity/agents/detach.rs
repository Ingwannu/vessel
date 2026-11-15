//! Detach consumes W2's landed atomic relationship call, never rewriting Creator.
use crate::{
    args::{Target, parity::Presentation},
    connect::connect,
    error::CliError,
    parity::output::{self, Row},
    selector::SessionSelector,
};
use serde::Serialize;
use vessel_proto::{
    SessionId, SessionInfo,
    subagents::{DetachDisposition, DetachSession, DetachedSession, Parentage},
};

pub(super) async fn run(
    target: &Target,
    presentation: Presentation,
    id: SessionSelector,
) -> Result<(), CliError> {
    let client = connect(target).await?.client;
    let before = id.resolve(&client).await?;
    let applied = client.call(DetachSession { session: before.id }).await?;
    let receipt = Receipt::checked(&before, &applied)?;
    output::record(presentation, &receipt).await
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum Status {
    Detached,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Receipt {
    agent_id: SessionId,
    status: Status,
}
impl Receipt {
    fn checked(before: &SessionInfo, applied: &DetachedSession) -> Result<Self, CliError> {
        if applied.session.id != before.id
            || applied.session.created_by != before.created_by
            || !matches!(applied.session.parentage(), Ok(Parentage::Detached(receipt)) if receipt == applied.receipt)
            || before
                .detach_receipt
                .is_some_and(|receipt| receipt != applied.receipt)
        {
            return Err(CliError::ParityInvalidReceipt(
                "detach target/provenance/receipt differ",
            ));
        }
        // Node-side marking admits this atomically with authoritative placement.
        // A client snapshot cannot fence a concurrent move by comparing cwd here.
        let status = match applied.disposition {
            DetachDisposition::Detached | DetachDisposition::AlreadyDetached => Status::Detached,
        };
        Ok(Self {
            agent_id: before.id,
            status,
        })
    }
}
impl Row for Receipt {
    fn id(&self) -> String {
        self.agent_id.to_string()
    }
    fn headers() -> &'static [&'static str] {
        &["AGENT ID", "STATUS"]
    }
    fn cells(&self) -> Vec<String> {
        let status = match self.status {
            Status::Detached => "detached",
        };
        vec![self.id(), status.to_owned()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::DateTime;
    use vessel_proto::{
        Creator, HarnessKind, NodeId, SessionMode, SessionRef, SessionSpec,
        subagents::DetachReceipt,
    };
    fn fixture() -> (SessionInfo, DetachedSession) {
        let parent = SessionRef {
            node: NodeId::random(),
            session: SessionId::random(),
        };
        let mut before = SessionInfo::new(
            SessionId::random(),
            SessionSpec::new(
                HarnessKind::ClaudeCode,
                SessionMode::Structured {
                    model: None,
                    resume: None,
                    options: Vec::new(),
                },
            ),
            DateTime::UNIX_EPOCH,
        );
        before.created_by = Creator::Agent {
            node: parent.node,
            session: parent.session,
        };
        let receipt = DetachReceipt::new(parent, DateTime::UNIX_EPOCH);
        let mut after = before.clone();
        after.detach_receipt = Some(receipt);
        (
            before,
            DetachedSession {
                session: Box::new(after),
                receipt,
                disposition: DetachDisposition::Detached,
            },
        )
    }
    #[test]
    fn detach_emits_the_reference_receipt_only_after_checked_atomic_acknowledgement() {
        let (before, mut applied) = fixture();
        let receipt = Receipt::checked(&before, &applied).unwrap();
        assert_eq!(
            serde_json::to_string(&receipt).unwrap(),
            format!("{{\"agentId\":\"{}\",\"status\":\"detached\"}}", before.id)
        );
        applied.disposition = DetachDisposition::AlreadyDetached;
        let mut prior = before;
        prior.detach_receipt = Some(applied.receipt);
        assert!(Receipt::checked(&prior, &applied).is_ok());
        applied.receipt = DetachReceipt::new(
            applied.receipt.former_parent(),
            DateTime::UNIX_EPOCH
                .checked_add_signed(chrono::Duration::seconds(1))
                .unwrap(),
        );
        applied.session.detach_receipt = Some(applied.receipt);
        assert!(matches!(
            Receipt::checked(&prior, &applied),
            Err(CliError::ParityInvalidReceipt(_))
        ));
    }
    #[test]
    fn wrong_id_missing_detach_or_rewritten_creator_never_become_success() {
        let (before, mut applied) = fixture();
        applied.session.id = SessionId::random();
        assert!(Receipt::checked(&before, &applied).is_err());
        applied.session.id = before.id;
        applied.session.detach_receipt = None;
        assert!(Receipt::checked(&before, &applied).is_err());
        applied.session.detach_receipt = Some(applied.receipt);
        applied.session.created_by = Creator::Owner;
        assert!(Receipt::checked(&before, &applied).is_err());
    }
}
