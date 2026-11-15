//! Reserved local-name syntax shared by book mutation and native admission.
//! A name is never controller provenance, connection or process authority.

use crate::NodeId;

const PREFIX: &str = "desktop-";
const UUID_DISPLAY_BYTES: usize = 36;

/// Exactly the deterministic local namespace, not an ordinary book alias.
///
/// This seals syntax only. A legacy book entry can have this spelling and
/// still remains a book entry. Callers require explicit provenance and apply
/// their ordinary name validation separately; `None` is not a valid-name proof.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ManagedLocalAlias(String);

impl ManagedLocalAlias {
    /// Generate the stable spelling for the controller's typed node identity.
    /// This does not authenticate that identity or authorize an admission.
    #[must_use]
    pub fn for_node(node: NodeId) -> Self {
        Self(format!("{PREFIX}{node}"))
    }

    /// Recognize only prefix plus canonical lowercase, hyphenated UUID.
    /// No trim, case-fold, URI, compact or braced UUID normalization occurs.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let value = text.strip_prefix(PREFIX)?;
        if value.len() != UUID_DISPLAY_BYTES {
            return None;
        }
        let canonical = Self::for_node(value.parse::<NodeId>().ok()?);
        (canonical.as_str() == text).then_some(canonical)
    }

    /// Borrow the canonical spelling for the owning layer's name constructor.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests;
