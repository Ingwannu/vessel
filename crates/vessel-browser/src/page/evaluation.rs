//! Function execution uses the ordinary page and invalidates prior element refs.
use super::{Page, PageError, requests};
use crate::evaluation;
use vessel_proto::browser::{CappedText, RequiredInput};

impl Page {
    /// No optional element is accepted until the checked element resolver exists.
    /// Node principal, tool policy, setting and resource admission remain above this path.
    pub(crate) async fn evaluate_function(
        &mut self,
        function: &RequiredInput,
    ) -> Result<CappedText, PageError> {
        self.active()?;
        let expression = evaluation::expression(function)?;
        let frame = self.execute(requests::FrameTree {}).await?.frame_tree.frame;
        // Arbitrary user code may change the DOM even when it throws or its caller
        // stops waiting. Old action refs therefore stop resolving before execution.
        let _revision = self.invalidate()?;
        let observed = self.execute(requests::Evaluate::new(expression)).await?;
        let after = self.execute(requests::FrameTree {}).await?.frame_tree.frame;
        if frame.id != after.id || frame.loader_id != after.loader_id {
            let _revision = self.invalidate()?;
            return Err(PageError::Navigation);
        }
        evaluation::finish(&observed).map_err(PageError::Function)
    }
}
