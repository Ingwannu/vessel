//! The binary alone forwards the private result to its inherited stdout.

impl super::Node {
    /// Consume this launch's one private-pipe challenge result.
    ///
    /// No pipe name, credential or transport is exposed. The original server
    /// handle remains in the tracked task, awaiting authentication on that same
    /// handle. `None` means this node has no private desktop bootstrap.
    ///
    /// # Errors
    /// Creation, transcript, deadline, teardown or tracked-task failure.
    pub async fn desktop_bootstrap_ready(
        &mut self,
    ) -> Result<Option<vessel_proto::local_launch::BootstrapReady>, crate::DesktopBootstrapError>
    {
        let Some(proof) = self.desktop_ready.take() else {
            return Ok(None);
        };
        proof.await?.map(Some)
    }
}
