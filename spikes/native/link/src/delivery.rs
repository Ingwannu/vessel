//! Queued answers carry weak validity, never ownership of an old transport.
use vessel_app_view::connection::{ConnectionContext, Refusal};

/// Opaque original-lease validity for Unary and stream result envelopes.
///
/// Construction stays in the link actor. Cloning retains neither a client nor
/// its transport. A receiver checks immediately before installation, with no
/// intervening await, in addition to its own binding and operation ticket.
#[derive(Clone)]
pub struct DeliveryGuard {
    #[cfg(not(target_arch = "wasm32"))]
    source: std::sync::Weak<crate::connections::SelectedLease>,
    #[cfg(target_arch = "wasm32")]
    never: std::convert::Infallible,
}
impl DeliveryGuard {
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn from_lease(source: &std::sync::Arc<crate::connections::SelectedLease>) -> Self {
        Self {
            source: std::sync::Arc::downgrade(source),
        }
    }
    /// Validate the exact original admission immediately before local mutation.
    ///
    /// # Errors
    /// Refuses expired owner, changed identity, retired watch or closed transport.
    pub fn validate(&self, expected: &ConnectionContext) -> Result<(), Refusal> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let source = self.source.upgrade().ok_or(Refusal::Changed)?;
            source.validate(expected)
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _expected = expected;
            match self.never {}
        }
    }
}
impl std::fmt::Debug for DeliveryGuard {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("DeliveryGuard(<opaque>)")
    }
}
