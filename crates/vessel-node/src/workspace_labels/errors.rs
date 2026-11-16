//! Exercise the production error boundary without registering a pending RPC.

#[cfg(test)]
mod tests {
    use crate::error::ServiceError;
    use vessel_proto::{
        ErrorCode, ErrorDetail,
        workspace_labels::{LabelColor, WorkspaceLabelFailure},
    };
    use vessel_store::StoreError;

    #[test]
    fn label_refusals_keep_structured_reason_and_broad_code() {
        let error = ServiceError::from(StoreError::WorkspaceLabelRefused(
            WorkspaceLabelFailure::NameTaken,
        ))
        .to_wire();
        assert_eq!(error.code, ErrorCode::Conflict);
        assert_eq!(
            error.detail,
            Some(ErrorDetail::WorkspaceLabel {
                reason: WorkspaceLabelFailure::NameTaken
            })
        );
    }

    #[test]
    fn corrupt_row_color_diagnostics_cannot_echo_stored_data_to_a_client() {
        let canary = "fake-private-row-content";
        let source = serde_json::from_str::<LabelColor>(&format!("\"{canary}\"")).unwrap_err();
        let service = ServiceError::from(StoreError::WorkspaceLabelColor(source));
        assert!(std::error::Error::source(&service).is_some());
        let wire = service.to_wire();
        assert_eq!(wire.code, ErrorCode::Internal);
        assert!(!wire.message.contains(canary));
    }
}
