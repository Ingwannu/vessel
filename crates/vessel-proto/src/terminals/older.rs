//! Added activity is absent for every peer until the integrator assigns it.
use super::TerminalInfo;
use crate::ProtocolVersion;
impl TerminalInfo {
    /// This snapshot without attention a peer cannot understand. Used by
    /// direct, list, script and nested fleet reply projections alike.
    #[must_use]
    pub fn for_protocol(&self, protocol: ProtocolVersion) -> Option<Self> {
        if self.attention.is_none()
            || crate::notifications::ATTENTION_NOTIFICATIONS_PROTOCOL
                .is_some_and(|since| protocol >= since)
        {
            return None;
        }
        let mut older = self.clone();
        older.attention = None;
        Some(older)
    }
}
