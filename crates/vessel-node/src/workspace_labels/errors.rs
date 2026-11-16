//! Exercise the production error boundary without registering a pending RPC.

#[cfg(test)]
mod tests {
    use crate::error::ServiceError;
    use vessel_proto::{
        ErrorCode, ErrorDetail,
        workspace_labels::WorkspaceLabelFailure,
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
        let row = vessel_store::LabelRow::Catalog(std::num::NonZeroUsize::MIN);
        let service = ServiceError::from(StoreError::WorkspaceLabelColor { row });
        assert!(std::error::Error::source(&service).is_some());
        let wire = service.to_wire();
        assert_eq!(wire.code, ErrorCode::Internal);
        // A client sees the fixed notice only, not even the row's place.
        assert_eq!(wire.message, "stored workspace labels could not be read");
    }
}
