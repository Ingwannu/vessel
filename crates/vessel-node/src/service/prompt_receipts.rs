//! Operate and exact target ownership are enforced by the common guard.
use super::{Connection, NodeService, wire};
use crate::{profiles::launch::creator, transfers::Holder};
use vessel_proto::mcp::{
    ClosePromptReceipt, PromptReceiptState, ReadPromptReceipt, TrackPrompt, TrackedPrompt,
};
use vessel_proto::{Ack, WireError};

impl NodeService {
    pub(crate) fn check_prompt_tracking(
        &self,
        _: &Connection,
        _: vessel_proto::mcp::CheckPromptTracking,
    ) -> std::future::Ready<Result<Ack, WireError>> {
        std::future::ready(Ok(Ack))
    }
    pub(crate) async fn track_prompt(
        &self,
        connection: &Connection,
        request: TrackPrompt,
    ) -> Result<TrackedPrompt, WireError> {
        let reservation = self
            .prompt_receipts
            .reserve(Holder::of(connection.principal()), request.session)?;
        let (delivered, turn) = self
            .sessions
            .prompt_tracked(
                request.session,
                request.content,
                request.delivery,
                creator(connection.principal()),
            )
            .await
            .map_err(|error| wire(&error))?;
        let receipt = reservation.id;
        reservation.follow(turn, &self.forwarders, self.lifecycle.stopping());
        Ok(TrackedPrompt { delivered, receipt })
    }
    pub(crate) async fn read_prompt_receipt(
        &self,
        connection: &Connection,
        request: ReadPromptReceipt,
    ) -> Result<PromptReceiptState, WireError> {
        self.prompt_receipts
            .read(
                Holder::of(connection.principal()),
                request.session,
                request.receipt,
            )
            .await
    }
    pub(crate) fn close_prompt_receipt(
        &self,
        connection: &Connection,
        request: ClosePromptReceipt,
    ) -> std::future::Ready<Result<Ack, WireError>> {
        std::future::ready(
            self.prompt_receipts
                .close(
                    Holder::of(connection.principal()),
                    request.session,
                    request.receipt,
                )
                .map(|()| Ack),
        )
    }
}
