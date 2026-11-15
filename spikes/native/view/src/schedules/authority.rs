//! Authority evidence for one schedule row on one connection. An absent scope
//! report is not a denial. Keep a refusal on its row, not on unrelated rows;
//! replace this evidence when the connection changes.

use crate::Lang;
use vessel_proto::{ErrorCode, Scope};

/// Row/connection-local evidence, using the wire's actual scope type.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum Authority {
    /// The connection does not report its scopes. Offer actions and let the
    /// node decide, as the rest of the app does.
    #[default]
    Unknown,
    /// Scopes actually reported for this device, never inferred from defaults.
    Scopes(Vec<Scope>),
    /// The node answered Forbidden to an operation on this connection. This
    /// records the refusal without inventing a list of the device's scopes.
    OperationForbidden,
}

impl Authority {
    /// Known insufficient or actually refused means no mutation actions.
    #[must_use]
    pub fn allows_actions(&self) -> bool {
        match self {
            Self::Unknown => true,
            Self::Scopes(scopes) => scopes.contains(&Scope::Operate),
            Self::OperationForbidden => false,
        }
    }

    /// A node refusal is evidence for this connection only. Starting a new
    /// connection constructs a new Authority; other failures never narrow it.
    pub fn refused(&mut self, code: ErrorCode) {
        match code {
            ErrorCode::Forbidden => *self = Self::OperationForbidden,
            ErrorCode::InvalidRequest
            | ErrorCode::Unauthorized
            | ErrorCode::ProtocolMismatch
            | ErrorCode::NotFound
            | ErrorCode::Conflict
            | ErrorCode::Unsupported
            | ErrorCode::HarnessNotInstalled
            | ErrorCode::SpawnFailed
            | ErrorCode::Unavailable
            | ErrorCode::Internal => {}
        }
    }

    /// Explain a known limitation where this row's actions would appear.
    #[must_use]
    pub fn explanation(&self, lang: Lang) -> Option<&'static str> {
        (!self.allows_actions()).then_some(lang.schedules().read_only)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schedules::{Action, actions};

    #[test]
    fn unknown_is_not_read_only_and_a_real_forbidden_refusal_is_remembered() {
        let schedule = super::super::tests::info();
        let mut authority = Authority::Unknown;
        assert!(actions(&schedule, &authority).contains(&Action::RunNow));
        assert_eq!(authority.explanation(Lang::En), None);
        authority.refused(ErrorCode::Forbidden);
        assert!(actions(&schedule, &authority).is_empty());
        assert_eq!(
            authority.explanation(Lang::En),
            Some("This device can only view this machine's schedules.")
        );
        assert_eq!(
            authority.explanation(Lang::Ko),
            Some("이 머신의 예약은 이 기기에서 조회만 할 수 있습니다.")
        );
        assert!(actions(&schedule, &Authority::default()).contains(&Action::RunNow));
        // A refusal for this row cannot suppress another row's unknown
        // authority, nor survive constructing a new connection's evidence.
        assert!(Authority::Unknown.allows_actions());
    }

    #[test]
    fn known_scopes_are_used_without_assuming_admin_implies_operate() {
        let schedule = super::super::tests::info();
        for scopes in [vec![], vec![Scope::Read], vec![Scope::Admin]] {
            let authority = Authority::Scopes(scopes);
            assert!(actions(&schedule, &authority).is_empty());
            assert!(authority.explanation(Lang::En).is_some());
            assert!(authority.explanation(Lang::Ko).is_some());
        }
        let authority = Authority::Scopes(vec![Scope::Read, Scope::Operate]);
        assert!(actions(&schedule, &authority).contains(&Action::RunNow));
        assert_eq!(authority.explanation(Lang::Ko), None);
    }

    #[test]
    fn a_connection_or_operation_error_is_not_authority_evidence() {
        for code in [
            ErrorCode::InvalidRequest,
            ErrorCode::Unauthorized,
            ErrorCode::ProtocolMismatch,
            ErrorCode::NotFound,
            ErrorCode::Conflict,
            ErrorCode::Unsupported,
            ErrorCode::HarnessNotInstalled,
            ErrorCode::SpawnFailed,
            ErrorCode::Unavailable,
            ErrorCode::Internal,
        ] {
            let mut authority = Authority::Unknown;
            authority.refused(code);
            assert_eq!(authority, Authority::Unknown);
        }
    }
}
